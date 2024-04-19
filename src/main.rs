use chrono::DateTime;
use std::collections::VecDeque;
use std::env;
use std::net::SocketAddr;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::sleep;

use crate::messages::{find_nearby_peers, handle_request, send_packet, wait_for_response};
use colored::Colorize;

use crate::node_state::{load_node_state, save_node_state};
use crate::server::start_server;
use crate::utilities::random_sha1_to_string;

mod arguments;
mod messages;
mod node_state;
mod peers;
mod server;
mod structures;
mod terminal;
mod utilities;

fn main() {
    let arguments =
        arguments::parse_arguments(env::args().collect()).unwrap_or_else(|error| fatal_log(error));

    let (node_state, _node_state_lock) =
        load_node_state(&arguments.state_file).unwrap_or_else(|error| fatal_log(error));

    let peer_manager = peers::PeerManager::new(&arguments.peer_file, &node_state.node_id)
        .unwrap_or_else(|error| fatal_log(error));

    debug_log(format!("Loaded {} peers", peer_manager.to_vec().len()));

    let (receive_tx, receive_rx): (
        Sender<(SocketAddr, Vec<u8>)>,
        Receiver<(SocketAddr, Vec<u8>)>,
    ) = mpsc::channel();

    let (send_tx, send_rx): (
        Sender<(SocketAddr, Vec<u8>)>,
        Receiver<(SocketAddr, Vec<u8>)>,
    ) = mpsc::channel();

    let is_running = Arc::new(AtomicBool::new(true));
    let socket_addr: SocketAddr = format!("{}:{}", arguments.bind_address, arguments.port)
        .parse()
        .unwrap_or_else(|error| fatal_log(format!("Failed to parse address: {}", error)));

    debug_log(format!(
        "[{}] Starting server on {}",
        &node_state.node_id, socket_addr
    ));

    let is_running_clone = is_running.clone();
    let (receive_thread, send_thread) =
        start_server(socket_addr, is_running_clone, receive_tx, send_rx)
            .unwrap_or_else(|error| fatal_log(error));

    let is_running_clone = is_running.clone();
    ctrlc::set_handler(move || {
        is_running_clone.store(false, std::sync::atomic::Ordering::Relaxed);
    })
    .unwrap_or_else(|error| fatal_log(format!("Failed to set Ctrl-C handler: {}", error)));

    let response_queue: Arc<Mutex<VecDeque<structures::Packet>>> =
        Arc::new(Mutex::new(VecDeque::new()));

    let peer_manager = Arc::new(Mutex::new(peer_manager));
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
            return Err("Invalid command: add_node <ip:port>".to_string());
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

        send_packet(&packet, socket_addr, send_tx_clone.clone())?;

        wait_for_response(
            is_running_clone.clone(),
            response_queue_clone.clone(),
            &packet.transaction_id,
        )
        .map(|_| ())
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
                println!("    Address: {}:{}", peer.address, peer.port);

                if let Some(last_seen) = peer.last_seen {
                    let last_seen = DateTime::from_timestamp(last_seen as i64, 0)
                        .ok_or("Invalid last seen timestamp.")
                        .unwrap();
                    println!("  Last seen: {}", last_seen.format("%Y-%m-%d %H:%M:%S"));
                } else {
                    println!("  Last seen: Never");
                }

                let first_seen = DateTime::from_timestamp(peer.first_seen as i64, 0)
                    .ok_or("Invalid first seen timestamp.")
                    .unwrap();
                println!(" First seen: {}", first_seen.format("%Y-%m-%d %H:%M:%S"));
            });

        Ok(())
    });

    let is_running_clone = is_running.clone();
    let local_node_id = node_state.node_id.clone();
    let peer_manager_clone = peer_manager.clone();
    let response_queue_clone = response_queue.clone();
    let send_tx_clone = send_tx.clone();

    std::thread::spawn(move || {
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

    while is_running.load(std::sync::atomic::Ordering::Relaxed) {
        let peer_manager_clone = peer_manager.clone();

        receive_rx.try_iter().for_each(|(src, data)| {
            let packet: structures::Packet = match bincode::deserialize(data.as_slice()) {
                Ok(packet) => packet,
                Err(error) => {
                    error_log(format!("Failed to deserialize packet: {}", error));
                    return;
                }
            };

            let peer = match peer_manager_clone
                .lock()
                .unwrap()
                .add_peer(src, &packet.node_id, true)
            {
                Ok(peer) => peer,
                Err(error) => {
                    error_log(error);
                    return;
                }
            };

            recv_log(format!(
                "Received {:?} from peer {} ({})",
                &packet.message, &peer.node_id, &peer.address
            ));

            if let structures::Message::Response(_) = packet.message {
                let mut queue = response_queue.lock().unwrap();

                queue.push_back(packet);

                return;
            }

            let send_tx = send_tx.clone();
            let local_node_id = node_state.node_id.clone();

            handle_request(
                &local_node_id,
                &packet,
                &peer,
                peer_manager_clone.clone(),
                send_tx,
            );
        });

        sleep(std::time::Duration::from_millis(100));
    }

    debug_log("Waiting for server threads to finish".to_string());
    receive_thread.join().unwrap();
    send_thread.join().unwrap();
    // terminal_thread.join().unwrap();

    debug_log(format!("Saving node state to {}", arguments.state_file));
    save_node_state(&arguments.state_file, &node_state).unwrap_or_else(|error| fatal_log(error));

    debug_log(format!("Saving peers to {}", arguments.peer_file));
    peer_manager
        .lock()
        .unwrap()
        .save(&arguments.peer_file)
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
