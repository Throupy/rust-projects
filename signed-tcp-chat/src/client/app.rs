use std::{
    io::{BufRead, BufReader, Write},
    net::TcpStream,
};

use super::tui::App;
use crate::protocol::{Message, MessageKind};

pub fn connect(addr: &str, username: &str) {
    let mut stream = TcpStream::connect(addr).expect("Could not connect to the server");

    let join_message = Message::new(
        username.to_string(),
        "".to_string(),
        MessageKind::ServerJoin(username.to_string()),
    );
    stream
        .write_all((join_message.to_json().unwrap() + "\n").as_bytes())
        .unwrap();
    stream.flush().unwrap();

    let stream_reader = stream.try_clone().unwrap();
    let stream_writer = stream.try_clone().unwrap();

    let (tx, rx) = std::sync::mpsc::channel::<Message>();

    std::thread::spawn(move || {
        let mut reader = BufReader::new(stream_reader);
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => {
                    if let Ok(msg) = Message::from_json(&line) {
                        tx.send(msg).unwrap();
                    }
                }
                Err(_) => break,
            }
        }
    });

    let app = App::new(stream_writer, username.to_string(), rx);
    ratatui::run(|terminal| app.run(terminal)).unwrap();
}