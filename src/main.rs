use chrono::DateTime;
use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::atomic::AtomicBool;
use std::sync::{mpsc, Arc, Mutex};
use std::{env, thread};

use crate::messages::{
    find_nearby_peers, process_incoming_requests, send_packet, wait_for_response,
};
use colored::Colorize;

use crate::node_state::{load_node_state, save_node_state};
use crate::server::start_server;
use crate::structures::NodeState;
use crate::utilities::random_sha1_to_string;

mod arguments;
mod messages;
mod node_state;
mod peers;
mod server;
mod structures;
mod terminal;
mod utilities;
mod values;

fn main() {
    let arguments =
        arguments::parse_arguments(env::args().collect()).unwrap_or_else(|error| fatal_log(error));

    let (node_state, _node_state_lock) =
        load_node_state(&arguments.state_file).unwrap_or_else(|error| fatal_log(error));

    let peer_manager = peers::PeerManager::new(node_state.buckets, &node_state.node_id)
        .unwrap_or_else(|error| fatal_log(error));

    debug_log(format!("Loaded {} peers", peer_manager.to_vec().len()));

    let value_store =
        values::ValueStore::new(node_state.values).unwrap_or_else(|error| fatal_log(error));

    debug_log(format!("Loaded {} values", value_store.len()));

    let (receive_tx, receive_rx): (
        mpsc::Sender<(SocketAddr, Vec<u8>)>,
        mpsc::Receiver<(SocketAddr, Vec<u8>)>,
    ) = mpsc::channel();

    let (send_tx, send_rx): (
        mpsc::Sender<(SocketAddr, Vec<u8>)>,
        mpsc::Receiver<(SocketAddr, Vec<u8>)>,
    ) = mpsc::channel();

    let is_running = Arc::new(AtomicBool::new(true));
    let socket_addr: SocketAddr = format!("{}:{}", arguments.bind_address, arguments.port)
        .parse()
        .unwrap_or_else(|error| fatal_log(format!("Failed to parse address: {}", error)));

    debug_log(format!(
        "[{}] Starting server on {}",
        &node_state.node_id, socket_addr
    ));

    let (receive_thread, send_thread) =
        start_server(socket_addr, is_running.clone(), receive_tx, send_rx)
            .unwrap_or_else(|error| fatal_log(error));

    let is_running_clone = is_running.clone();
    ctrlc::set_handler(move || {
        is_running_clone.store(false, std::sync::atomic::Ordering::Relaxed);
    })
    .unwrap_or_else(|error| fatal_log(format!("Failed to set Ctrl-C handler: {}", error)));

    let response_queue: Arc<Mutex<VecDeque<structures::Packet>>> =
        Arc::new(Mutex::new(VecDeque::new()));

    let peer_manager = Arc::new(Mutex::new(peer_manager));
    let value_store = Arc::new(Mutex::new(value_store));

    let is_running_clone = is_running.clone();

    let mut terminal = terminal::Terminal::new(|message| println!("{}", message));
    let _terminal_thread = terminal.start(is_running.clone());

    terminal.on_command("exit", move |_args| {
        is_running_clone.store(false, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    });

    let is_running_clone = is_running.clone();
    let local_node_id = node_state.node_id.clone();
    let response_queue_clone = response_queue.clone();
    let send_tx_clone = send_tx.clone();

    terminal.on_command("add_peer", move |args| {
        if args.len() < 2 {
            return Err("Usage: add_node <ip:port>".to_string());
        }

        let socket_addr = match args[1].parse::<SocketAddr>() {
            Ok(socket_addr) => socket_addr,
            Err(error) => {
                return Err(format!("Failed to parse address: {}", error));
            }
        };

        let packet = structures::Packet {
            node_id: local_node_id.clone(),
            message: structures::Message::Request(structures::Request::Ping),
            transaction_id: random_sha1_to_string(),
        };

        send_packet(&packet, &socket_addr, send_tx_clone.clone())?;

        wait_for_response(
            is_running_clone.clone(),
            response_queue_clone.clone(),
            &packet.transaction_id,
        )
        .map(|_| ())
    });

    let is_running_clone = is_running.clone();
    let local_node_id = node_state.node_id.clone();
    let peer_manager_clone = peer_manager.clone();
    let response_queue_clone = response_queue.clone();
    let send_tx_clone = send_tx.clone();
    let value_store_clone = value_store.clone();

    terminal.on_command("store_value", move |args| {
        if args.len() < 3 {
            return Err("Usage: store_value <key> <value>".to_string());
        }

        let key = args[1].clone();
        let value = args[2].clone();

        if key.len() != 40 {
            return Err("Key must be a SHA1 hash.".to_string());
        }

        value_store_clone
            .lock()
            .unwrap()
            .store(&key, value.as_bytes());

        let peers_near_value = peer_manager_clone.lock().unwrap().nearby_peers(&key)?;

        for peer in peers_near_value {
            let is_running_clone = is_running_clone.clone();
            let key = key.clone();
            let local_node_id = local_node_id.clone();
            let response_queue_clone = response_queue_clone.clone();
            let send_tx_clone = send_tx_clone.clone();
            let value = value.clone();

            thread::spawn(move || {
                let packet = structures::Packet {
                    node_id: local_node_id.clone(),
                    message: structures::Message::Request(structures::Request::Store(
                        key.clone(),
                        value.as_bytes().to_vec(),
                    )),
                    transaction_id: random_sha1_to_string(),
                };

                send_packet(&packet, &peer.address, send_tx_clone.clone())
                    .unwrap_or_else(|error| error_log(error));

                let response = wait_for_response(
                    is_running_clone.clone(),
                    response_queue_clone.clone(),
                    &packet.transaction_id,
                );

                match response {
                    Ok(_) => debug_log(format!("Stored value on {}", peer.node_id)),
                    Err(error) => debug_log(format!(
                        "Failed to store value on {}: {}",
                        peer.node_id, error
                    )),
                }
            });
        }

        Ok(())
    });

    let peer_manager_clone = peer_manager.clone();

    terminal.on_command("list_peers", move |_args| {
        peer_manager_clone
            .lock()
            .unwrap()
            .to_vec()
            .iter()
            .for_each(|peer: &structures::Peer| {
                println!("[{}]", peer.node_id);
                println!("     Active: {}", peer.active);
                println!("    Address: {}", peer.address);

                let first_seen = DateTime::from_timestamp(peer.first_seen as i64, 0)
                    .ok_or("Invalid first seen timestamp.")
                    .unwrap();
                println!(" First seen: {}", first_seen.format("%Y-%m-%d %H:%M:%S"));

                if let Some(last_seen) = peer.last_seen {
                    let last_seen = DateTime::from_timestamp(last_seen as i64, 0)
                        .ok_or("Invalid last seen timestamp.")
                        .unwrap();
                    println!("  Last seen: {}", last_seen.format("%Y-%m-%d %H:%M:%S"));
                } else {
                    println!("  Last seen: Never");
                }
            });

        Ok(())
    });

    let process_messages_thread = process_incoming_requests(
        is_running.clone(),
        node_state.node_id.clone(),
        peer_manager.clone(),
        value_store.clone(),
        response_queue.clone(),
        receive_rx,
        send_tx.clone(),
    );

    let is_running_clone = is_running.clone();
    let local_node_id = node_state.node_id.clone();
    let peer_manager_clone = peer_manager.clone();
    let response_queue_clone = response_queue.clone();
    let send_tx_clone = send_tx.clone();

    let find_peers_thread = thread::spawn(move || {
        match find_nearby_peers(
            is_running_clone,
            local_node_id.as_str(),
            peer_manager_clone,
            response_queue_clone,
            send_tx_clone,
        ) {
            Ok(_) => debug_log("Finished finding nearby peers".to_string()),
            Err(error) => error_log(format!("Failed to find nearby peers: {}", error)),
        }
    });

    debug_log("Waiting for server threads to finish".to_string());
    receive_thread.join().unwrap();
    send_thread.join().unwrap();
    // terminal_thread.join().unwrap();
    find_peers_thread.join().unwrap();
    process_messages_thread.join().unwrap();

    debug_log(format!("Saving node state to {}", arguments.state_file));
    save_node_state(
        &arguments.state_file,
        &NodeState {
            node_id: node_state.node_id,
            buckets: peer_manager.lock().unwrap().buckets(),
            values: value_store.lock().unwrap().values(),
        },
    )
    .unwrap_or_else(|error| fatal_log(error));
}

fn debug_log(message: String) {
    let readable_time = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    println!(
        "{} {} {}",
        readable_time.to_string().bright_black(),
        " DEBUG ".bold().black().on_bright_blue(),
        message
    );
}

fn error_log(message: String) {
    let readable_time = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    eprintln!(
        "{} {} {}",
        readable_time.to_string().bright_black(),
        " ERROR ".bold().black().on_red(),
        message
    );
}

fn fatal_log(message: String) -> ! {
    error_log(message);
    std::process::exit(1);
}

fn recv_log(message: String) {
    let readable_time = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");

    eprintln!(
        "{} {} {}",
        readable_time.to_string().bright_black(),
        " RECV ".bold().black().on_bright_magenta(),
        message
    );
}

fn send_log(message: String) {
    let readable_time = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");

    println!(
        "{} {} {}",
        readable_time.to_string().bright_black(),
        " SEND ".bold().black().on_bright_green(),
        message
    );
}
