// Old raw libc socket capture loop, now replaced by direct nfqueue pipeline
std::thread::spawn(move || {
    // raw sock for passively sniffing traffic
    let fd = open_raw_socket();

    capture_loop(fd, move |data| {
        // if the packet can be parsed
        if let Some(packet) = Packet::parse(data) {

            let matched = {
                let ruleset = rules.lock().unwrap();

                match_rules(&ruleset, &packet)
                    .map(|r| format!(" MATCH: {}", r.name))
                    .unwrap_or_default()
            };

            let mut state = capture_app_state.lock().unwrap();
            state.total += 1;
            if !matched.is_empty() { state.matched += 1; }
            state.packets.push(format!("{}{}", packet, matched));
        }
    });
});