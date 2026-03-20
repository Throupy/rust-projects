use std::{io::Write, net::TcpStream, sync::{Arc, Mutex}};

use crate::protocol::Message;

                             //  peer addr, username, stream
pub type ClientList = Arc<Mutex<Vec<(String, String, TcpStream)>>>;

pub fn broadcast(client_list: &ClientList, message: &Message, skip_peer_addr: Option<&str>) {
    let serialised_message: String = message.to_json().expect("Failed to serialise message") + "\n";

    for client in client_list.lock().unwrap().iter_mut() {
        if skip_peer_addr.map_or(true, |addr| client.0 != addr) {
            client.2.write_all(serialised_message.as_bytes()).expect("Error while writing");
            client.2.flush().expect("Failed to flush writer");
        }
    }
}