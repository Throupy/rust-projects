pub const FLAG_FIN: u8 = 0x01;
pub const FLAG_SYN: u8 = 0x02;
pub const FLAG_RST: u8 = 0x04;
pub const FLAG_PSH: u8 = 0x08;
pub const FLAG_ACK: u8 = 0x10;
pub const FLAG_URG: u8 = 0x20;


pub struct TcpHeader {
    pub src_port: u16,
    pub dst_port: u16,
    pub flags: u8,
}

impl TcpHeader {
    pub fn parse(data: &[u8]) -> Option<TcpHeader> {
        if data.len() < 20 { return None; }
    
        let src_port = u16::from_be_bytes(data[0..2].try_into().unwrap());
        let dst_port = u16::from_be_bytes(data[2..4].try_into().unwrap());
        let flags = data[13].try_into().unwrap();
    
        Some(TcpHeader { src_port, dst_port, flags })
    }
}

pub fn flags_to_string(flags: u8) -> String {
    // small util func to convert flag e.g. 0x20 to string e.g. URG
    // maybe a better way to do this, and maybe something from std::net
    // but it'll do. learning after all.
    let mut parts = Vec::new();
    if flags & FLAG_SYN != 0 { parts.push("SYN"); }
    if flags & FLAG_ACK != 0 { parts.push("ACK"); }
    if flags & FLAG_FIN != 0 { parts.push("FIN"); }
    if flags & FLAG_RST != 0 { parts.push("RST"); }
    if flags & FLAG_PSH != 0 { parts.push("PSH"); }
    if flags & FLAG_URG != 0 { parts.push("URG"); }
    parts.join("|")
}