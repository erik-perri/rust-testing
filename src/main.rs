use std::env;
use std::io::{self, Write};
use std::net::TcpListener;
use std::thread;

mod config;

fn output_line(input: String) {
    if !input.is_empty() {
        println!("\r{}\n", input);
    }

    print!("> ");

    io::stdout().flush().unwrap();
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let config = match config::parse_arguments(args) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("{}", error);
            std::process::exit(1);
        }
    };

    thread::spawn(move || {
        output_line(format!("Listening on port: {}", config.port));
        let listener = TcpListener::bind(format!("0.0.0.0:{}", config.port)).unwrap();

        for stream in listener.incoming() {
            let stream = stream.unwrap();

            output_line(format!(
                "Connection established to {}",
                stream.peer_addr().unwrap()
            ));
        }
    });

    loop {
        let mut input = String::new();

        io::stdin().read_line(&mut input).unwrap();

        match input.trim() {
            "exit" | "quit" | 1 => break,
            "" => output_line("".to_string()),
            _ => output_line(format!("Invalid command: {}", input.trim())),
        }
    }
}
