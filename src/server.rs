use crate::node_state::NodeState;
use crate::OutputWriter;
use std::net::TcpListener;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;

pub struct Server {
    pub bind_address: String,
    pub port: u16,
    logger: Arc<Mutex<dyn OutputWriter + Send>>,
}

impl Server {
    pub fn new(bind_address: &str, port: u16, logger: Arc<Mutex<dyn OutputWriter + Send>>) -> Self {
        Server {
            bind_address: bind_address.to_string(),
            logger,
            port,
        }
    }

    pub fn start(
        &self,
        is_running: &Arc<AtomicBool>,
        node_state: &NodeState,
    ) -> Result<JoinHandle<()>, String> {
        let is_running = Arc::clone(&is_running);
        let bind_address = format!("{}:{}", self.bind_address, self.port);

        let listener = TcpListener::bind(&bind_address).map_err(|error| {
            format!(
                "Failed to bind to address {}: {}. Is the port already in use?",
                bind_address, error
            )
        })?;

        let logger = Arc::clone(&self.logger);
        let node_id = node_state.node_id.clone();
        let bind_address = bind_address.clone();

        Ok(thread::spawn(move || {
            logger
                .lock()
                .unwrap()
                .output(format!("Node [{}] listening on {}", node_id, bind_address,));

            for stream in listener.incoming() {
                // TODO Pull in a library for non-blocking IO so we can check this regularly
                //      and not only on new connections
                if !is_running.load(std::sync::atomic::Ordering::Relaxed) {
                    break;
                }

                let stream = match stream {
                    Ok(stream) => stream,
                    Err(error) => {
                        logger
                            .lock()
                            .unwrap()
                            .output(format!("Failed to accept connection: {}", error));
                        continue;
                    }
                };

                logger.lock().unwrap().output(format!(
                    "Connection established to {}",
                    stream.peer_addr().unwrap()
                ));
            }
        }))
    }
}
