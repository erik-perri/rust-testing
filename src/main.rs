use std::env;
use std::sync::{atomic::AtomicBool, Arc, Mutex};

mod arguments;
mod node_state;
mod output_buffer;
mod server;
mod terminal;

trait OutputWriter {
    fn output(&self, message: String);
}

fn main() {
    let arguments = match arguments::parse_arguments(env::args().collect()) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("\nFailed to parse arguments: {}", error);
            std::process::exit(1);
        }
    };

    let node_state = match node_state::get_state(&arguments.state_file) {
        Ok(node_state) => node_state,
        Err(error) => {
            eprintln!("\nFailed to get node state: {}", error);
            std::process::exit(1);
        }
    };

    let output_buffer = Arc::new(Mutex::new(output_buffer::OutputBuffer::new()));

    let terminal_output_buffer = Arc::clone(&output_buffer);
    let terminal = terminal::Terminal::new(terminal_output_buffer);

    let server_output_buffer = Arc::clone(&output_buffer);
    let server = server::Server::new(
        &arguments.bind_address,
        arguments.port,
        server_output_buffer,
    );

    let is_running = Arc::new(AtomicBool::new(true));

    let terminal_handle = terminal.start(&is_running);
    let _ = server.start(&is_running, &node_state).map_err(|error| {
        eprintln!("\nFailed to start server: {}", error);
        std::process::exit(1);
    });

    terminal_handle.join().unwrap();

    output_buffer
        .lock()
        .unwrap()
        .output("terminal_handle done".to_string());

    // We can't wait for this until we use a non-blocking IO library
    // server_handle.unwrap().join().unwrap();
}
