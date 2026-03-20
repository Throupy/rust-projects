// like a c# tagged union, for all intents + purposes
// enum but each variant can carry data
pub enum Transport {
    Tcp(u16, u16, u8), // src_port, dst_port, flags
    Udp(u16, u16),
    Unknown,
}