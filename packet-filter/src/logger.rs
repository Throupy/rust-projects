use std::fs::OpenOptions;
use std::io::Write;

pub fn log_packet(path: &str, message: &str) {
    let mut file = OpenOptions::new()
        .create(true) // create if don't exist
        .append(true) // don't overwrite, addd to end
        .open(path)
        .unwrap();

    writeln!(file, "{}", message).unwrap();
}