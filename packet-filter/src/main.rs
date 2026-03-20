mod headers;
mod rules;
mod logger;
mod capture;
mod app;
mod tui;
mod nfqueue;

use std::sync::{Arc, Mutex};

use rules::{
    load_rules
};

use crate::app::AppState;
use crate::nfqueue::{CallbackContext, QueueResources, SendableHandle};

const RULES_FILE: &str = "rules.json";

#[tokio::main]
async fn main() {
    let rules = Arc::new(Mutex::new(load_rules(RULES_FILE)));
    let app_state = Arc::new(Mutex::new(AppState::new()));

    // need a block to limit .lock() on rules
    {
        let ruleset = rules.lock().unwrap();
        let mut state = app_state.lock().unwrap();
        // only need names to display.. for now
        state.rules = ruleset.rules.iter().map(|r| r.name.clone()).collect()
    }

    let callback_context = CallbackContext {
        ruleset: Arc::clone(&rules),
        app_state: Arc::clone(&app_state),
    };

    let queue_resources: QueueResources = nfqueue::open_queue(callback_context);
    let sendable = nfqueue::SendableHandle(queue_resources.connection.0);
    let cleanup_handle = nfqueue::SendableHandle(queue_resources.connection.0); // second handle for cleanup

    std::thread::spawn(move || {
        nfqueue::run_queue_loop(sendable, queue_resources.file_descriptor);
    });

    tui::run_tui(Arc::clone(&app_state));
    // reached when users pressed q
    nfqueue::close_queue(cleanup_handle, queue_resources.queue_handle);
}   

