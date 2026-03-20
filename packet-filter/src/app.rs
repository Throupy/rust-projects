pub struct AppState {
    pub packets: Vec<String>, // recent packet summaries
    pub matched: u64, // totla rule matches
    pub total: u64, // total pkts seen
    pub rules: Vec<String>,
    pub scroll: u16,
    pub paused: bool,
    pub pause_anchor: usize, // pkt(s) to pause on
}

impl AppState {
    pub fn new() -> AppState {
        AppState {
            packets: Vec::new(),
            matched: 0,
            total: 0,
            rules: Vec::new(),
            scroll: 0,
            paused: false,
            pause_anchor: 0,
        }
    }
}