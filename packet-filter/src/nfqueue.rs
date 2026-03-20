// there are wrappers, but trying to keep it low level
// will b more invovled than the raw socket work from before (capture.rs)
// because libnetfilter_queue has a callback-based C API which looks... awkward
// https://github.com/fqrouter/libnetfilter_queue/blob/master/src/libnetfilter_queue.c

// import primitive C types - rust and C have different type systems, so need to match C func sigs EXACTLY
use libc::{c_int, c_void, uint32_t};
use std::sync::{Arc, Mutex};

use crate::app::AppState;
use crate::headers::Packet;
use crate::rules::{RuleSet, match_rules};

// TODO: constants file?
use crate::capture::{MAX_ETHERNET_FRAME_SIZE};

// libnetfilter_queue has internal structs like nfq_handle that you never actually have to look inside
// you just pass pointers to them between C functions. The _private attr is a zero-size
// field that basically has unknown contents. #[repr(C)] ensures that rust lays it out
// (in memory) the same way C would.
// they are a bit like file descriptors - but as a pointer.
#[repr(C)] 
pub struct NetFilterConnectionHandle { _private: [u8; 0] }

#[repr(C)] 
pub struct NetFilterQueueHandle { _private: [u8; 0] }

#[repr(C)]
pub struct QueuedPacket { _private: [u8; 0] }

// there is noo padding bytes between fields - matches the kernel struc exactly
// must mark as packed or rust will (might???) add padding for aligment, breaking offsets
// https://doc.rust-lang.org/reference/type-layout.html#r-layout.repr.alignment.packed
#[repr(C, packed)]
pub struct QueuedPacketHeader {
    pub packet_id: u32,
    pub hw_protocol: u16,
    pub hook: u8,
}

pub struct CallbackContext {
    pub ruleset: Arc<Mutex<RuleSet>>,
    pub app_state: Arc<Mutex<AppState>>,
}

pub struct QueueResources {
    pub connection: SendableHandle,
    pub queue_handle: *mut NetFilterQueueHandle,
    pub callback_context_ptr: *mut c_void,
    pub file_descriptor: c_int,
}

// mitigate 'cannoot be sent between threads safely' error.. TODO: look into this
pub struct SendableHandle(pub *mut NetFilterConnectionHandle);
unsafe impl Send for SendableHandle {}
// same for QueueResouces
unsafe impl Send for QueueResources {}

// effectively tell rust that these functions exists in C, and this is how they are defined in rust
// it's up to me (the developer) to ensure that the signatures match - the 'unsafe' block means that the 
// compiler will trust that they match the actual library sigs
unsafe extern "C" {
    // https://github.com/fqrouter/libnetfilter_queue/blob/master/src/libnetfilter_queue.c
    // the function names have to match exactly, for the reasons above.
    // you could wrap them in rust functions, but not really much point.
    
    // raw mutable pointer - C's struct 'nfq_handle*'
    fn nfq_open() -> *mut NetFilterConnectionHandle;
    fn nfq_close(handle: *mut NetFilterConnectionHandle) -> c_int;

    // destroy queue handler for cleanup
    fn nfq_destroy_queue(queue_handle: *mut NetFilterQueueHandle) -> c_int;

    // bind the handle of the queue to a protocol family (pf).
    // AF_INET is for IPv4. 
    fn nfq_bind_pf(handle: *mut NetFilterConnectionHandle, pf: u16) -> c_int;

    // create queue number 'num' and registers 'cb' as the callback function.
    // every packet that arrives on the queue will trigger 'cb'.
    // data is the raw ptr to anything to be passed through to the callback
    fn nfq_create_queue(handle: *mut NetFilterConnectionHandle, num: u16, cb: extern "C" fn(*mut NetFilterQueueHandle, *mut c_void, *mut QueuedPacket, *mut c_void) -> c_int, data: *mut c_void) -> *mut NetFilterQueueHandle;
    
    // issue the 'verdict' of a packet (i.e. NF_ACCEPT, NF_DROP).
    // data_len and buf are for if you want to modify the packet (I think?) - pass 0 and null to just accept/drop unmodified
    fn nfq_set_verdict(handle: *mut NetFilterQueueHandle, id: u32, verdict: u32, data_len: u32, buf: *const u8) -> c_int;
    fn nfq_fd(handle: *mut NetFilterConnectionHandle) -> c_int;
    /*
    from: https://github.com/fqrouter/libnetfilter_queue/blob/master/src/libnetfilter_queue.c#907
    struct nfqnl_msg_packet_hdr *nfq_get_msg_packet_hdr(struct nfq_data *nfad)
    {
        return nfnl_get_pointer_to_data(nfad->data, NFQA_PACKET_HDR,
                        struct nfqnl_msg_packet_hdr);
    }
     */
    // struct here is the difficult one... nfq_data refers to...
    // i have created a new struct QueuedPacket for this, unsure if required.
    fn nfq_get_msg_packet_hdr(nfad: *mut QueuedPacket) -> *mut QueuedPacketHeader;

    fn nfq_handle_packet(handle: *mut NetFilterConnectionHandle, buf: *mut c_void, len: c_int) -> c_int;

    // set the copy mode for the queue:
    // NFQNL_COPY_NONE (0) - no packet data (default, nfq_get_payload returns -1)
    // NFQNL_COPY_META (1) - metadata only
    // NFQNL_COPY_PACKET (2) - full packet data; range is the max bytes to copy
    fn nfq_set_mode(handle: *mut NetFilterQueueHandle, mode: u8, range: u32) -> c_int;

    // gets the raw pkt bytes from a queued pkt
    // buf is set to point at the pkt data
    // returns the len of the pkt
    // buf: *mut *mut u8 is a pointer to a pointer.. odd.
    fn nfq_get_payload(packet: *mut QueuedPacket, buf: *mut *mut u8) -> c_int;
}

