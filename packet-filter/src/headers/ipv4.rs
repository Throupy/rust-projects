pub struct Ipv4Packet {
    pub protocol: u8,
    pub src_ip: [u8; 4],
    pub dst_ip: [u8; 4],
}

impl Ipv4Packet {
    pub fn parse(data: &[u8]) -> Option<Ipv4Packet> {
        if data.len() < 20 { return None; }
    
        let protocol: u8 = data[9];
        let src_ip: [u8; 4] = data[12..16].try_into().unwrap();
        let dst_ip: [u8; 4] = data[16..20].try_into().unwrap();
    
        Some(Ipv4Packet { protocol, src_ip, dst_ip })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_ipv4_data() -> [u8; 20] {
        [
            0x45, 0x00, 0x00, 0x00, // version/IHL, DSCP, total length
            0x00, 0x00, 0x00, 0x00, // identification, flags, fragment offset
            0x00, 0x06, // TTL, protocol (6 = TCP)
            0x00, 0x00, // checksum
            192, 168, 0, 1, // src IP
            8, 8, 8, 8, // dst IP
        ]
    }

    #[test]
    fn test_parse_protocol() {
        let ipv4_packet = Ipv4Packet::parse(&sample_ipv4_data()).unwrap();
        assert_eq!(ipv4_packet.protocol, 6)
    }

    #[test]
    fn test_parse_addresses() {
        let ipv4_packet = Ipv4Packet::parse(&sample_ipv4_data()).unwrap();
        assert_eq!(ipv4_packet.src_ip, [192, 168, 0, 1]);
        assert_eq!(ipv4_packet.dst_ip, [8, 8, 8, 8]);
    }

    #[test]
    fn test_parse_not_enough_data() {
        // first 10 bytes - need 20 to not panic
        let data = &sample_ipv4_data()[0..10];
        assert!(Ipv4Packet::parse(data).is_none());
    }
}