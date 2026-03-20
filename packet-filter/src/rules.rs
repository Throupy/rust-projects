use std::net::Ipv4Addr;
use std::{fs::File, io::BufReader, path::Path};

use ipnetwork::Ipv4Network;
use serde::{Deserialize, Serialize};

use crate::headers::Packet;

use crate::headers::Transport;

#[derive(Debug, Deserialize, Serialize)]
pub struct RuleSet {
    pub rules: Vec<Rule>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Rule {
    pub name: String,
    #[serde(deserialize_with = "deserialize_ipv4_network")]
    pub src_ip: Option<Ipv4Network>,
    #[serde(deserialize_with = "deserialize_ipv4_network")]
    pub dst_ip: Option<Ipv4Network>,
    pub dst_port: Option<u16>,
    pub protocol: Option<String>, // TCP/UDP for human readable json
}


fn deserialize_ipv4_network<'de, D>(deserializer: D) -> Result<Option<Ipv4Network>, D::Error>
where
    D: serde::Deserializer<'de> 
{
    // read the raw JSON val as an Option<String>
    // if present, we get Some("X.X.X.X/X") or Some("X.X.X.X") (no cidr)
    let ip_value: Option<String> = Option::deserialize(deserializer)?;

    match ip_value {
        Some(raw_string) => {
            // field present
            let normalised = if !raw_string.contains('/') {
                format!("{}/32", raw_string)
            } else { raw_string };

            normalised.parse::<Ipv4Network>()
                .map(Some) // wrap Ok value back into Option
                .map_err(serde::de::Error::custom) // convert parse error into serde error
        },

        None => Ok(None),
    }
}


pub fn load_rules(path: &str) -> RuleSet {
    if !Path::new(path).exists() {
        // raw string literal in rust - # means you don't have to escape inner quotes
        let default = r#"{"rules": []}"#;
        std::fs::write(path, default).unwrap();
        println!("No rules.json found, created one.");
    }

    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);
    serde_json::from_reader(reader).unwrap()
}

pub fn save_rule(path: &str, ruleset: &RuleSet) {
    let serialized = serde_json::to_string_pretty(ruleset).unwrap();
    std::fs::write(path, serialized).unwrap();
}

