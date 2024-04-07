use crate::OutputWriter;
use std::io;
use std::sync::{atomic::AtomicBool, Arc, Mutex};
use std::thread::{self, JoinHandle};

pub struct Terminal {
    logger: Arc<Mutex<dyn OutputWriter + Send>>,
}

impl Terminal {
    pub fn new(logger: Arc<Mutex<dyn OutputWriter + Send>>) -> Self {
        Terminal { logger }
    }

    pub fn start(&self, is_running: &Arc<AtomicBool>) -> JoinHandle<()> {
        let is_running = Arc::clone(&is_running);
        let logger = Arc::clone(&self.logger);

        thread::spawn(move || {
            while is_running.load(std::sync::atomic::Ordering::Relaxed) {
                let mut input = String::new();

                io::stdin().read_line(&mut input).unwrap();

                let parts: Vec<&str> = input.split_whitespace().collect();

                if parts.is_empty() {
                    continue;
                }

                let command = parts[0];

                match command {
                    "exit" | "quit" => {
                        is_running.store(false, std::sync::atomic::Ordering::Relaxed);
                        break;
                    }
                    "" => {
                        logger.lock().unwrap().output("".to_string());
                    }
                    _ => {
                        logger
                            .lock()
                            .unwrap()
                            .output(format!("Invalid command: {}", command));
                    }
                }
            }
        })
    }
}
