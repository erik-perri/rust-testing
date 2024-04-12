use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::net::SocketAddr;

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct Peer {
    #[serde(skip_serializing, skip_deserializing)]
    pub active: bool,
    pub address: String,
    pub first_seen: u64,
    pub last_seen: u64,
    pub node_id: String,
    pub port: u16,
}

const BUCKET_SIZE: usize = 20;
const ID_BITS: usize = 160;

pub struct PeerManager {
    buckets: Vec<VecDeque<Peer>>,
    node_id_bytes: [u8; 20],
}

impl PeerManager {
    pub fn initialize_or_create(path: &str, node_id: &str) -> Result<Self, String> {
        let node_id_bytes = sha1_to_bytes(node_id).map_err(|_| "Invalid node ID")?;

        if !std::path::Path::new(path).exists() {
            return Ok(Self {
                buckets: vec![VecDeque::with_capacity(BUCKET_SIZE); ID_BITS],
                node_id_bytes,
            });
        }

        let contents = std::fs::read(path).map_err(|error| error.to_string())?;

        let buckets: Vec<VecDeque<Peer>> =
            bincode::deserialize(&contents).map_err(|error| error.to_string())?;

        Ok(Self {
            buckets,
            node_id_bytes,
        })
    }

    pub fn add_peer(&mut self, socket_addr: SocketAddr, peer_node_id: &str) -> Result<(), String> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("Unable to generate timestamp due to current time.")
            .as_secs();

        let peer_node_id_bytes = sha1_to_bytes(peer_node_id).map_err(|_| "Invalid peer node ID")?;
        let distance = distance_to_node(&self.node_id_bytes, &peer_node_id_bytes);
        let bucket_index = distance.leading_zeros() as usize;

        if self.buckets[bucket_index].len() >= BUCKET_SIZE {
            return Err("Bucket is full".to_string());
        }

        let peer_index = self.buckets[bucket_index]
            .iter()
            .position(|peer| peer.node_id == peer_node_id);

        match peer_index {
            Some(index) => {
                let peer = &mut self.buckets[bucket_index][index];
                peer.active = true;
                peer.last_seen = now;
            }
            None => {
                self.buckets[bucket_index].push_back(Peer {
                    active: true,
                    address: socket_addr.ip().to_string(),
                    first_seen: now,
                    last_seen: now,
                    node_id: peer_node_id.to_string(),
                    port: socket_addr.port(),
                });
            }
        }

        Ok(())
    }

    pub fn get_closest_peers(&self, target_id: &str) -> Result<Vec<Peer>, String> {
        let target_id_bytes = sha1_to_bytes(&target_id).map_err(|_| "Invalid target ID")?;
        let distance = distance_to_node(&self.node_id_bytes, &target_id_bytes);
        let bucket_index = distance.leading_zeros() as usize;

        let peers = self.buckets[bucket_index]
            .iter()
            .cloned()
            .collect::<Vec<Peer>>();

        let filtered_peers = peers
            .into_iter()
            .filter(|peer| peer.node_id != target_id)
            .collect();

        Ok(filtered_peers)
    }

    pub fn get_peer(&self, node_id: &str) -> Option<Peer> {
        let peer_node_id_bytes = sha1_to_bytes(node_id).ok()?;
        let distance = distance_to_node(&self.node_id_bytes, &peer_node_id_bytes);
        let bucket_index = distance.leading_zeros() as usize;

        self.buckets[bucket_index]
            .iter()
            .find(|peer| peer.node_id == node_id)
            .cloned()
    }

    pub fn for_each(&self, callback: &mut dyn FnMut(&Peer)) {
        for (_, bucket) in self.buckets.iter().enumerate() {
            for peer in bucket.iter() {
                callback(peer);
            }
        }
    }

    pub fn len(&self) -> usize {
        return self.buckets.iter().map(|bucket| bucket.len()).sum();
    }

    pub fn save_to(&self, path: &str) -> Result<(), String> {
        let contents = bincode::serialize(&self.buckets).map_err(|error| error.to_string())?;

        std::fs::write(path, contents).map_err(|error| error.to_string())?;

        Ok(())
    }
}

fn distance_to_node(node_id_a: &[u8; 20], node_id_b: &[u8; 20]) -> u32 {
    let mut distance = 0;

    for i in 0..20 {
        let xor = node_id_a[i] ^ node_id_b[i];
        if xor != 0 {
            distance = 8 * (19 - i) + xor.leading_zeros() as usize;
            break;
        }
    }

    distance as u32
}

fn sha1_to_bytes(sha1: &str) -> Result<[u8; 20], std::num::ParseIntError> {
    let mut bytes: [u8; 20] = [0; 20];

    for i in 0..20 {
        bytes[i] = u8::from_str_radix(&sha1[i * 2..i * 2 + 2], 16)?;
    }

    Ok(bytes)
}
