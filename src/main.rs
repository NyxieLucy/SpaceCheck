mod app;
mod scan;
mod ui;

use std::io;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{execute, ExecutableCommand};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use app::{App, Mode, ScanCommand, ScanMessage};

fn main() -> Result<()> {
    //selecting arguments to the terminal wlla the cmd 
    let start_path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().expect("cwd"));

    // --- terminal setup -----------------------------------------------
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut terminal, start_path);

    // --- terminal teardown ---------------------------------------------
    // restore the terminal eeven if it returned an error  s that a crush doesn't leave the user
    // fucked or stuck 
    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;

    result
}

fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, start_path: PathBuf) -> Result<()> {
    // Two channels: one to send scan requests to the worker thread, one
    // for it to send results back keeping the worker separate means a
    // slow directory (millions of files, a network share, whatever) never
    // freezes keyboard input or redraws.
    let (cmd_tx, cmd_rx) = mpsc::channel::<ScanCommand>();
    let (msg_tx, msg_rx) = mpsc::channel::<ScanMessage>();

    let worker = std::thread::spawn(move || scan::run(cmd_rx, msg_tx));

    let mut app = App::new(start_path.clone());
    scan::request_scan(&cmd_tx, start_path);

    loop {
        // drain any scan results that arrived since the last frame.
        while let Ok(msg) = msg_rx.try_recv() {
            match msg {
                ScanMessage::DirReady { path, entries } => app.set_entries(path, entries),
                ScanMessage::Error { message, .. } => {
                    app.loading = false;
                    app.mode = Mode::Message(format!("error: {message}"));
                }
            }
        }

        terminal.draw(|frame| ui::draw(frame, &app))?;

        // Poll with a timeout instead of blocking so we keep redrawing
        // (e.g. to clear the "scanning..." state) even with no keypress.
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                handle_key(&mut app, &cmd_tx, key.code);
            }
        }

        if app.should_quit {
            let _ = cmd_tx.send(ScanCommand::Quit);
            break;
        }
    }

    let _ = worker.join();
    Ok(())
}

fn handle_key(app: &mut App, cmd_tx: &mpsc::Sender<ScanCommand>, key: KeyCode) {
    // The confirm-delete modal intercepts all keys until answered.
    if let Mode::ConfirmDelete { index } = app.mode {
        match key {
            KeyCode::Char('y') | KeyCode::Char('Y') => delete_entry(app, cmd_tx, index),
            _ => app.mode = Mode::Browsing,
        }
        return;
    }

    // Any keypress clears a transient status message.
    if matches!(app.mode, Mode::Message(_)) {
        app.mode = Mode::Browsing;
    }

    match key {
        KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
        KeyCode::Down | KeyCode::Char('j') => app.move_selection(1),
        KeyCode::Up | KeyCode::Char('k') => app.move_selection(-1),
        KeyCode::Enter => enter_selected(app, cmd_tx),
        KeyCode::Backspace => go_up(app, cmd_tx),
        KeyCode::Char('d') => {
            if !app.entries.is_empty() {
                app.mode = Mode::ConfirmDelete { index: app.selected };
            }
        }
        _ => {}
    }
}

/// Descend into the currently selected directory (no-op on files).
fn enter_selected(app: &mut App, cmd_tx: &mpsc::Sender<ScanCommand>) {
    let Some(entry) = app.selected_entry() else {
        return;
    };
    if !entry.is_dir {
        return;
    }
    app.current_path = entry.path.clone();
    app.loading = true;
    scan::request_scan(cmd_tx, app.current_path.clone());
}

/// Go up to the parent directory, if there is one.
fn go_up(app: &mut App, cmd_tx: &mpsc::Sender<ScanCommand>) {
    let Some(parent) = app.current_path.parent() else {
        return;
    };
    app.current_path = parent.to_path_buf();
    app.loading = true;
    scan::request_scan(cmd_tx, app.current_path.clone());
}

/// Move the entry at `index` to the Recycle Bin (via the `trash` crate)
/// rather than permanently deleting it — a mistaken keypress here
/// shouldn't be unrecoverable. Re-scans the current directory afterward
/// so the listing reflects the deletion.
fn delete_entry(app: &mut App, cmd_tx: &mpsc::Sender<ScanCommand>, index: usize) {
    app.mode = Mode::Browsing;

    let Some(entry) = app.entries.get(index).cloned() else {
        return;
    };

    match trash::delete(&entry.path) {
        Ok(()) => {
            app.mode = Mode::Message(format!("moved to recycle bin: {}", entry.name));
            app.loading = true;
            scan::request_scan(cmd_tx, app.current_path.clone());
        }
        Err(e) => {
            app.mode = Mode::Message(format!("delete failed: {e}"));
        }
    }
}
