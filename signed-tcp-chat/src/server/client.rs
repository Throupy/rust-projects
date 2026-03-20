use std::{io::{BufRead, BufReader}, net::TcpStream};

use crate::{
    crypto::verify,
    protocol::{Message, MessageKind},
    server::broadcast::{ClientList, broadcast},
};

pub fn handle_client(stream: TcpStream, client_list: ClientList) {
    // on connect — username unknown yet
    let peer_addr = stream.peer_addr().unwrap().to_string();
    client_list.lock().unwrap().push((
        peer_addr.clone(), // clone goes into the list
        "".to_string(),
        stream.try_clone().unwrap(),
    ));
    let mut reader = BufReader::new(stream);

    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => {
                let mut clients = client_list.lock().unwrap();
                let username = clients
                    .iter()
                    .find(|c| c.0 == peer_addr)
                    .map(|c| c.1.clone())
                    .unwrap_or("unknown".to_string());
                println!("[DEBUG] user: {} has left the server", &username);
                clients.retain(|c| c.0 != peer_addr);
                drop(clients);
                broadcast(
                    &client_list,
                    &Message::new(
                        username.clone(),
                        format!("{} has left the server!", &username),
                        MessageKind::ServerEvent,
                    ),
                    None,
                );
                break;
            }
            Ok(_) => {
                let message = Message::from_json(&line).unwrap();
                match message.kind {
                    MessageKind::ServerJoin(ref username) => {
                        println!("[DEBUG] user: '{}' has joined the server", username);
                        let mut clients = client_list.lock().unwrap();
                        if let Some(client) = clients.iter_mut().find(|c| c.0 == peer_addr) {
                            client.1 = username.to_string();
                        }
                        drop(clients);
                        broadcast(&client_list, &message, None);
                    }
                    MessageKind::Chat => {
                        // now verify the HMAC
                        if verify(&message.sender_username, &message.content, &message.hmac) {
                            println!(
                                "[DEBUG] chat from: '{}': {}",
                                message.sender_username, message.content
                            );
                            broadcast(&client_list, &message, None);
                        } else {
                            println!(
                                "[DEBUG] chat from: '{}' was blocked due to incorrect HMAC",
                                message.sender_username
                            );
                            broadcast(
                                &client_list,
                                &Message::new(
                                    "SERVER".to_string(),
                                    format!("Message from {} blocked due to HMAC verification failure", message.sender_username),
                                    MessageKind::ServerEvent,
                                ),
                                Some(&peer_addr),
                            );
                        }
                    }
                    _ => {
                        eprintln!("Unknown message kind");
                    }
                }
            }
            Err(e) => {
                eprintln!("Error: {e}");
                break;
            }
        }
    }
}
