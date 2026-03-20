pub fn open_raw_socket() -> i32 {
    unsafe {
        let sock = libc::socket(
            libc::AF_PACKET, 
            libc::SOCK_RAW, 
            (libc::ETH_P_ARP as u16).to_be() 
        as i32);
        if sock == -1 { panic!("libc::socket() failed"); }
        sock
    }
}

pub fn read_packet(fd: i32, buf: &mut [u8]) -> usize {
    unsafe {
        let retval = libc::recv(
            fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len(), 0
        ) as isize;
        if retval == -1 { panic!("libc::recv() failed"); }
        retval as usize
    }
}

pub fn capture_loop<F>(fd: i32, mut callback: F) 
where 
    F: FnMut(&[u8])
{
    let mut buf = [0u8; 65535];
    loop {
        let len_bytes = read_packet(fd, &mut buf);
        if len_bytes == 0 { continue; }
        callback(&buf[..len_bytes]);
    }
}

pub fn send_arp_request(sock_fd: i32, ifindex: i32, frame: &[u8]) -> () {
    unsafe {
        let mut addr = libc::sockaddr_ll {
            sll_family: libc::AF_PACKET as u16,
            sll_protocol: (libc::ETH_P_ARP as u16).to_be(),
            sll_ifindex: ifindex as i32,
            sll_hatype: 0,
            sll_pkttype: 0,
            sll_halen: 6,
            sll_addr: [0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0, 0], 
        };

        let retval = libc::sendto(
            sock_fd,
            frame.as_ptr() as *const libc::c_void,
            frame.len(),
            0,
            &addr as *const libc::sockaddr_ll as *const libc::sockaddr,
            std::mem::size_of::<libc::sockaddr_ll>() as u32,
        );
        if retval == -1 { panic!("sendto() failed"); }
    }
}