// this is the callback ('cb') function to be called against every packet
// that arrives on the queue/
// extern "C" because C is calling it
extern "C" fn callback(
    queue_handle: *mut NetFilterQueueHandle, // queue handler - used to call nfq_set_verdict to issue a verdict for a pkt.
    _raw_message: *mut c_void,
    packet_data: *mut QueuedPacket, // pakcet data
    callback_context_ptr: *mut c_void,
) -> c_int {
    unsafe {
        // in the callback, get the packet hdr first
        let packet_header_ptr = nfq_get_msg_packet_hdr(packet_data);
        if packet_header_ptr.is_null() { return 0; }
        
        // cos the struct is marked packed, fields may noot be at aligned mem addrsses
        // e.g. packet_id might be at 0ffset 0x...2 instead of 0x0004
        // read_unaligned copies the bytes regardless of alignment - safe but (slightly) slower
        let packet_header = std::ptr::read_unaligned(packet_header_ptr);
        let packet_id = u32::from_be(packet_header.packet_id);

        // convert the rules back into a type that rust can understand
        // ManuallyDrop avoids 'dropping' the Arc at the end of the callback
        let callback_context = std::mem::ManuallyDrop::new(
            Box::from_raw(callback_context_ptr as *mut CallbackContext)
        );

        // get the packet contents
        let mut payload_ptr: *mut u8 = std::ptr::null_mut();
        // fill payload_ptr with the payload from packet_data
        let payload_len: c_int = nfq_get_payload(
            packet_data, &mut payload_ptr
        );

        if payload_len < 0 || payload_ptr.is_null() {
            // can't  get payload, just accpet it
            nfq_set_verdict(queue_handle, packet_id, NF_ACCEPT, 0, std::ptr::null());
            return 0;
        }

        // convert to rust bytes to parse downstream with headers.rs
        let packet_bytes = std::slice::from_raw_parts(payload_ptr, payload_len as usize);

        // now we have the packet bytes, and we can parse them with headers.rs and establish a verdict
        // note that packet_bytes does not contain a ethernet frame hdr, C func strips this
        let verdict = if let Some(packet) = Packet::parse_ip(packet_bytes) {
            let ruleset = callback_context.ruleset.lock().unwrap();
            let matched = match_rules(&ruleset, &packet); // still <Option>
            let verdict = if matched.is_some() { NF_DROP } else { NF_ACCEPT };
            drop(ruleset); // drop lock

            // update app state and term interface
            // first get lock on appstate
            let mut state = callback_context.app_state.lock().unwrap();
            state.total += 1;

            if verdict == NF_DROP {
                state.matched += 1;
                state.packets.push(format!("{} [BLOCKED]", packet));
            }
            else {
                state.packets.push(format!("{}", packet));
            }

            verdict
        } else {
            NF_ACCEPT //accept if parse failure
        };

        nfq_set_verdict(
            queue_handle,
            packet_id,
            verdict,
            0,
            std::ptr::null(),
        );
    }
    0
}

