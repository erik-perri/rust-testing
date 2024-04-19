use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct NodeState {
    pub node_id: String,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct Peer {
    #[serde(skip_serializing, skip_deserializing)]
    pub active: bool,
    pub address: String,
    pub first_seen: u64,
    pub last_seen: Option<u64>,
    pub node_id: String,
    pub port: u16,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct Packet {
    pub message: Message,
    pub node_id: String,
    pub transaction_id: String,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub enum Message {
    Request(Request),
    Response(Response),
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub enum Request {
    Ping,
    Store(String, Vec<u8>),
    FindNode(String),
    FindValue(String),
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub enum Response {
    Pong,
    Store,
    FindNode(Vec<FoundNode>),
    FindValue(Vec<u8>),
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct FoundNode {
    pub address: String,
    pub node_id: String,
}
