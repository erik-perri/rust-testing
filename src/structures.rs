use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::net::SocketAddr;

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct NodeState {
    pub buckets: Vec<VecDeque<Peer>>,
    pub node_id: String,
    pub values: HashMap<String, Vec<u8>>,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct Peer {
    #[serde(skip_serializing, skip_deserializing)]
    pub active: bool,
    pub address: SocketAddr,
    pub first_seen: u64,
    pub last_seen: Option<u64>,
    pub node_id: String,
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
    FindValue(FoundValue),
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct FoundNode {
    pub address: SocketAddr,
    pub node_id: String,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub enum FoundValue {
    Value(Vec<u8>),
    Nodes(Vec<FoundNode>),
}
