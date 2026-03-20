use std::net::Ipv4Addr;
use std::fmt;

use crate::headers::tcp::flags_to_string;

use super::transport::Transport;
use super::{PROTO_TCP, PROTO_UDP};
use super::ipv4::Ipv4Packet;
use super::ethernet::EthernetFrame;
use super::tcp::TcpHeader;
use super::udp::UdpHeader;

pub struct Packet {
    pub src_ip: Ipv4Addr,
    pub dst_ip: Ipv4Addr,
    pub transport: Transport,
}

impl Packet {
    pub fn parse(data: &[u8]) -> Option<Packet> {
        let ethernet_frame = EthernetFrame::parse(&data)?; // the ? means 'if this is None, return None from the func'
        if ethernet_frame.ethertype != 0x0800 { return None; }

        let ipv4_packet = Ipv4Packet::parse(&data[14..])?; // same ? here
        let src_ip = Ipv4Addr::from(ipv4_packet.src_ip); 
        let dst_ip = Ipv4Addr::from(ipv4_packet.dst_ip);
    
        let transport: Transport = match ipv4_packet.protocol {
            PROTO_TCP => TcpHeader::parse(&data[34..])
                .map(|tcp| Transport::Tcp(tcp.src_port, tcp.dst_port, tcp.flags))
                .unwrap_or(Transport::Unknown),
            PROTO_UDP => UdpHeader::parse(&data[34..])
                .map(|udp| Transport::Udp(udp.src_port, udp.dst_port))
                .unwrap_or(Transport::Unknown),
            _ => Transport::Unknown,
        };

        Some(Packet{ src_ip, dst_ip, transport })
    }

    // when eth header is stripped off (in nfqueue)
    pub fn parse_ip(data: &[u8]) -> Option<Packet> {
        let ipv4_packet = Ipv4Packet::parse(data)?; // ret None if this is None
        let src_ip = Ipv4Addr::from(ipv4_packet.src_ip);
        let dst_ip = Ipv4Addr::from(ipv4_packet.dst_ip);

        let transport = match ipv4_packet.protocol {
            PROTO_TCP => TcpHeader::parse(&data[20..])
                .map(|tcp| Transport::Tcp(tcp.src_port, tcp.dst_port, tcp.flags))
                .unwrap_or(Transport::Unknown),
            PROTO_UDP => UdpHeader::parse(&data[20..])
                .map(|udp| Transport::Udp(udp.src_port, udp.dst_port))
                .unwrap_or(Transport::Unknown),
            _ => Transport::Unknown,
        };
        
        Some(Packet { src_ip, dst_ip, transport })
    }
}

// display 'trait' for the Packet struct
// think of trait like interface. fmt::Display is an interface, we are implementing
// the interface specifically for Packet
// like __str__ in python - controls how it's displayed
impl fmt::Display for Packet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // [PROTO] SRC_IP:SRC_PORT -> DST_IP:DST_PORT 
        match &self.transport {
            Transport::Tcp(src_port, dst_port, flags) => {
                let flag_str = flags_to_string(*flags);
                write!(f, "[TCP] {}:{} -> {}:{} [{}]", self.src_ip, src_port, self.dst_ip, dst_port, flag_str)
            },
            
            Transport::Udp(src_port, dst_port) => 
                write!(f, "[UDP] {}:{} -> {}:{}", self.src_ip, src_port, self.dst_ip, dst_port),
            
            Transport::Unknown => 
                write!(f, "[UNK] {} -> {}", self.src_ip, self.dst_ip),
        }
    }
}