use std::{net::TcpListener, sync::{Arc, Mutex}};

use crate::server::{broadcast::ClientList, client::handle_client};

pub fn start(addr: &str) {
    let client_list: ClientList = Arc::new(Mutex::new(Vec::new()));
    let listener = TcpListener::bind(addr).expect("Failed to bind");
    
    for stream in listener.incoming() {
        let client_list_clone = Arc::clone(&client_list);
        std::thread::spawn(move || {
            handle_client(stream.unwrap(), client_list_clone);
        });
    }
}