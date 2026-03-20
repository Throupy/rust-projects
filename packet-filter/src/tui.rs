use std::sync::{Arc, Mutex};

use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    style::{Color, Style},
    Terminal,
};

use crossterm::{
    execute,
    terminal::{
        enable_raw_mode, disable_raw_mode,
        EnterAlternateScreen, LeaveAlternateScreen
    },
};

use crate::app::AppState;

pub fn run_tui(app_state: Arc<Mutex<AppState>>) {
    // setup terminal
    enable_raw_mode().unwrap();
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen).unwrap();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).unwrap();

    loop {
        terminal.draw(|f| {

            // layout, split screen into 3 vertical chunks (for now... might change)
            let main_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(68),
                    Constraint::Percentage(24),
                    Constraint::Percentage(8),
                ])
                .split(f.area());
        
            let bottom_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(50),
                    Constraint::Percentage(50),
                ])
                .split(main_chunks[1]);

            let state = app_state.lock().unwrap();

            // get the skip val - 0 if not paused, otherwise scroll value (num to skip)
            let skip = if state.paused { 
                let from_end = state.packets.len().saturating_sub(state.pause_anchor);
                from_end + state.scroll as usize
            } else { 
                0 
            };

            let total = state.packets.len();
            // was hard coding the val here, but i can get the height of the widget actually
            let visible = main_chunks[0].height as usize - 2;
            let start = total.saturating_sub(visible + skip);
            let end = total.saturating_sub(skip);
            // packet log pane
            let items: Vec<ListItem> = state.packets[start..end].iter()
                .map(|p| {
                    let style = if p.contains("BLOCK") {
                        Style::default().fg(Color::Red)
                    } else if p.contains("[UNK]") {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default().fg(Color::Green)
                    };
                    ListItem::new(p.as_str()).style(style)
                })
                .collect();

            let title = if state.paused { "Packets [PAUSED] (space to resume)" } else { "Packets [LIVE] (UP to pause)" };
            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title(title));
            f.render_widget(list, main_chunks[0]);

            // stats pane
            let stats_text = format!(
                "Total Packets Recieved: {}\nTotal Rule Matches: {}", 
                state.total, state.matched,
            );
            let stats_para = Paragraph::new(stats_text)
                .block(Block::default().borders(Borders::ALL).title("Stats"));
            f.render_widget(stats_para, bottom_chunks[1]);

            
            let rules_text = state.rules.iter()
                .take(10) // tkae 10 as max, otherwise won't fit... (until scrolling ;) )
                .cloned()
                .collect::<Vec<_>>() // <T> of _ means 'infer the type'
                .join("\n");
            
            let rules_para = Paragraph::new(rules_text)
                .block(Block::default().borders(Borders::ALL).title("Loaded Rules"));
            f.render_widget(rules_para, bottom_chunks[0]);

            let help_para = Paragraph::new("q: quit")
            // using tmux so cna't see content.. stick in title for now. TODO
                .block(Block::default().borders(Borders::ALL).title("q: quit"));
            f.render_widget(help_para, main_chunks[2]);

        }).unwrap();

        // handle input
        if crossterm::event::poll(std::time::Duration::from_millis(100)).unwrap() {
            if let crossterm::event::Event::Key(key) = crossterm::event::read().unwrap() {
                match key.code {
                    crossterm::event::KeyCode::Char('q') => break,
                    crossterm::event::KeyCode::Up => {
                        let mut state = app_state.lock().unwrap();
                        if !state.paused {
                            state.paused = true;
                            state.pause_anchor = state.packets.len();
                        }
                        state.scroll = state.scroll.saturating_add(1);
                        drop(state);
                    }
                    crossterm::event::KeyCode::Down => {
                        let mut state = app_state.lock().unwrap();
                        state.paused = true;
                        state.scroll = state.scroll.saturating_sub(1);
                        if state.scroll == 0 { state.paused = false; }
                    }
                    crossterm::event::KeyCode::Char(' ') => {
                        let mut state = app_state.lock().unwrap();
                        state.paused = false;
                        state.scroll = 0;
                    }
                    _ => {}
                }
            }
        }
    }
    
    // cleanup terminal
    disable_raw_mode().unwrap();
    execute!(terminal.backend_mut(), LeaveAlternateScreen).unwrap();
}

