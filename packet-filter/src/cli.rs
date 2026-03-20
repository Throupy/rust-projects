use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use std::sync::{Arc, Mutex};

use crate::rules::{save_rule, Rule, RuleSet};
use crate::RULES_FILE;

pub async fn handle_client(socket: TcpStream, ruleset: Arc<Mutex<RuleSet>>) {
    let (reader, mut writer) = tokio::io::split(socket);
    let mut reader = BufReader::new(reader);
    let mut buf = String::new();

    loop {
        buf.clear();
        let bytes_read = reader.read_line(&mut buf).await.unwrap_or(0);

        if bytes_read == 0 { break; } // disconnected

        match buf.trim() {
            "/list" => {
                let lines: Vec<String> = {
                    let locked_ruleset = ruleset.lock().unwrap();
                    locked_ruleset.rules.iter().map(|rule| format!("{}\n", rule.name)).collect()
                };

                for line in lines {
                    writer.write_all(line.as_bytes()).await.unwrap();
                }
            }
            "/quit" => {
                writer.write_all(b"Bye\n").await.unwrap();
                break;
            }
            other if other.starts_with("/block ") => {
                let ip = other.strip_prefix("/block ").unwrap();
                {
                    let mut locked_ruleset = ruleset.lock().unwrap();

                    let new_rule: Rule = Rule { 
                        name: format!("Block {}", ip.to_string()),
                        src_ip: None, 
                        dst_ip: Some(ip.to_string()), 
                        dst_port: None, 
                        protocol: None  
                    };
    
                    locked_ruleset.rules.push(new_rule);
                    save_rule(RULES_FILE, &locked_ruleset);
                }
                writer.write_all(format!("Blocked {}", ip.to_string()).as_bytes()).await.unwrap();
            }
            _ => {
                writer.write_all(b"Unknown Command\n").await.unwrap();
            }
        }
    }
}

pub async fn start_cli(rules: Arc<Mutex<RuleSet>>, port: u16) {
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).await.unwrap();

    loop {
        let (socket, _) = listener.accept().await.unwrap();
        let rules = Arc::clone(&rules);
        tokio::spawn(async move {
            handle_client(socket, rules).await;
        });
    }
}