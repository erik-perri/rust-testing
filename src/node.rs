use crate::hash::random_sha1_to_string;
use crate::peer_manager::PeerManager;
use crate::server::{Message, Packet, Server};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

pub struct Node {
    logger: Arc<Mutex<fn(String)>>,
    node_id: String,
    peer_manager: Arc<Mutex<PeerManager>>,
    server: Arc<Mutex<Server>>,
    transaction_ids: Arc<Mutex<HashMap<String, SocketAddr>>>,
}

impl Node {
    pub fn new(
        node_id: &str,
        peer_manager: Arc<Mutex<PeerManager>>,
        server: Arc<Mutex<Server>>,
        logger: fn(String),
    ) -> Self {
        Self {
            node_id: node_id.to_string(),
            peer_manager,
            server,
            transaction_ids: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn handle_packet(&self, socket_addr: SocketAddr, packet: Packet) -> Result<(), String> {
        match packet.message {
            Message::Ping => {
                self.send_pong(socket_addr, &packet.transaction_id)?;

                self.peer_manager
                    .lock()
                    .unwrap()
                    .add_peer(socket_addr, &packet.node_id);

                Ok(())
            }
            Message::Pong => {
                self.logger.lock().unwrap()(format!(
                    "Received PONG from {} (transaction: {})",
                    socket_addr, packet.transaction_id
                ));


                self.receive_pong(socket_addr, packet.transaction_id)?;

                self.peer_manager
                    .lock()
                    .unwrap()
                    .add_peer(socket_addr, &packet.node_id);

                Ok(())
            }
            _ => {
                self.logger.lock().unwrap()(format!(
                    "Received an unknown message from {}: {:?}",
                    socket_addr, packet
                ));

                Ok(())
            }
        }
    }

    pub fn send_ping(&self, socket_addr: SocketAddr) -> Result<(), String> {
        let transaction_id = random_sha1_to_string();

        self.transaction_ids
            .lock()
            .unwrap()
            .insert(transaction_id.clone(), socket_addr);

        self.logger.lock().unwrap()(format!(
            "Sending PING to {} (transaction {})",
            socket_addr, transaction_id
        ));

        self.server.lock().unwrap().send(
            socket_addr,
            Packet {
                message: Message::Ping,
                node_id: self.node_id.clone(),
                transaction_id,
            },
        )
    }

    fn send_pong(&self, socket_addr: SocketAddr, ping_transaction_id: &str) -> Result<(), String> {
        self.logger.lock().unwrap()(format!(
            "Received PING from {}, sending PONG (transaction: {})",
            socket_addr, packet.transaction_id
        ));

        self.server.lock().unwrap().send(
            socket_addr,
            Packet {
                message: Message::Pong,
                node_id: self.node_id.clone(),
                transaction_id: ping_transaction_id.to_string(),
            },
        )
    }

    fn receive_pong(&self, socket_addr: SocketAddr, transaction_id: String) -> Result<(), String> {
        let expected_socket_addr = self.transaction_ids.lock().unwrap().remove(&transaction_id);

        if expected_socket_addr.is_none() {
            return Err(format!(
                "Received a PONG with an unknown transaction ID: {}",
                transaction_id
            ));
        }

        let expected_socket_addr = expected_socket_addr.unwrap();

        if expected_socket_addr != socket_addr {
            return Err(format!(
                "Received a PONG from an unexpected address: {} (transaction: {})",
                socket_addr, transaction_id
            ));
        }

        return Ok(());
    }
}
