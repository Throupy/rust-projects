use std::{io::Write, net::TcpStream, sync::mpsc};

use crossterm::event::{self, KeyCode, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Layout, Position},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Text},
    widgets::{Block, List, ListItem, Paragraph},
};

use crate::protocol::{Message, MessageKind};

pub enum InputMode {
    Normal,
    Editing,
}

// entire app state for the TUI client
pub struct App {
    pub messages: Vec<Message>,
    pub stream_writer: TcpStream,
    pub input: String,
    pub character_index: usize, // position of the 'cursor' in the text editor area
    pub input_mode: InputMode,
    pub username: String,
    pub receiver: mpsc::Receiver<Message>,
}

impl App {
    pub fn new(
        stream_writer: TcpStream,
        username: String,
        receiver: mpsc::Receiver<Message>,
    ) -> App {
        App {
            messages: Vec::new(),
            stream_writer: stream_writer,
            input: String::new(),
            character_index: 0,
            input_mode: InputMode::Normal,
            username: username,
            receiver: receiver,
        }
    }

    fn move_cursor_left(&mut self) {
        let cursor_moved_left = self.character_index.saturating_sub(1);
        self.character_index = self.clamp_cursor(cursor_moved_left);
    }

    fn move_cursor_right(&mut self) {
        let cursor_moved_right = self.character_index.saturating_add(1);
        self.character_index = self.clamp_cursor(cursor_moved_right);
    }

    fn enter_char(&mut self, new_char: char) {
        let index = self.byte_index();
        self.input.insert(index, new_char);
        self.move_cursor_right();
    }

    // returns the byte indewx based on the character position
    fn byte_index(&self) -> usize {
        self.input
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.character_index)
            .unwrap_or(self.input.len())
    }

    // delete the char immediately to the left of the cursor
    // (backspace behaviour)
    fn delete_char(&mut self) {
        let is_not_cursor_leftmost = self.character_index != 0;
        if is_not_cursor_leftmost {
            let current_index = self.character_index;
            let from_left_to_current_index = current_index - 1;

            // rebuild the string without the deleted character, by chaining
            // the parts before and after it.
            let before_char_to_delete = self.input.chars().take(from_left_to_current_index);
            let after_char_to_delete = self.input.chars().skip(current_index);
            // modify input to this chained result
            self.input = before_char_to_delete.chain(after_char_to_delete).collect();
            self.move_cursor_left();
        }
    }

    // clamp cursor pos to valid bounds (0 - input length)
    fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.input.chars().count())
    }

    fn reset_cursor(&mut self) {
        self.character_index = 0;
    }

    // construct a chat message from the current input, serialise it to JSON,
    // and write it to the TCP stream. Clears the input buffer after sending.
    fn submit_message(&mut self) {
        let input_text = self.input.clone();

        let message: Message =
            Message::new(self.username.to_string(), input_text, MessageKind::Chat);
        let message_json = message.to_json().expect("Failed to parse JSON");
        // require newline, so append it.
        self.stream_writer
            .write_all((message_json + "\n").as_bytes())
            .unwrap();

        // clear the input
        self.input.clear();
        self.reset_cursor();
    }

    // main event loop - drains incoming messages, re-drawes the UI, and handles key presses.
    pub fn run(mut self, terminal: &mut DefaultTerminal) -> std::io::Result<()> {
        loop {
            // drain incoming messages from the recver
            while let Ok(message) = self.receiver.try_recv() {
                self.messages.push(message);
            }

            terminal.draw(|frame| self.render(frame))?;

            if crossterm::event::poll(std::time::Duration::from_millis(50))? {
                if let event::Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        match self.input_mode {
                            InputMode::Normal => match key.code {
                                KeyCode::Char('e') => {
                                    self.input_mode = InputMode::Editing;
                                }
                                KeyCode::Char('q') => {
                                    return Ok(());
                                }
                                _ => {}
                            },
                            InputMode::Editing if key.kind == KeyEventKind::Press => match key.code
                            {
                                KeyCode::Enter => self.submit_message(),
                                KeyCode::Char(to_insert) => self.enter_char(to_insert),
                                KeyCode::Backspace => self.delete_char(),
                                KeyCode::Left => self.move_cursor_left(),
                                KeyCode::Right => self.move_cursor_right(),
                                KeyCode::Esc => self.input_mode = InputMode::Normal,
                                _ => {}
                            },
                            InputMode::Editing => {}
                        }
                    }
                }
            }
        }
    }

    fn render(&self, frame: &mut Frame) {
        let layout = Layout::vertical([
            Constraint::Min(1),
            Constraint::Length(3),
            Constraint::Length(1),
        ]);
        let [messages_area, input_area, help_area] = frame.area().layout(&layout);

        // help bar txt should change based on whether user is typing or not
        let (msg, style) = match self.input_mode {
            InputMode::Normal => (
                vec![
                    "Press".into(),
                    " q".bold(),
                    " to exit, ".into(),
                    "e".bold(),
                    " to start editing".bold(),
                ],
                Style::default().add_modifier(Modifier::RAPID_BLINK),
            ),
            InputMode::Editing => (
                vec![
                    "Press".into(),
                    " Esc".bold(),
                    " to stop editing, ".into(),
                    "Enter".bold(),
                    " to send the message".bold(),
                ],
                Style::default(),
            ),
        };

        let text = Text::from(Line::from(msg)).patch_style(style);
        let help_message = Paragraph::new(text);
        frame.render_widget(help_message, help_area);

        let input = Paragraph::new(self.input.as_str())
            .style(match self.input_mode {
                InputMode::Normal => Style::default(),
                InputMode::Editing => Style::default().fg(Color::Yellow),
            })
            .block(Block::bordered().title("Input"));
        frame.render_widget(input, input_area);
        match self.input_mode {
            InputMode::Normal => {}
            InputMode::Editing => frame.set_cursor_position(Position::new(
                input_area.x + self.character_index as u16 + 1,
                input_area.y + 1,
            )),
        }
        let message_items: Vec<ListItem> =
            self.messages
                .iter()
                .map(|message| match &message.kind {
                    MessageKind::Chat => {
                        ListItem::new(format!("{}: {}", message.sender_username, message.content))
                            .style(
                                Style::default()
                                    .fg(Color::Rgb(200, 200, 200))
                                    .add_modifier(Modifier::BOLD),
                            )
                    }
                    MessageKind::ServerJoin(username) => {
                        ListItem::new(format!("{} has joined the server", username))
                            .style(Style::default().fg(Color::Yellow))
                    }
                    MessageKind::ServerEvent => ListItem::new(message.content.clone())
                        .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                })
                .collect();
        let messages = List::new(message_items).block(Block::bordered().title("Messages"));
        frame.render_widget(messages, messages_area);
    }
}