pub fn match_rules<'a>(ruleset: &'a RuleSet, packet: &Packet) -> Option<&'a Rule> {
    // unpack the union enum data
    let (dst_port, protocol): (Option<u16>, &str) = match packet.transport {
        Transport::Tcp(_, dst_port, _) => (Some(dst_port), "tcp"),
        Transport::Udp(_, dst_port) => (Some(dst_port), "udp"),
        Transport::Unknown => (None, "unknown"),
    };

    for rule in &ruleset.rules {

        if let Some(rule_dst_port) = &rule.dst_port {
            if Some(*rule_dst_port) != dst_port { continue; }
        }
        if let Some(rule_dst_ip_range) = &rule.dst_ip {
            if !rule_dst_ip_range.contains(packet.dst_ip) { continue }
        }
        if let Some(rule_src_ip_range) = &rule.src_ip {
            if !rule_src_ip_range.contains(packet.src_ip) { continue }
        }
        if let Some(rule_protocol) = &rule.protocol {
            if rule_protocol.as_str() != protocol { continue }
        }
        return Some(rule);
    }
    None
} 

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_cidr_match() {
        let ruleset = RuleSet {
            rules: vec![Rule {
                name: "test".to_string(),
                src_ip: None,
                dst_ip: "10.0.0.0/24".parse().ok(),
                dst_port: None,
                protocol: None,
            }]
        };

        // gen a packet, make sure it gets matched by rule
        let packet = Packet {
            src_ip: Ipv4Addr::new(192, 168, 0, 1),
            dst_ip: Ipv4Addr::new(10, 0, 0, 212),
            transport: Transport::Unknown,
        };

        let verdict = match_rules(&ruleset, &packet);
        assert!(verdict.is_some());
    }

    #[test]
    fn test_single_ip_match() {
        // need to parse like this for cusotm deserialisation to kick in
        let rule: Rule = serde_json::from_str(r#"{
            "name": "test",
            "src_ip": null,
            "dst_ip": "10.0.0.3",
            "dst_port": null,
            "protocol": null
        }"#).unwrap();
        let mut ruleset: RuleSet = RuleSet { rules: Vec::new() };
        ruleset.rules.push(rule);

        // gen a packet, make sure it gets matched by rule
        let packet = Packet {
            src_ip: Ipv4Addr::new(192, 168, 0, 1),
            dst_ip: Ipv4Addr::new(10, 0, 0, 3),
            transport: Transport::Unknown,
        };

        let verdict = match_rules(&ruleset, &packet);
        assert!(verdict.is_some());
    }

    #[test]
    fn test_single_ip_with_explicit_32_cidr_match() {
        // need to parse like this for cusotm deserialisation to kick in
        let rule: Rule = serde_json::from_str(r#"{
            "name": "test",
            "src_ip": null,
            "dst_ip": "10.0.0.3/32",
            "dst_port": null,
            "protocol": null
        }"#).unwrap();
        let mut ruleset: RuleSet = RuleSet { rules: Vec::new() };
        ruleset.rules.push(rule);

        // gen a packet, make sure it gets matched by rule
        let packet = Packet {
            src_ip: Ipv4Addr::new(192, 168, 0, 1),
            dst_ip: Ipv4Addr::new(10, 0, 0, 3),
            transport: Transport::Unknown,
        };

        let verdict = match_rules(&ruleset, &packet);
        assert!(verdict.is_some());
    }

    #[test]
    fn test_out_of_cidr_range_pass() {
        let ruleset = RuleSet {
            rules: vec![Rule {
                name: "test".to_string(),
                src_ip: None,
                dst_ip: "10.0.0.0/30".parse().ok(), // /30 means .1 - .3 (usable)
                dst_port: None,
                protocol: None,
            }]
        };

        // gen a packet, make sure it gets matched by rule
        let packet = Packet {
            src_ip: Ipv4Addr::new(192, 168, 0, 1),
            // .5 should pass
            dst_ip: Ipv4Addr::new(10, 0, 0, 5),
            transport: Transport::Unknown,
        };

        let verdict = match_rules(&ruleset, &packet);
        assert!(verdict.is_none());
    }

    #[test]
    fn test_port_match() {
        let ruleset = RuleSet {
            rules: vec![Rule {
                name: "test".to_string(),
                src_ip: None,
                dst_ip: "10.0.0.10".parse().ok(), 
                dst_port: Some(1234),
                protocol: None,
            }]
        };

        // gen a packet, make sure it gets matched by rule
        let packet = Packet {
            src_ip: Ipv4Addr::new(192, 168, 0, 1),
            // .5 should pass
            dst_ip: Ipv4Addr::new(10, 0, 0, 10),
            transport: Transport::Tcp(4321, 1234, 0x08),
        };

        let verdict = match_rules(&ruleset, &packet);
        assert!(verdict.is_some());
    }

    #[test]
    fn test_protocol_match() {
        let ruleset = RuleSet {
            rules: vec![Rule {
                name: "test".to_string(),
                src_ip: None,
                dst_ip: None,
                dst_port: Some(1234),
                protocol: "udp".parse().ok(),
            }]
        };

        // gen a packet, make sure it gets matched by rule
        let packet = Packet {
            src_ip: Ipv4Addr::new(192, 168, 0, 1),
            // .5 should pass
            dst_ip: Ipv4Addr::new(10, 0, 0, 10),
            transport: Transport::Udp(4321, 1234),
        };

        let verdict = match_rules(&ruleset, &packet);
        assert!(verdict.is_some());
    }
}