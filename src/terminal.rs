use std::io::{self, Write};
use std::sync::{atomic::AtomicBool, mpsc, Arc, Mutex};
use std::thread::{self, JoinHandle};

pub enum TerminalMessage {
    Command(String),
    Output(String),
}

pub struct Terminal {
    receiver: Arc<Mutex<mpsc::Receiver<TerminalMessage>>>,
    pub sender: mpsc::Sender<TerminalMessage>,
}

impl Terminal {
    pub fn new() -> Terminal {
        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));

        Terminal { receiver, sender }
    }

    pub fn listen_for_commands(&self, is_running: &Arc<AtomicBool>) {
        let is_running = Arc::clone(&is_running);
        let sender = self.sender.clone();

        thread::spawn(move || {
            while is_running.load(std::sync::atomic::Ordering::Relaxed) {
                let mut input = String::new();

                io::stdin().read_line(&mut input).unwrap();

                sender
                    .send(TerminalMessage::Command(input.trim().to_string()))
                    .unwrap()
            }
        });
    }

    pub(crate) fn output(&self, message: String) {
        self.sender.send(TerminalMessage::Output(message)).unwrap()
    }

    pub fn start(&self, is_running: &Arc<AtomicBool>) -> JoinHandle<()> {
        let is_running = Arc::clone(&is_running);
        let receiver = Arc::clone(&self.receiver);
        let sender = self.sender.clone();

        thread::spawn(move || {
            let receiver = receiver.lock().unwrap();

            for message in receiver.iter() {
                match message {
                    TerminalMessage::Command(input) => match input.trim() {
                        "exit" | "quit" => {
                            is_running.store(false, std::sync::atomic::Ordering::Relaxed);
                            break;
                        }
                        "" => sender
                            .send(TerminalMessage::Output("".to_string()))
                            .unwrap(),
                        _ => {
                            sender
                                .send(TerminalMessage::Output(format!(
                                    "Invalid command: {}",
                                    input
                                )))
                                .unwrap();
                        }
                    },
                    TerminalMessage::Output(message) => {
                        if !message.is_empty() {
                            println!("\r{}", message);
                        }

                        print!("> ");

                        io::stdout().flush().unwrap();
                    }
                }
            }
        })
    }
}
