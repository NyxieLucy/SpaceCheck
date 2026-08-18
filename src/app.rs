use std::path::PathBuf;

/// A single row we can show in the TUI: either a file or a directory,
/// living directly inside `App.current_path`.
#[derive(Debug, Clone)]
pub struct Entry {
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
    pub is_dir: bool,
}

/// Messages sent from the background scanning thread back to the UI thread.
/// We never touch the filesystem on the UI thread for anything that could
/// be slow (recursively sizing a directory), so the UI stays responsive.
pub enum ScanMessage {
    /// A directory listing (with per-entry sizes already computed) is ready.
    DirReady {
        path: PathBuf,
        entries: Vec<Entry>,
    },
    /// Something went wrong while scanning (permissions, missing dir, etc).
    Error { path: PathBuf, message: String },
}

/// Commands sent from the UI thread to the background scanning thread.
pub enum ScanCommand {
    Scan(PathBuf),
    /// Ask the worker thread to shut down.
    Quit,
}

/// What the UI is currently doing. Kept separate from `App` fields so the
/// draw function can pattern-match on a single value instead of juggling
/// several booleans.
pub enum Mode {
    Browsing,
    /// Asking "really delete this?" before we touch anything.
    ConfirmDelete { index: usize },
    /// Showing a one-line error/status message until the next keypress.
    Message(String),
}

pub struct App {
    pub current_path: PathBuf,
    pub entries: Vec<Entry>,
    pub selected: usize,
    pub loading: bool,
    pub mode: Mode,
    pub should_quit: bool,
}

impl App {
    pub fn new(start_path: PathBuf) -> Self {
        Self {
            current_path: start_path,
            entries: Vec::new(),
            selected: 0,
            loading: true,
            mode: Mode::Browsing,
            should_quit: false,
        }
    }

    /// Currently highlighted entry, if any (the list can be empty for an
    /// empty directory).
    pub fn selected_entry(&self) -> Option<&Entry> {
        self.entries.get(self.selected)
    }

    pub fn move_selection(&mut self, delta: i32) {
        if self.entries.is_empty() {
            return;
        }
        let len = self.entries.len() as i32;
        let mut next = self.selected as i32 + delta;
        next = next.clamp(0, len - 1);
        self.selected = next as usize;
    }

    /// Replace the current listing once a scan finishes, sorted largest
    /// first so the biggest space hogs are always at the top.
    pub fn set_entries(&mut self, path: PathBuf, mut entries: Vec<Entry>) {
        // Ignore stale results from a scan we've since navigated away from.
        if path != self.current_path {
            return;
        }
        entries.sort_by(|a, b| b.size.cmp(&a.size));
        self.entries = entries;
        self.selected = 0;
        self.loading = false;
    }
}