pub const NF_DROP: u32 = 0;
pub const NF_ACCEPT: u32 = 1;
const NFQNL_COPY_PACKET: u8 = 2;

// ret a tuple of: main handle, queue handle, and filter descriptor
// need all of em: handles for issuing verdicts, fd for reading packets in a loop
// open_queue will take rules to determnie verdict
// maybe this will cause issues / if when rules change, but that's a later problem for now
pub fn open_queue(callback_context: CallbackContext) -> (QueueResources) {
    unsafe { // 'unsafe' feels bad... TODO: is this not rust best practice or just something to accept? 
        let connection_handle = nfq_open();
        if connection_handle.is_null() { panic!("nfq_open() failed for some reason..."); }

        if nfq_bind_pf(connection_handle, libc::AF_INET as u16) < 0 {
            panic!("nfq_bind_pf() failed for some reason...");
        }

        // callback ctx (contains rules) will be passed into the queue
        // BUT it's C, C has no idea what CallbackContext or RulesSet is (or Arc, Mutex), etc.
        // need to convert to a POINTER of this value
        let callback_context_ptr = Box::into_raw(Box::new(callback_context)) as *mut c_void;

        // create the queue itself
        let queue_handle: *mut NetFilterQueueHandle = nfq_create_queue(
            connection_handle, 
            0, 
            callback, 
            callback_context_ptr // passed to callback() as _data
        );
        if queue_handle.is_null() { panic!("nfq_create_queue() failed for some reason - couldn't get queue_handle"); }

        // https://manpages.debian.org/testing/libnetfilter-queue-doc/nfq_set_mode.3.en.html#int_nfq_set_mode_(struct_nfq_q_handle_*_qh,_uint8_t_mode,_uint32_t_range)
        // CRITICAL to call this after nfq_create_queue
        // libnetfilter_queue defaults to NFQNL_COPY_NONE (mode 0), which means
        // 'don't copy packet payload to user space'. nfq_get_payload always return -1 as a result
        // mode 2 (NFQNL_COPY_PACKET) tells kernel to copyy up to 0xffff bytes of
        // each packet's actual data into the buffer (where recv() goes).
        // without this, the callback ifres but packet contents cannot be viewed
        if nfq_set_mode(queue_handle, NFQNL_COPY_PACKET, 0xffff) < 0 {
            panic!("nfq_set_mode() failed");
        }

        // get the file descriptor so we can recv()
        let file_descriptor: i32 = nfq_fd(connection_handle);
        // return all three - conn handle, queue handle, and fd
        let connection: SendableHandle = SendableHandle(connection_handle);
        QueueResources { connection, queue_handle, callback_context_ptr, file_descriptor }
    }
}

// read loop, similar to capture.rs but reading from the nfqeue fs instead of the raw socket fd
pub fn run_queue_loop(connection_handle: SendableHandle, queue_fd: c_int) {
    let mut buf = [0u8; MAX_ETHERNET_FRAME_SIZE * 4];
    loop {
        unsafe {
            let retval: isize = libc::recv(
                queue_fd, // file desc of sock (from open_raw_socket)
                buf.as_mut_ptr() as *mut libc::c_void, // raw ptr to the buffer memory
                buf.len(),  // buffer size - no buf overflows here
                0 // 0 - blocking - recv will wait til a packet arrives
            ) as isize;
            if retval == -1 { continue; } // dont block all traffic when overwhelmed
            nfq_handle_packet(
                connection_handle.0, 
                buf.as_mut_ptr() as *mut c_void,
                retval as c_int
            );
        }
    }
}

// i think this will be done by the OS anyway, and closed when the app quits
// but memory management is 'good to learn' i guess, for low level langs
pub fn close_queue(
    connection_handle: SendableHandle,
    queue_handle: *mut NetFilterQueueHandle,
) {
    unsafe {
        // destroy queue handle
        nfq_destroy_queue(queue_handle);
        // close conn
        nfq_close(connection_handle.0);
    }
}