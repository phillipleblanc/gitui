use std::sync::{
    mpsc::{channel, Receiver, Sender},
    OnceLock,
};

static DEBUG_SENDER: OnceLock<Sender<String>> = OnceLock::new();

pub fn init_debug() -> Receiver<String> {
    let (sender, receiver) = channel();
    let _ = DEBUG_SENDER.set(sender);
    receiver
}

pub fn debug_log(message: &str) {
    let _ = DEBUG_SENDER.get().unwrap().send(message.to_string());
}
