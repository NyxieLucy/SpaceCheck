use humansize::{format_size, DECIMAL};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};
use ratatui::Frame;

use crate::app::{App, Mode};

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Top: breadcrumb. Middle: entry table (fills remaining space).
    // Bottom: status/help line.
    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .split(area);

    draw_breadcrumb(frame, chunks[0], app);
    draw_table(frame, chunks[1], app);
    draw_status(frame, chunks[2], app);

    if let Mode::ConfirmDelete { index } = &app.mode {
        draw_confirm_modal(frame, area, app, *index);
    }
}

fn draw_breadcrumb(frame: &mut Frame, area: Rect, app: &App) {
    let text = app.current_path.display().to_string();
    frame.render_widget(Paragraph::new(text).style(Style::default().fg(Color::Cyan)), area);
}

fn draw_table(frame: &mut Frame, area: Rect, app: &App) {
    if app.loading {
        frame.render_widget(Paragraph::new("scanning..."), area);
        return;
    }

    // Biggest entry sets the scale for the little bar graph in each row.
    let max_size = app.entries.iter().map(|e| e.size).max().unwrap_or(1).max(1);

    let rows: Vec<Row> = app
        .entries
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let bar_width = 20;
            let filled = ((entry.size as f64 / max_size as f64) * bar_width as f64) as usize;
            let bar = format!("{}{}", "#".repeat(filled), "-".repeat(bar_width - filled));

            let kind = if entry.is_dir { "/" } else { " " };
            let name = format!("{}{}", entry.name, kind);
            let size = format_size(entry.size, DECIMAL);

            let style = if i == app.selected {
                Style::default().add_modifier(Modifier::REVERSED)
            } else if entry.is_dir {
                Style::default().fg(Color::Blue)
            } else {
                Style::default()
            };

            Row::new(vec![
                Cell::from(name),
                Cell::from(bar),
                Cell::from(size),
            ])
            .style(style)
        })
        .collect();

    let widths = [
        Constraint::Percentage(50),
        Constraint::Length(22),
        Constraint::Percentage(20),
    ];

    let table = Table::new(rows, widths)
        .block(Block::default().borders(Borders::ALL).title("entries (largest first)"));

    frame.render_widget(table, area);
}

fn draw_status(frame: &mut Frame, area: Rect, app: &App) {
    let text = match &app.mode {
        Mode::Message(msg) => msg.clone(),
        _ => "j/k or arrows: move  enter: open  backspace: up  d: delete  q: quit".to_string(),
    };
    frame.render_widget(Paragraph::new(text).style(Style::default().fg(Color::DarkGray)), area);
}

fn draw_confirm_modal(frame: &mut Frame, area: Rect, app: &App, index: usize) {
    let Some(entry) = app.entries.get(index) else {
        return;
    };

    // A small centered box over the existing frame.
    let width = 60.min(area.width.saturating_sub(4));
    let height = 5;
    let modal = Rect {
        x: area.x + (area.width.saturating_sub(width)) / 2,
        y: area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    };

    let lines = vec![
        Line::from(Span::styled(
            format!("Move to Recycle Bin: {}?", entry.name),
            Style::default().fg(Color::Yellow),
        )),
        Line::from(""),
        Line::from("y: confirm    any other key: cancel"),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .title("confirm delete")
        .style(Style::default().fg(Color::Red));

    frame.render_widget(ratatui::widgets::Clear, modal);
    frame.render_widget(Paragraph::new(lines).block(block), modal);
}
