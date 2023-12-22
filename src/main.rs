use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::sync::mpsc;
use std::thread;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::screen::AlternateScreen;
use tui::backend::TermionBackend;
use tui::layout::{Constraint, Direction, Layout};
use tui::widgets::{Block, Borders, List, ListItem, Paragraph, Widget, Wrap};
use tui::Terminal;
use std::time::Duration;
use chrono;

fn append_to_log(logs: Arc<Mutex<Vec<String>>>) {
    thread::spawn(move || {
        // This thread will continuously append a line to the log vector every 5 seconds
        loop {
            thread::sleep(Duration::from_secs(1));
            logs.lock().unwrap().push(format!("Additional log message at {}", chrono::Local::now()));
        }
    });
}

fn main() -> Result<(), io::Error> {
    let (tx, rx) = mpsc::channel::<Key>();

    // Set up terminal and backend
    let stdout = io::stdout().into_raw_mode()?;
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create a shared log vector wrapped in Arc and Mutex
    let logs: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![
        "Starting mining process...".to_owned(),
        "New block found!".to_owned(),
        "Received share accepted by the pool.".to_owned(),
    ]));

    // Create a separate Arc for the TUI to use
    let tui_logs = logs.clone();

    // Spawn a separate thread to append to the log
    append_to_log(logs.clone());
    // Spawn a separate thread to handle keyboard events
    let tx_kb = tx.clone();
    thread::spawn(move || {
        let stdin = io::stdin();
        for key in stdin.keys() {
            if let Ok(key) = key {
                if tx_kb.send(key).is_err() {
                    break;
                }
            }
        }
    });

    // Example stats and log - make sure these are mutable if they change over time
    let mut stats = "Hash rate: 50 MH/s | Accepted shares: 1024 | Rejected shares: 16".to_owned();

    // Clear the screen
    terminal.clear()?;

    loop {
        // Rendering the UI
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(2)
                .constraints([
                    Constraint::Percentage(80),
                    Constraint::Percentage(20),
                ].as_ref())
                .split(f.size());

            // Render log area
            // Calculate the start index for the visible area
            let mut log_items_locked = tui_logs.lock().unwrap();
            let total_log_items = log_items_locked.len();
            let visible_log_items = if total_log_items >= chunks[0].height as usize {
                log_items_locked.split_at(total_log_items - chunks[0].height as usize).1.to_owned()
            } else {
                log_items_locked.clone()
            };

            // Render log area with scrolling
            let log = List::new(visible_log_items.iter().map(|item| ListItem::new(item.to_string())).collect::<Vec<ListItem>>())
                .block(Block::default().title("Log").borders(Borders::ALL));
            f.render_widget(log, chunks[0]);
            // Render stats area
            let stats_text = Paragraph::new(stats.clone())
                .block(Block::default().title("Stats").borders(Borders::ALL))
                .wrap(Wrap { trim: true });
            f.render_widget(stats_text, chunks[1]);
        })?;

        // Handle input events received from the channel
        if let Ok(key) = rx.try_recv() {
            match key {
                Key::Char('q') => break,  // Exit the program if the 'q' key is pressed
                Key::Ctrl('c') => break,  // Exit the program if CTRL+c is pressed
                // Handle other types of key events for TUI here...
                _ => {}
            }
        }
    }

    // For completeness, the terminal should be properly cleaned up here, but in this case Rust's RAII will handle it

    Ok(())
}