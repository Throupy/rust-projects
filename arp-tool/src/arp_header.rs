use std::{fmt, net::Ipv4Addr};

pub struct ArpPacket {
    pub hw_type: [u8; 2],
    pub proto_type: [u8; 2],
    pub hw_len: u8,
    pub proto_len: u8,
    pub operation: [u8; 2],
    pub sender_mac: [u8; 6],
    pub sender_ip: [u8; 4],
    pub target_mac: [u8; 6],
    pub target_ip: [u8; 4],
}

impl ArpPacket {
    pub fn parse(data: &[u8]) -> Option<ArpPacket> {
        if data.len() < 28 { return None; }
        // sender MAC - 8-12
        // tgt MAC - 16-20
        // tgt IP - 20-24
        let hw_type = data[0..2].try_into().ok()?;
        let proto_type = data[2..4].try_into().ok()?;
        let hw_len = data[4].try_into().ok()?;
        let proto_len = data[5].try_into().ok()?;
        let operation = data[6..8].try_into().ok()?;
        let sender_mac = data[8..14].try_into().ok()?;
        let sender_ip = data[14..18].try_into().ok()?;
        let target_mac = data[18..24].try_into().ok()?;
        let target_ip = data[24..28].try_into().ok()?;
        Some(ArpPacket { 
            hw_type, proto_type, hw_len, proto_len, operation, 
            sender_mac, sender_ip, target_mac, target_ip 
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        // convert from ArpPacket instance to bytes to send over wire
        let mut buf = Vec::with_capacity(28);
        buf.extend_from_slice(&self.hw_type);
        buf.extend_from_slice(&self.proto_type);
        buf.push(self.hw_len);
        buf.push(self.proto_len);
        buf.extend_from_slice(&self.operation);
        buf.extend_from_slice(&self.sender_mac);
        buf.extend_from_slice(&self.sender_ip);
        buf.extend_from_slice(&self.target_mac);
        buf.extend_from_slice(&self.target_ip);
        buf
    }

    pub fn new_request(sender_mac: [u8;6], sender_ip: [u8;4], target_ip:[u8;4]) -> Vec<u8> {
        let arp_request_packet = ArpPacket {
            hw_type: [0x00, 0x01], // eth
            proto_type: [0x08, 0x00],
            hw_len: 6,
            proto_len: 4,
            operation: [0x00, 0x01],
            sender_mac,
            sender_ip,
            target_mac: [0x00; 6], // zeros for req
            target_ip
        };

        // now eth frame
        let mut frame = Vec::with_capacity(42);
        frame.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]); // broadcast
        frame.extend_from_slice(&sender_mac);
        frame.extend_from_slice(&[0x08, 0x06]); // eth type arp
        frame.extend_from_slice(&arp_request_packet.to_bytes());
        frame
    }

    pub fn fmt_mac(mac: &[u8; 6]) -> String {
        format!("{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            mac[0], mac[1], mac[2], mac[3], mac[4], mac[5])
    }
}

impl fmt::Display for ArpPacket {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let operation = u16::from_be_bytes(self.operation);
        match operation {
            1 => {
                write!(f, 
                    "[REQUEST] who has {}? tell {}", 
                    Ipv4Addr::from(self.target_ip), 
                    Ipv4Addr::from(self.sender_ip)
                )
            }
            2 => {
                write!(f, 
                    "[RESPONSE] {} has {}", 
                    Ipv4Addr::from(self.sender_ip), 
                    ArpPacket::fmt_mac(&self.sender_mac)
                )
            }
            _ => write!(f, "Unknown")
        }
    }
}