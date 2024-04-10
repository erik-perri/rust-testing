use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

    pub fn add_peer(&mut self, node: Peer) {
        let node_id = node.node_id.clone();

        self.peers.entry(node_id).or_insert(node).active = true;
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
