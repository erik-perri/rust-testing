pub struct NodeConfig {
    pub port: u16,
}

pub fn parse_arguments(args: Vec<String>) -> Result<NodeConfig, String> {
    let args: Vec<String> = args
        .into_iter()
        // Skip past the binary name
        .skip(1)
        // Expand any arguments in the form of --port=1234 into --port 1234 for easier parsing
        .flat_map(|arg| {
            arg.trim()
                .split('=')
                .map(String::from)
                .collect::<Vec<String>>()
        })
        .collect();

    let mut port: u16 = 16600;
    let mut current_index = 0;

    while current_index < args.len() {
        let arg = &args[current_index];

        match arg.as_str().trim() {
            "-p" | "--port" => {
                if current_index + 1 >= args.len() {
                    return Err("No port number provided.".to_string());
                }

                port = match args[current_index + 1].parse() {
                    Ok(port) => port,
                    Err(error) => {
                        return Err(format!(
                            "Invalid port number provided \"{}\", {}.",
                            args[current_index + 1],
                            error.to_string()
                        ));
                    }
                };

                current_index += 1;
            }
            _ => {
                return Err(format!("Invalid argument provided: \"{}\"", arg));
            }
        }

        current_index += 1;
    }

    Ok(NodeConfig { port })
}
