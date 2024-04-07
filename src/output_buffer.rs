use crate::OutputWriter;
use std::io::{self, Write};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self, JoinHandle};

pub struct OutputBuffer {
    receiver: Arc<Mutex<mpsc::Receiver<String>>>,
    sender: mpsc::Sender<String>,
}

impl OutputWriter for OutputBuffer {
    fn output(&self, message: String) {
        self.sender.send(message.to_string()).unwrap();
    }
}

impl OutputBuffer {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));

        OutputBuffer { receiver, sender }
    }

    pub fn start(&self) -> JoinHandle<()> {
        let receiver = Arc::clone(&self.receiver);

        thread::spawn(move || {
            let receiver = receiver.lock().unwrap();

            for message in receiver.iter() {
                if !message.is_empty() {
                    println!("\r{}", message);
                }

                print!("> ");

                io::stdout().flush().unwrap();
            }
        })
    }
}
