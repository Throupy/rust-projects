use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use serde_json::{from_str, to_string};

use crate::crypto::sign;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum MessageKind {
    ServerJoin(String), // username
    Chat,
    ServerEvent,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Message {
    pub timestamp: u64, // seconds since epoch (serde doesn't like SystemTime)
    pub sender_username: String,
    pub content: String,
    pub kind: MessageKind,
    pub hmac: String,
}

impl Message {
    pub fn new(
        sender_username: String,
        content: String,
        kind: MessageKind,
    ) -> Message {
        // generate the hmac and timestamp
        let hmac: String = sign(&sender_username, &content);
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        Message { timestamp, sender_username, content, kind, hmac }
    }

    pub fn from_json(json_payload: &str) -> Result<Message, serde_json::Error> {
        from_str(json_payload)
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        to_string(&self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn util_gen_message() -> Message {
        Message::new(
            "owen".to_string(),
            "hello, world".to_string(),
            MessageKind::Chat,
        )
    }

    #[test]
    pub fn test_json_serialisation_round_trip() {
        // seriailise, then deserialise
        let message: Message = util_gen_message();

        let message_serialised: String = message.to_json().unwrap();
        let message_deserialised: Message = Message::from_json(
            &message_serialised.as_str()
        ).unwrap();

        // I could just dervice PartialEq across Message + MessageKind,
        // and compare them. However comparing the actual fields
        // is more of a functional test in my opinion...
        assert_eq!(message.sender_username, message_deserialised.sender_username);
        assert_eq!(message.content, message_deserialised.content);
        assert_eq!(message.hmac, message_deserialised.hmac);
        assert_eq!(message.timestamp, message_deserialised.timestamp);
    }

    #[test]
    pub fn test_to_json_contains_field_names() {
        let message: Message = util_gen_message();
        let message_json = message.to_json().unwrap();
        assert!(message_json.contains("sender_username"));
        assert!(message_json.contains("hmac"));
    }

    #[test]
    pub fn test_from_json_invalid_input() {
        assert!(Message::from_json("this is invalid JSON").is_err());
    }

    #[test]
    pub fn test_from_json_missing_field() {
        let json = r#"{"timestamp":0,"sender_username":"owen","kind":"Chat","hmac":"abc"}"#;
        assert!(Message::from_json(json).is_err());
    }

    #[test]
    pub fn test_from_json_wrong_type() {
        let json = r#"{"timestamp":"not_a_number","sender_username":"owen","content":"hello","kind":"Chat","hmac":"abc"}"#;
        assert!(Message::from_json(json).is_err());
    }
}