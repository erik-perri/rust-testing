use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct Peer {
    #[serde(skip_serializing, skip_deserializing)]
    pub active: bool,
    pub address: String,
    pub first_seen: u64,
    pub last_seen: u64,
    pub node_id: String,
    pub port: u16,
}

pub struct PeerManager {
    peers: HashMap<String, Peer>,
}

impl PeerManager {
    pub fn initialize_or_create(path: &str) -> Result<Self, String> {
        if !std::path::Path::new(path).exists() {
            return Ok(Self {
                peers: HashMap::new(),
            });
        }

        let contents = std::fs::read(path).map_err(|error| error.to_string())?;

        let nodes: HashMap<String, Peer> =
            bincode::deserialize(&contents).map_err(|error| error.to_string())?;

        Ok(Self { peers: nodes })
    }

    pub fn add_peer(&mut self, socket_addr: SocketAddr, node_id: &str) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("Unable to generate timestamp due to current time.")
            .as_secs();

        let peer = self.peers.entry(node_id.to_string()).or_insert(Peer {
            active: true,
            address: socket_addr.ip().to_string(),
            first_seen: now,
            last_seen: now,
            node_id: node_id.to_string(),
            port: socket_addr.port(),
        });

        peer.active = true;
        peer.last_seen = now;
    }

    pub fn peers_iter(&self) -> std::collections::hash_map::Iter<String, Peer> {
        return self.peers.iter();
    }

    pub fn len(&self) -> usize {
        return self.peers.len();
    }

    pub fn save_to(&self, path: &str) -> Result<(), String> {
        let contents = bincode::serialize(&self.peers).map_err(|error| error.to_string())?;

        std::fs::write(path, contents).map_err(|error| error.to_string())?;

        Ok(())
    }
}
