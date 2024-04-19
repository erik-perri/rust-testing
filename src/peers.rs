use crate::structures;
use std::cmp::{max, min};
use std::collections::{HashMap, VecDeque};
use std::fs::File;
use std::net::SocketAddr;

use crate::utilities::{calculate_xor_distance, lock_file};

const BUCKET_SIZE: usize = 20;
const ID_BITS: usize = 160;
const FIND_PEER_COUNT: usize = 20;

pub struct PeerManager {
    _state_lock: File,
    buckets: Vec<VecDeque<structures::Peer>>,
    local_node_id: String,
}

impl PeerManager {
    pub fn new(path: &str, local_node_id: &str) -> Result<Self, String> {
        let buckets: Vec<VecDeque<structures::Peer>>;

        if !std::path::Path::new(path).exists() {
            buckets = vec![VecDeque::with_capacity(BUCKET_SIZE); ID_BITS];

            PeerManager::save_buckets(path, &buckets)?;
        } else {
            let contents = std::fs::read(path)
                .map_err(|error| format!("Failed to read peer file: {}", error))?;

            buckets = bincode::deserialize(&contents)
                .map_err(|error| format!("Failed to deserialize peers: {}", error))?;
        }

        Ok(Self {
            _state_lock: lock_file(path)?,
            buckets,
            local_node_id: local_node_id.to_string(),
        })
    }

    pub fn add_peer(
        &mut self,
        socket_addr: SocketAddr,
        peer_node_id: &str,
        active: bool,
    ) -> Result<structures::Peer, String> {
        let distance = calculate_xor_distance(&self.local_node_id, peer_node_id)
            .map_err(|error| format!("Failed to calculate distance: {}", error))?;
        let bucket_index = distance.leading_zeros() as usize;

        if self.buckets[bucket_index].len() >= BUCKET_SIZE {
            return Err("Bucket is full".to_string());
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("Unable to generate timestamp due to current time.")
            .as_secs();

        let peer_index = self.buckets[bucket_index]
            .iter()
            .position(|peer| peer.node_id == peer_node_id);

        let peer = match peer_index {
            Some(index) => {
                let peer = &mut self.buckets[bucket_index][index];

                peer.active = active;
                if active {
                    peer.last_seen = Some(now);
                }

                peer.clone()
            }
            None => {
                let last_seen;

                if active {
                    last_seen = Some(now);
                } else {
                    last_seen = None;
                }

                let peer = structures::Peer {
                    active,
                    address: socket_addr.ip().to_string(),
                    first_seen: now,
                    last_seen,
                    node_id: peer_node_id.to_string(),
                    port: socket_addr.port(),
                };

                self.buckets[bucket_index].push_back(peer.clone());

                peer
            }
        };

        Ok(peer)
    }

    pub fn nearby_peers(&self, target_node_id: &str) -> Result<Vec<structures::Peer>, String> {
        let mut peers: HashMap<String, structures::Peer> = self
            .find_nearby_peers(target_node_id, false)?
            .into_iter()
            .map(|peer| (peer.node_id.clone(), peer))
            .collect();

        // If we don't have enough seen peers, include unseen peers
        if peers.len() < FIND_PEER_COUNT {
            let unseen_peers = self.find_nearby_peers(target_node_id, true)?;

            for peer in unseen_peers {
                if peers.contains_key(&peer.node_id) {
                    continue;
                }

                peers.insert(peer.node_id.clone(), peer);

                if peers.len() >= FIND_PEER_COUNT {
                    break;
                }
            }
        }

        Ok(peers.into_iter().map(|(_, peer)| peer).collect())
    }

    pub fn save(&self, path: &str) -> Result<(), String> {
        PeerManager::save_buckets(path, &self.buckets)?;

        Ok(())
    }

    pub fn to_vec(&self) -> Vec<structures::Peer> {
        self.buckets
            .iter()
            .flat_map(|bucket| bucket.iter())
            .cloned()
            .collect()
    }

    fn find_nearby_peers(
        &self,
        target_node_id: &str,
        include_unseen: bool,
    ) -> Result<Vec<structures::Peer>, String> {
        let mut peers: HashMap<String, structures::Peer> = HashMap::new();

        let target_distance = calculate_xor_distance(&self.local_node_id, target_node_id)
            .map_err(|error| format!("Failed to calculate distance: {}", error))?;
        let target_bucket_index = target_distance.leading_zeros() as usize;

        for offset in 0..ID_BITS {
            // Check bucket at distance + offset
            if let Ok(nodes) = self.find_peers_at_offset(
                target_node_id,
                target_bucket_index + offset,
                include_unseen,
            ) {
                for node in nodes {
                    if node.node_id == target_node_id {
                        continue;
                    }

                    if peers.contains_key(&node.node_id) {
                        continue;
                    }

                    peers.insert(node.node_id.clone(), node);

                    if peers.len() >= FIND_PEER_COUNT {
                        return Ok(peers.into_iter().map(|(_, peer)| peer).collect());
                    }
                }
            }

            // Check bucket at distance - offset
            if offset > 0 && offset <= target_bucket_index {
                if let Ok(nodes) = self.find_peers_at_offset(
                    target_node_id,
                    target_bucket_index - offset,
                    include_unseen,
                ) {
                    for node in nodes {
                        if node.node_id == target_node_id {
                            continue;
                        }

                        if peers.contains_key(&node.node_id) {
                            continue;
                        }

                        peers.insert(node.node_id.clone(), node);

                        if peers.len() >= FIND_PEER_COUNT {
                            return Ok(peers.into_iter().map(|(_, peer)| peer).collect());
                        }
                    }
                }
            }
        }

        Ok(peers.into_iter().map(|(_, peer)| peer).collect())
    }

    fn find_peers_at_offset(
        &self,
        target_node_id: &str,
        distance_offset: usize,
        include_unseen: bool,
    ) -> Result<Vec<structures::Peer>, String> {
        let distance = calculate_xor_distance(&self.local_node_id, target_node_id)
            .map_err(|error| format!("Failed to calculate distance: {}", error))?;

        let distance = distance + distance_offset as u32;
        let distance = max(distance, 0);
        let distance = min(distance, ID_BITS as u32);

        let bucket_index = distance.leading_zeros() as usize;

        let mut peers: Vec<structures::Peer> = self.buckets[bucket_index]
            .iter()
            .cloned()
            .collect::<Vec<structures::Peer>>();

        if !include_unseen {
            peers = peers
                .into_iter()
                .filter(|peer| peer.last_seen != None)
                .collect();
        }

        peers.sort_by(|a, b| b.last_seen.cmp(&a.last_seen));

        Ok(peers)
    }

    fn save_buckets(path: &str, buckets: &Vec<VecDeque<structures::Peer>>) -> Result<(), String> {
        let contents = bincode::serialize(buckets)
            .map_err(|error| format!("Failed to serialize peers: {}", error))?;

        std::fs::write(path, contents)
            .map_err(|error| format!("Failed to write peers: {}", error))?;

        Ok(())
    }
}
