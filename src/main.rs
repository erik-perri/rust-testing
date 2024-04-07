use std::env;
use std::net::TcpListener;
use std::sync::{atomic::AtomicBool, Arc};
use std::thread;

mod config;
mod node_state;
mod terminal;

fn main() {
    const NODE_STATE_FILE: &str = "state.toml";

    let node_state = match node_state::get_state(NODE_STATE_FILE) {
        Ok(node_state) => node_state,
        Err(error) => {
            eprintln!("{}", error);
            std::process::exit(1);
        }
    };

    let args: Vec<String> = env::args().collect();
    let config = match config::parse_arguments(args) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("{}", error);
            std::process::exit(1);
        }
    };

    let is_running = Arc::new(AtomicBool::new(true));

    let terminal = terminal::Terminal::new();
    let terminal_handle = terminal.start(&is_running);

    terminal.listen_for_commands(&is_running);

    thread::spawn(move || {
        let bind_address = format!("0.0.0.0:{}", config.port);

        terminal.output(format!(
            "Node [{}] listening on {}",
            node_state.node_id, bind_address,
        ));

        let listener = TcpListener::bind(bind_address).unwrap();

        for stream in listener.incoming() {
            let stream = stream.unwrap();

            terminal.output(format!(
                "Connection established to {}",
                stream.peer_addr().unwrap()
            ));
        }
    });

    terminal_handle.join().unwrap();
}
