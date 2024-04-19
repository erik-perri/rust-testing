use std::collections::HashMap;
use std::io;
use std::sync::{atomic::AtomicBool, Arc, Mutex};
use std::thread::{self, JoinHandle};

pub struct Terminal {
    command_handlers:
        Arc<Mutex<HashMap<String, Box<dyn FnMut(Vec<String>) -> Result<(), String> + Send>>>>,
    logger: Arc<Mutex<fn(String)>>,
}

impl Terminal {
    pub fn new(logger: fn(String)) -> Self {
        let command_handlers = Arc::new(Mutex::new(HashMap::new()));
        let logger = Arc::new(Mutex::new(logger));

        Self {
            command_handlers,
            logger,
        }
    }

    pub fn on_command<F>(&mut self, command: &str, callback: F)
    where
        F: FnMut(Vec<String>) -> Result<(), String> + 'static + Send,
    {
        self.command_handlers
            .lock()
            .unwrap()
            .insert(command.to_string(), Box::new(callback));
    }

    pub fn start(&self, is_running: Arc<AtomicBool>) -> JoinHandle<()> {
        let command_handlers = self.command_handlers.clone();
        let logger = self.logger.clone();

        thread::spawn(move || {
            while is_running.load(std::sync::atomic::Ordering::Relaxed) {
                let mut input = String::new();

                io::stdin().read_line(&mut input).unwrap();

                let parts: Vec<&str> = input.split_whitespace().collect();

                if parts.is_empty() {
                    logger.lock().unwrap()("".to_string());
                    continue;
                }

                let command = parts[0];
                let mut handler = command_handlers.lock().unwrap();

                let callback = handler.get_mut(command);

                if callback.is_none() {
                    logger.lock().unwrap()(format!("Invalid command: {}", command));
                    continue;
                }

                let parts = parts.iter().map(|part| part.to_string()).collect();

                match callback.unwrap()(parts) {
                    Ok(()) => {}
                    Err(error) => logger.lock().unwrap()(format!(
                        "Failed to run command {}, {}",
                        command, error
                    )),
                }
            }
        })
    }
}
