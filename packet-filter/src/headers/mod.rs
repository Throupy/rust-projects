pub const PROTO_TCP: u8 = 6;
pub const PROTO_UDP: u8 = 17;

mod ethernet;
mod ipv4;
mod packet;
mod tcp;
mod transport;
mod udp;

pub use ethernet::EthernetFrame;
pub use ipv4::Ipv4Packet;
pub use tcp::TcpHeader;
pub use udp::UdpHeader;
pub use transport::Transport;
pub use packet::Packet;