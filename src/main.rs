use std::env;
use std::net::TcpListener;
use std::sync::{atomic::AtomicBool, Arc};
use std::thread;

mod arguments;
mod node_state;
mod terminal;

fn main() {
    let arguments = match arguments::parse_arguments(env::args().collect()) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("Failed to parse arguments: {}", error);
            std::process::exit(1);
        }
    };

    let node_state = match node_state::get_state(&arguments.state_file) {
        Ok(node_state) => node_state,
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
        let bind_address = format!("0.0.0.0:{}", arguments.port);

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
