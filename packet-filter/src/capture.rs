pub const MAX_ETHERNET_FRAME_SIZE: usize = 65535;

pub fn open_raw_socket() -> i32 {
    unsafe {
        let sock = libc::socket(
            libc::AF_PACKET, 
            libc::SOCK_RAW, 
            (libc::ETH_P_ALL as u16).to_be() 
        as i32);

        if sock == -1 { panic!("Failed to open raw socket - are you running with cap_net_raw?") }
        sock
    }
}

pub fn read_packet(fd: i32, buf: &mut [u8]) -> usize {
    unsafe {
        let retval: isize = libc::recv(
            fd, // file desc of sock (from open_raw_socket)
            buf.as_mut_ptr() as *mut libc::c_void, // raw ptr to the buffer memory
            buf.len(),  // buffer size - no buf overflows here
            0 // 0 - blocking - recv will wait til a packet arrives
        ) as isize;
        if retval == -1 { panic!("recv() failed"); }
        retval as usize // where retval is not -1 (fail) or 0 (conn closed), it's the number of bytes written to the buf
    }
}

pub fn capture_loop<F>(fd: i32, mut callback: F) // 'F' is just a generic type
where // constinats on the generic type
    F: FnMut(&[u8]) // 'F must be callable, takes a &[u8], returns nothing'
// effectively the whole thing means 'capture_loop takes a file descriptor, and 
// ANY callable that accepts a byte slice
// cool but kind of confusing syntax though!
{
    let mut buf = [0u8; MAX_ETHERNET_FRAME_SIZE];
    loop {
        let len_bytes = read_packet(fd, &mut buf);
        if len_bytes == 0 { continue; }
        callback(&buf[..len_bytes]);
    }
}