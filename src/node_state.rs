use rand::{thread_rng, Rng};
use sha1::{Digest, Sha1};
use std::path::Path;
use std::str;
use toml::Table;

pub struct NodePeer {
    pub address: String,
    pub last_seen: u64,
    pub node_id: String,
    pub port: u16,
}

pub struct NodeState {
    pub node_id: String,
    pub peers: Vec<NodePeer>,
}

pub fn get_state(state_file: &str) -> Result<NodeState, String> {
    if Path::new(state_file).exists() {
        return load_state_from_file(state_file);
    }

    save_state_to_file(
        state_file,
        NodeState {
            node_id: generate_sha1(),
            peers: vec![],
        },
    )
}

fn load_state_from_file(file: &str) -> Result<NodeState, String> {
    let contents = match std::fs::read_to_string(file) {
        Ok(contents) => contents,
        Err(error) => {
            return Err(error.to_string());
        }
    };

    deserialize_state(&contents)
}

fn save_state_to_file(file: &str, state: NodeState) -> Result<NodeState, String> {
    let toml = serialize_state(&state)?;

    std::fs::write(file, toml).map_err(|err| err.to_string())?;

    Ok(state)
}

fn deserialize_state(contents: &str) -> Result<NodeState, String> {
    let root_table: Table = toml::from_str(&contents).map_err(|error| error.to_string())?;

    let node_table = root_table["node"]
        .as_table()
        .ok_or("No node table found in state file.")?;

    let peers_table = root_table["peers"]
        .as_table()
        .ok_or("No peers table found in state file.")?;

    let node_id = node_table["node_id"]
        .as_str()
        .ok_or("No node_id found in state file.")?;

    let mut peers = Vec::new();

    for (_, peer) in peers_table.iter() {
        let peer_table = peer
            .as_table()
            .ok_or("No peer table found in state file.")?;

        let peer_last_seen = peer_table["last_seen"]
            .as_integer()
            .ok_or("No peer last_seen found in state file.")? as u64;

        let peer_node_id = peer_table["node_id"]
            .as_str()
            .ok_or("No peer node_id found in state file.")?;

        let peer_address = peer_table["address"]
            .as_str()
            .ok_or("No peer address found in state file.")?;

        let peer_port = peer_table["port"]
            .as_integer()
            .ok_or("No peer port found in state file.")? as u16;

        peers.push(NodePeer {
            address: peer_address.to_string(),
            last_seen: peer_last_seen,
            node_id: peer_node_id.to_string(),
            port: peer_port,
        });
    }

    Ok(NodeState {
        node_id: node_id.to_string(),
        peers,
    })
}

fn serialize_state(state: &NodeState) -> Result<String, String> {
    let mut root_table = Table::new();
    let mut node_table = Table::new();

    node_table.insert(
        "node_id".to_string(),
        toml::Value::String(state.node_id.clone()),
    );

    let mut peers_table = Table::new();

    for (_, peer) in state.peers.iter().enumerate() {
        let mut peer_table = Table::new();

        peer_table.insert(
            "node_id".to_string(),
            toml::Value::String(peer.node_id.clone()),
        );

        let peer_address: String = match peer.address.parse() {
            Ok(address) => address,
            Err(error) => {
                return Err(format!("Failed to parse address: {}", error));
            }
        };

        peer_table.insert("address".to_string(), toml::Value::String(peer_address));

        peer_table.insert(
            "last_seen".to_string(),
            toml::Value::Integer(peer.last_seen as i64),
        );

        let peer_port: u16 = match peer.port.to_string().parse() {
            Ok(port) => port,
            Err(error) => {
                return Err(format!("Failed to parse port: {}", error));
            }
        };

        peer_table.insert("port".to_string(), toml::Value::Integer(peer_port.into()));

        peers_table.insert(peer.node_id.clone(), toml::Value::Table(peer_table));
    }

    root_table.insert("node".to_string(), toml::Value::Table(node_table));
    root_table.insert("peers".to_string(), toml::Value::Table(peers_table));

    let toml = toml::to_string(&root_table).map_err(|error| error.to_string())?;

    Ok(toml)
}

fn generate_sha1() -> String {
    let mut rng = thread_rng();
    let mut bytes: [u8; 64] = [0; 64];
    rng.fill(&mut bytes[..]);

    let mut hasher = Sha1::new();
    hasher.update(bytes);

    let sha1 = hasher.finalize();

    format!("{:x}", sha1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_sha1() {
        let node_id = generate_sha1();

        assert_eq!(40, node_id.len());
    }

    #[test]
    fn test_deserialize_state() {
        let toml = "[node]\nnode_id = \"node_id\"\n\n[peers.peer_a]\naddress = \"127.0.0.1\"\nlast_seen = 1\nnode_id = \"peer_a\"\nport = 9091\n\n[peers.peer_b]\naddress = \"127.0.0.1\"\nlast_seen = 2\nnode_id = \"peer_b\"\nport = 9092\n";

        let state = deserialize_state(toml).unwrap();

        assert_eq!(state.node_id, "node_id");
        assert_eq!(2, state.peers.len());
        assert_eq!(state.peers[0].address, "127.0.0.1");
        assert_eq!(state.peers[0].last_seen, 1);
        assert_eq!(state.peers[0].node_id, "peer_a");
        assert_eq!(state.peers[0].port, 9091);
        assert_eq!(state.peers[1].address, "127.0.0.1");
        assert_eq!(state.peers[1].last_seen, 2);
        assert_eq!(state.peers[1].node_id, "peer_b");
        assert_eq!(state.peers[1].port, 9092);
    }

    #[test]
    fn test_serialize_state() {
        let state = NodeState {
            node_id: "node_id".to_string(),
            peers: vec![
                NodePeer {
                    address: "127.0.0.1".to_string(),
                    last_seen: 1,
                    node_id: "peer_a".to_string(),
                    port: 9091,
                },
                NodePeer {
                    address: "127.0.0.1".to_string(),
                    last_seen: 2,
                    node_id: "peer_b".to_string(),
                    port: 9092,
                },
            ],
        };

        let toml = serialize_state(&state).unwrap();

        assert_eq!(
            "[node]\nnode_id = \"node_id\"\n\n[peers.peer_a]\naddress = \"127.0.0.1\"\nlast_seen = 1\nnode_id = \"peer_a\"\nport = 9091\n\n[peers.peer_b]\naddress = \"127.0.0.1\"\nlast_seen = 2\nnode_id = \"peer_b\"\nport = 9092\n",
            toml,
        );
    }
}
