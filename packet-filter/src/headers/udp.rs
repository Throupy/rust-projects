pub struct UdpHeader {
    pub src_port: u16,
    pub dst_port: u16,
}

impl UdpHeader {
    pub fn parse(data: &[u8]) -> Option<UdpHeader> {
        if data.len() < 8 { return None; }
    
        let src_port = u16::from_be_bytes(data[0..2].try_into().unwrap());
        let dst_port = u16::from_be_bytes(data[2..4].try_into().unwrap());
    
        return Some(UdpHeader { src_port, dst_port })
    }
}