use crate::hash::random_sha1_to_string;
use crate::peer_manager::PeerManager;
use crate::server::{FoundNode, Message, Packet, Server};
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
        let logger = Arc::new(Mutex::new(logger));

        Self {
            logger,
            node_id: node_id.to_string(),
            peer_manager,
            server,
            transaction_ids: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn find_nearby_nodes(&self, node_id: &str) -> Result<(), String> {
        self.peer_manager.lock().unwrap().for_each(&mut |peer| {
            let peer_address = format!("{}:{}", peer.address, peer.port);
            let peer_address: SocketAddr = match peer_address.parse() {
                Ok(address) => address,
                Err(_) => {
                    self.logger.lock().unwrap()(format!(
                        "Failed to parse peer address: {}",
                        peer.address
                    ));

                    return;
                }
            };

            let transaction_id = random_sha1_to_string();

            self.transaction_ids
                .lock()
                .unwrap()
                .insert(transaction_id.clone(), peer_address);

            let send_result = self.server.lock().unwrap().send(
                peer_address,
                Packet {
                    message: Message::FindNode(node_id.to_string()),
                    node_id: self.node_id.clone(),
                    transaction_id,
                },
            );

            match send_result {
                Ok(_) => {}
                Err(error) => {
                    self.logger.lock().unwrap()(format!(
                        "Failed to send FIND_NODE to {}: {}",
                        peer.address, error
                    ));
                }
            }
        });

        Ok(())
    }

    pub fn handle_packet(&self, socket_addr: SocketAddr, packet: Packet) -> Result<(), String> {
        match packet.message.clone() {
            Message::Ping => self.handle_ping(socket_addr, &packet),
            Message::Pong => self.handle_pong(socket_addr, &packet),
            Message::FindNode(target_id) => self.handle_find_node(socket_addr, &packet, target_id),
            Message::FindNodeResponse(nodes) => {
                self.handle_find_node_response(socket_addr, &packet, nodes)
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

    fn handle_find_node(
        &self,
        socket_addr: SocketAddr,
        packet: &Packet,
        target_id: String,
    ) -> Result<(), String> {
        let closest_peers = self
            .peer_manager
            .lock()
            .unwrap()
            .get_closest_peers(&target_id)?;

        // TODO If closest peers is empty we should return the current node. This is awkward at the
        //      moment since we bind to 0.0.0.0 and don't know our external address yet.

        self.logger.lock().unwrap()(format!(
            "Sending FIND_NODE_RESPONSE {:?} (transaction: {})",
            closest_peers,
            packet.transaction_id.to_string()
        ));

        let found_nodes: Vec<FoundNode> = closest_peers
            .iter()
            .map(|peer| FoundNode {
                address: format!("{}:{}", peer.address, peer.port),
                node_id: peer.node_id.clone(),
            })
            .collect();

        self.server.lock().unwrap().send(
            socket_addr,
            Packet {
                message: Message::FindNodeResponse(found_nodes),
                node_id: self.node_id.clone(),
                transaction_id: packet.transaction_id.to_string(),
            },
        )
    }

    fn handle_find_node_response(
        &self,
        socket_addr: SocketAddr,
        packet: &Packet,
        nodes: Vec<FoundNode>,
    ) -> Result<(), String> {
        self.validate_transaction_id(socket_addr, &packet.transaction_id)?;

        let mut nodes = nodes.clone();

        if nodes.is_empty() {
            // TODO This should be done on the other side of this request.
            nodes.push(FoundNode {
                address: socket_addr.to_string(),
                node_id: packet.node_id.clone(),
            });
        }

        let mut pinged_nodes: HashMap<String, bool> = HashMap::new();

        for found_node in nodes {
            let found_node_address: SocketAddr = match found_node.address.parse() {
                Ok(address) => address,
                Err(_) => {
                    self.logger.lock().unwrap()(format!(
                        "Failed to parse found node address: {}",
                        found_node.address
                    ));

                    continue;
                }
            };

            let existing_peer = self
                .peer_manager
                .lock()
                .unwrap()
                .get_peer(&found_node.node_id);

            if existing_peer.is_some() && existing_peer.unwrap().active {
                continue;
            }

            if pinged_nodes.contains_key(&found_node.node_id) {
                continue;
            }

            pinged_nodes.insert(found_node.node_id.clone(), true);

            if let Err(error) = self.send_ping(found_node_address) {
                self.logger.lock().unwrap()(format!(
                    "Failed to send PING to {}: {}",
                    found_node.address, error
                ));
            }
        }

        Ok(())
    }

    fn handle_ping(&self, socket_addr: SocketAddr, packet: &Packet) -> Result<(), String> {
        self.logger.lock().unwrap()("Sending PONG".to_string());

        self.server.lock().unwrap().send(
            socket_addr,
            Packet {
                message: Message::Pong,
                node_id: self.node_id.clone(),
                transaction_id: packet.transaction_id.to_string(),
            },
        )?;

        self.peer_manager
            .lock()
            .unwrap()
            .add_peer(socket_addr, &packet.node_id)?;

        Ok(())
    }

    fn handle_pong(&self, socket_addr: SocketAddr, packet: &Packet) -> Result<(), String> {
        self.validate_transaction_id(socket_addr, &packet.transaction_id)?;

        self.peer_manager
            .lock()
            .unwrap()
            .add_peer(socket_addr, &packet.node_id)?;

        Ok(())
    }

    fn validate_transaction_id(
        &self,
        socket_addr: SocketAddr,
        transaction_id: &str,
    ) -> Result<(), String> {
        let expected_socket_addr = self.transaction_ids.lock().unwrap().remove(transaction_id);

        if expected_socket_addr.is_none() {
            return Err(format!("Unknown transaction ID: {}", transaction_id));
        }

        let expected_socket_addr = expected_socket_addr.unwrap();

        if expected_socket_addr != socket_addr {
            return Err(format!(
                "Unexpected address: {}, expected {} (transaction: {})",
                socket_addr, expected_socket_addr, transaction_id
            ));
        }

        Ok(())
    }
}
