use crate::peers::PeerManager;
use crate::utilities::random_sha1_to_string;
use crate::{error_log, send_log, structures};
use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

pub fn handle_request(
    local_node_id: &str,
    packet: &structures::Packet,
    peer: &structures::Peer,
    peer_manager: Arc<Mutex<PeerManager>>,
    send_tx: Sender<(SocketAddr, Vec<u8>)>,
) {
    let message = match &packet.message {
        structures::Message::Request(request) => request,
        _ => {
            error_log("Received response when expecting request".to_string());
            return;
        }
    };

    match message {
        structures::Request::Ping => {
            let response = structures::Packet {
                node_id: local_node_id.to_string(),
                transaction_id: packet.transaction_id.clone(),
                message: structures::Message::Response(structures::Response::Pong),
            };

            send_packet_to_peer(&response, &peer, send_tx).unwrap();
        }
        structures::Request::FindNode(node_id) => {
            let nodes = peer_manager.lock().unwrap().nearby_peers(node_id);

            if let Err(error) = nodes {
                error_log(error);
                return;
            }

            let nodes = nodes
                .unwrap()
                .iter()
                .map(|peer| structures::FoundNode {
                    address: format!("{}:{}", peer.address, peer.port),
                    node_id: peer.node_id.clone(),
                })
                .collect();

            let response = structures::Packet {
                node_id: local_node_id.to_string(),
                transaction_id: packet.transaction_id.clone(),
                message: structures::Message::Response(structures::Response::FindNode(nodes)),
            };

            send_packet_to_peer(&response, &peer, send_tx).unwrap();
        }
        _ => {
            error_log("Received unhandled request".to_string());
        }
    }
}

pub fn find_nearby_peers(
    is_running: Arc<AtomicBool>,
    local_node_id: &str,
    peer_manager: Arc<Mutex<PeerManager>>,
    response_queue: Arc<Mutex<VecDeque<structures::Packet>>>,
    send_tx: Sender<(SocketAddr, Vec<u8>)>,
) -> Result<(), String> {
    let nearby_peers = peer_manager.lock().unwrap().nearby_peers(local_node_id)?;

    for peer in nearby_peers {
        let packet = structures::Packet {
            node_id: local_node_id.to_string(),
            message: structures::Message::Request(structures::Request::FindNode(
                local_node_id.to_string(),
            )),
            transaction_id: random_sha1_to_string(),
        };

        send_packet_to_peer(&packet, &peer, send_tx.clone())?;

        let response = match wait_for_response(
            is_running.clone(),
            response_queue.clone(),
            &packet.transaction_id,
        ) {
            Ok(response) => response,
            Err(_) => {
                continue;
            }
        };

        let nodes = match response.message {
            structures::Message::Response(structures::Response::FindNode(nodes)) => nodes,
            _ => {
                error_log("Received unexpected response".to_string());
                continue;
            }
        };

        for node in nodes {
            let peer_socket_addr: SocketAddr = node.address.parse().map_err(|error| {
                format!("Failed to parse peer address {}: {}", node.address, error)
            })?;

            let peer =
                peer_manager
                    .lock()
                    .unwrap()
                    .add_peer(peer_socket_addr, &node.node_id, false)?;

            let ping_packet = structures::Packet {
                node_id: local_node_id.to_string(),
                message: structures::Message::Request(structures::Request::Ping),
                transaction_id: random_sha1_to_string(),
            };

            send_packet_to_peer(&ping_packet, &peer, send_tx.clone())?;

            let _ = wait_for_response(
                is_running.clone(),
                response_queue.clone(),
                &ping_packet.transaction_id,
            );
        }
    }

    Ok(())
}

pub fn send_packet_to_peer(
    packet: &structures::Packet,
    peer: &structures::Peer,
    send_tx: Sender<(SocketAddr, Vec<u8>)>,
) -> Result<(), String> {
    let socket_addr: SocketAddr = format!("{}:{}", peer.address, peer.port)
        .parse()
        .map_err(|error| format!("Failed to parse peer address: {}", error))?;

    send_packet(packet, socket_addr, send_tx)
}

pub fn send_packet(
    packet: &structures::Packet,
    socket_addr: SocketAddr,
    send_tx: Sender<(SocketAddr, Vec<u8>)>,
) -> Result<(), String> {
    let data = bincode::serialize(&packet).map_err(|error| {
        format!(
            "Failed to serialize packet for peer {}: {}",
            socket_addr, error
        )
    })?;

    send_log(format!(
        "Sending {:?} to peer {}",
        &packet.message, &socket_addr
    ));

    send_tx
        .send((socket_addr, data))
        .map_err(|error| format!("Failed to send packet to peer: {}", error))?;

    Ok(())
}

pub fn wait_for_response(
    is_running: Arc<AtomicBool>,
    response_queue: Arc<Mutex<VecDeque<structures::Packet>>>,
    transaction_id: &str,
) -> Result<structures::Packet, String> {
    let end_time = std::time::SystemTime::now() + std::time::Duration::from_secs(5);

    while is_running.load(std::sync::atomic::Ordering::Relaxed) {
        let mut queue = response_queue.lock().unwrap();

        if let Some(val) = queue.pop_front() {
            if val.transaction_id == *transaction_id {
                return Ok(val);
            }

            queue.push_back(val);
        }

        drop(queue);

        std::thread::sleep(std::time::Duration::from_millis(50));

        if std::time::SystemTime::now() > end_time {
            return Err(format!(
                "Timed out waiting for response to transaction: {}",
                transaction_id
            ));
        }
    }

    Err(format!(
        "Aborting wait for response to transaction: {}",
        transaction_id
    ))
}
