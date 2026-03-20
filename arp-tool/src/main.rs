mod capture;
mod arp_header;

use std::{collections::HashMap, fs::read_to_string, net::Ipv4Addr};

use capture::{open_raw_socket, capture_loop};

use crate::{arp_header::ArpPacket, capture::send_arp_request};

const IFACE: &str = "enp12s0";
const LOCAL_IP: [u8; 4] = [192, 168, 0, 183]; // could get this.. but learning and don't need to

pub fn get_ifindex(iface: &str) -> u32 {
    let index = read_to_string(format!("/sys/class/net/{}/ifindex", iface)).unwrap();
    index.trim().parse::<u32>().expect("invalid index")
}

pub fn get_mac(iface: &str) -> [u8; 6] {
    let mac = read_to_string(format!("/sys/class/net/{}/address", iface)).unwrap();
    
    let mac_bytes: Vec<u8> = mac.trim()
        .split(':')
        .map(|s| u8::from_str_radix(s, 16).expect("invalid hex"))
        .collect();

    mac_bytes.try_into().expect("MAC not 6 bytes")
}


fn main() {
    println!("== ARP Packets ==");

    let mut cache: HashMap<[u8;4], [u8;6]> = HashMap::new();

    let sock_fd = open_raw_socket();

    let my_mac = get_mac(IFACE);
    let my_ifindex = get_ifindex(IFACE);

    let target_ip = [192, 168, 0, 1];
    let frame = ArpPacket::new_request(my_mac, LOCAL_IP, target_ip);

    println!("MAC: {}", ArpPacket::fmt_mac(&my_mac));
    println!("ifindex: {}", my_ifindex);
    println!("===============");
    println!("Going to send an arp request");
    send_arp_request(sock_fd, my_ifindex as i32, &frame);
    println!("Sent ARP request for {}", Ipv4Addr::from(target_ip));

    capture_loop(sock_fd, |data| {
        let ethertype = u16::from_be_bytes([data[12], data[13]]);
        if ethertype != 0x0806 { return; }

        if let Some(arp_packet) = ArpPacket::parse(&data[14..]) {
            println!("Got an arp: {}", arp_packet);

            match u16::from_be_bytes(arp_packet.operation) {
                2 => {
                    // catch the response to add to cache
                    if !cache.contains_key(&arp_packet.sender_ip) {
                        cache.insert(arp_packet.sender_ip, arp_packet.sender_mac);
                        println!("Added to cache: {} ---> {}", Ipv4Addr::from(arp_packet.sender_ip), ArpPacket::fmt_mac(&arp_packet.sender_mac))
                    }
                }
                _ => {}
            }
        }
    });
}
