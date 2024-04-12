use crate::app_state::AppState;
use crate::thread_joiner::ThreadJoiner;
use serde::{Deserialize, Serialize};
use std::net::{SocketAddr, UdpSocket};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::sleep;
use std::{io, thread};

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct FoundNode {
    pub node_id: String,
    pub address: String,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub enum Message {
    Ping,
    Pong,
    Store(String, Vec<u8>),
    FindNode(String),
    FindNodeResponse(Vec<FoundNode>),
    FindValue(String),
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct Packet {
    pub node_id: String,
    pub transaction_id: String,
    pub message: Message,
}

pub struct Server {
    pub bind_address: String,
    pub port: u16,
    logger: Arc<Mutex<fn(String)>>,
    outgoing_sender: Sender<(SocketAddr, Vec<u8>)>,
    outgoing_receiver: Arc<Mutex<Receiver<(SocketAddr, Vec<u8>)>>>,
    receive_callbacks:
        Arc<Mutex<Vec<Box<dyn FnMut(SocketAddr, Packet) -> Result<(), String> + Send>>>>,
}

impl Server {
    pub fn new(bind_address: &str, port: u16, logger: fn(String)) -> Self {
        let (outgoing_sender, outgoing_receiver): (
            Sender<(SocketAddr, Vec<u8>)>,
            Receiver<(SocketAddr, Vec<u8>)>,
        ) = mpsc::channel();

        let logger = Arc::new(Mutex::new(logger));
        let outgoing_receiver = Arc::new(Mutex::new(outgoing_receiver));

        let receive_callbacks = Arc::new(Mutex::new(vec![]));

        Self {
            bind_address: bind_address.to_string(),
            logger,
            outgoing_sender,
            outgoing_receiver,
            port,
            receive_callbacks,
        }
    }

    pub fn on_receive<F>(&mut self, callback: F)
    where
        F: FnMut(SocketAddr, Packet) -> Result<(), String> + 'static + Send,
    {
        self.receive_callbacks
            .lock()
            .unwrap()
            .push(Box::new(callback));
    }

    pub fn send(&self, socket_addr: SocketAddr, packet: Packet) -> Result<(), String> {
        let bytes = bincode::serialize(&packet).unwrap();

        self.outgoing_sender
            .send((socket_addr, bytes))
            .map_err(|error| error.to_string())
    }

    pub fn start(
        &self,
        is_running: &Arc<AtomicBool>,
        app_state: &AppState,
    ) -> Result<ThreadJoiner, String> {
        let logger = Arc::clone(&self.logger);
        let bind_address = format!("{}:{}", self.bind_address, self.port);

        let receive_socket = UdpSocket::bind(&bind_address).map_err(|error| {
            format!(
                "Failed to bind to address {}: {}. Is the port already in use?",
                bind_address, error
            )
        })?;

        let send_socket = receive_socket
            .try_clone()
            .map_err(|error| format!("Failed to clone socket: {}", error))?;

        receive_socket
            .set_nonblocking(true)
            .map_err(|error| format!("Failed to set non-blocking: {}", error))?;

        logger.lock().unwrap()(format!(
            "[{}] Listening on {}",
            app_state.node_id, bind_address
        ));

        let receive_callbacks = Arc::clone(&self.receive_callbacks);

        let receive_is_running = Arc::clone(&is_running);
        let receive_thread = thread::spawn(move || {
            let mut buffer = [0; 1024];

            loop {
                match receive_socket.recv_from(&mut buffer) {
                    Ok((amt, src)) => {
                        let packet: Packet = match bincode::deserialize(&buffer[..amt]) {
                            Ok(packet) => packet,
                            Err(error) => {
                                logger.lock().unwrap()(format!(
                                    "Failed to deserialize packet: {}",
                                    error
                                ));
                                continue;
                            }
                        };

                        receive_callbacks
                            .lock()
                            .unwrap()
                            .iter_mut()
                            .for_each(|callback| {
                                if let Err(error) = callback(src, packet.clone()) {
                                    logger.lock().unwrap()(format!(
                                        "Failed to process packet: {}",
                                        error
                                    ));
                                }
                            });
                    }
                    Err(error) => {
                        if error.kind() != io::ErrorKind::WouldBlock {
                            logger.lock().unwrap()(format!("Failed to receive packet: {}", error));
                        }

                        if !receive_is_running.load(std::sync::atomic::Ordering::Relaxed) {
                            break;
                        }

                        sleep(std::time::Duration::from_millis(50));
                        continue;
                    }
                };
            }
        });

        let send_receiver = Arc::clone(&self.outgoing_receiver);
        let send_is_running = Arc::clone(&is_running);
        let send_thread = thread::spawn(move || loop {
            if !send_is_running.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }

            let (socket_addr, message) = match send_receiver.lock().unwrap().try_recv() {
                Ok(message) => message,
                Err(error) => {
                    if error == mpsc::TryRecvError::Empty {
                        sleep(std::time::Duration::from_millis(50));
                        continue;
                    } else {
                        panic!("Failed to receive outgoing message: {}", error);
                    }
                }
            };

            send_socket.send_to(&message, socket_addr).unwrap();
        });

        Ok(ThreadJoiner::new(vec![receive_thread, send_thread]))
    }
}
