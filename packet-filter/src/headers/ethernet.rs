pub struct EthernetFrame {
    pub dst_mac: [u8; 6],
    pub src_mac: [u8; 6],
    pub ethertype: u16,
}

impl EthernetFrame {
    pub fn parse(data: &[u8]) -> Option<EthernetFrame> {
        if data.len() < 14 { return None; }
    
        //            This type annotation is not needed, but linter puts it in
        let dst_mac: [u8; 6] = data[0..6].try_into().unwrap();
        let src_mac: [u8; 6] = data[6..12].try_into().unwrap();
        let ethertype: u16 = u16::from_be_bytes([data[12], data[13]]);
    
        // Instead of using 'return' keyword, you can just put the below
        // Notice the missing semi-colon - this indicates a retval.
        Some(EthernetFrame {
            dst_mac, src_mac, ethertype
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_ethernet_data() -> [u8; 14] {
        [
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, // dst mac
            0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, // src mac
            0x08, 0x00,  // ethertype IPv4
        ]
    }

    #[test]
    fn test_parse_ethertype() {
        let eth_frame: EthernetFrame = EthernetFrame::parse(&sample_ethernet_data()).unwrap();
        assert_eq!(eth_frame.ethertype, 0x0800);
    }

    #[test]
    fn test_parse_mac_addresses() {
        let eth_frame = EthernetFrame::parse(&sample_ethernet_data()).unwrap();
        assert_eq!(eth_frame.dst_mac, [0x01, 0x02, 0x03, 0x04, 0x05, 0x06]);
        assert_eq!(eth_frame.src_mac, [0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c]);
    }

    #[test]
    fn test_parse_not_enough_data() {
        // first 10 bytes (needs 14 to not panic)
        let data = &sample_ethernet_data()[..10];
        assert!(EthernetFrame::parse(data).is_none());
    }
}