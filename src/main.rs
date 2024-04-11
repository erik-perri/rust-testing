use crate::server::Message;
use chrono::DateTime;
use std::env;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

mod app_state;
mod arguments;
mod hash;
mod peer_manager;
mod server;
mod terminal;
mod thread_joiner;

fn main() {
    let arguments = match arguments::parse_arguments(env::args().collect()) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("\nFailed to parse arguments: {}", error);
            std::process::exit(1);
        }
    };

    let app_state = match app_state::AppState::initialize_or_create(&arguments.state_file) {
        Ok(app_state) => app_state,
        Err(error) => {
            eprintln!(
                "\nFailed to load state from \"{}\", {}.",
                arguments.state_file, error
            );
            std::process::exit(1);
        }
    };

    let peer_manager = match peer_manager::PeerManager::initialize_or_create(&arguments.peer_file) {
        Ok(node_manager) => Arc::new(Mutex::new(node_manager)),
        Err(error) => {
            eprintln!(
                "\nFailed to load nodes from \"{}\", {}",
                arguments.peer_file, error
            );
            std::process::exit(1);
        }
    };

    println!(
        "Loaded {} known peers for node {}",
        peer_manager.lock().unwrap().len(),
        app_state.node_id
    );

    let mut terminal = terminal::Terminal::new(|message| {
        println!("{}", message);
    });

    let server = Arc::new(Mutex::new(server::Server::new(
        &arguments.bind_address,
        arguments.port,
        |message| {
            println!("{}", message);
        },
    )));

    let is_running = Arc::new(AtomicBool::new(true));

    //
    // Interactions
    //

    let is_running_clone = Arc::clone(&is_running);

    terminal.on_command("exit", move |_args| {
        is_running_clone.store(false, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    });

    let node_id_clone = app_state.node_id.clone();
    let server_clone = Arc::clone(&server);

    terminal.on_command("add_peer", move |args| {
        if args.len() < 2 {
            return Err("Invalid command: add_node <ip:port>".to_string());
        }

        let socket_addr = match args[1].parse::<std::net::SocketAddr>() {
            Ok(socket_addr) => socket_addr,
            Err(error) => {
                return Err(format!("Failed to parse address: {}", error));
            }
        };

        println!("Sending PING to {}", socket_addr);

        server_clone.lock().unwrap().send(
            socket_addr,
            server::Packet {
                node_id: node_id_clone.clone(),
                transaction_id: "test".to_string(),
                message: Message::Ping,
            },
        )
    });

    let peer_manager_clone = Arc::clone(&peer_manager);

    terminal.on_command("list_peers", move |_args| {
        for (_node_id, peer) in peer_manager_clone.lock().unwrap().peers_iter() {
            let last_seen = DateTime::from_timestamp(peer.last_seen as i64, 0)
                .ok_or("Invalid last seen timestamp.")?;
            let first_seen = DateTime::from_timestamp(peer.first_seen as i64, 0)
                .ok_or("Invalid first seen timestamp.")?;

            println!("[{}]", peer.node_id);
            println!("     Active: {}", peer.active);
            println!("    Address: {}:{}", peer.address, peer.port);
            println!("  Last seen: {}", last_seen.format("%Y-%m-%d %H:%M:%S"));
            println!(" First seen: {}", first_seen.format("%Y-%m-%d %H:%M:%S"));
        }

        Ok(())
    });

    let node_id_clone = app_state.node_id.clone();
    let server_clone = Arc::clone(&server);
    let peer_manager_clone = Arc::clone(&peer_manager);

    server
        .lock()
        .unwrap()
        .on_receive(move |socket_addr, packet| {
            println!("Received packet: from {}, {:?}", socket_addr, packet);

            match packet.message {
                Message::Ping => {
                    println!("Received PING from {}, sending PONG.", socket_addr);

                    peer_manager_clone
                        .lock()
                        .unwrap()
                        .add_peer(socket_addr, &packet.node_id);

                    server_clone.lock().unwrap().send(
                        socket_addr,
                        server::Packet {
                            node_id: node_id_clone.clone(),
                            transaction_id: packet.transaction_id,
                            message: Message::Pong,
                        },
                    )
                }
                Message::Pong => {
                    println!("Received PONG from {}, adding peer.", socket_addr);

                    peer_manager_clone
                        .lock()
                        .unwrap()
                        .add_peer(socket_addr, &packet.node_id);

                    Ok(())
                }
                _ => {
                    println!("Received unknown message from {}", socket_addr);
                    Ok(())
                }
            }
        });

    //
    // Start threads
    //

    let server_handles = match server.lock().unwrap().start(&is_running, &app_state) {
        Ok(handle) => handle,
        Err(error) => {
            eprintln!("\nFailed to start server: {}", error);
            std::process::exit(1);
        }
    };

    let terminal_handle = terminal.start(&is_running);

    //
    // Wait for threads
    //

    server_handles.join().unwrap();
    terminal_handle.join().unwrap();

    //
    // Save state
    //

    app_state.save_to(&arguments.state_file).unwrap();
    peer_manager
        .lock()
        .unwrap()
        .save_to(&arguments.peer_file)
        .unwrap();
}
