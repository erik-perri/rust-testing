use crate::OutputWriter;
use std::io::{self, Write};
use std::sync::mpsc;
use std::thread;

pub struct OutputBuffer {
    sender: mpsc::Sender<String>,
}

impl OutputWriter for OutputBuffer {
    fn output(&self, message: String) {
        self.sender.send(message.to_string()).unwrap();
    }
}

impl OutputBuffer {
    pub fn new() -> Self {
        let (sender, receiver): (mpsc::Sender<String>, mpsc::Receiver<String>) = mpsc::channel();

        thread::spawn(move || loop {
            let message = match receiver.recv() {
                Ok(message) => message,
                Err(_) => break,
            };

            if !message.is_empty() {
                println!("\r{}", message);
            }

            print!("> ");

            io::stdout().flush().unwrap();
        });

        OutputBuffer { sender }
    }
}
