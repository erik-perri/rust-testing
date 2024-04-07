pub struct Arguments {
    pub bind_address: String,
    pub port: u16,
    pub state_file: String,
}

pub fn parse_arguments(args: Vec<String>) -> Result<Arguments, String> {
    let binary_name = args[0].clone();
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
    let mut state_file = String::from("state.toml");
    let mut bind_address = String::from("0.0.0.0");

    let mut current_index = 0;

    while current_index < args.len() {
        let arg = &args[current_index];

        match arg.as_str().trim() {
            "-b" | "--bind-address" => {
                if current_index + 1 >= args.len() {
                    return Err("No bind address provided.".to_string());
                }

                bind_address = args[current_index + 1].clone();

                current_index += 1;
            }
            "-h" | "--help" => {
                println!("Usage: {} [options]", binary_name);
                println!("\nOptions:");
                println!(
                    "  -b, --bind-address <address>  Bind address for the server. Default: 0.0.0.0"
                );
                println!("  -h, --help                    Display this help message.");
                println!("  -p, --port <port>             Port for the server to listen on. Default: 16600");
                println!("  -s, --state-file <file>       File to read and write state to. Default: state.toml");

                std::process::exit(0);
            }
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
            "-s" | "--state-file" => {
                if current_index + 1 >= args.len() {
                    return Err("No state file provided.".to_string());
                }

                state_file = args[current_index + 1].clone();

                current_index += 1;
            }
            _ => {
                return Err(format!("Invalid argument provided: \"{}\"", arg));
            }
        }

        current_index += 1;
    }

    Ok(Arguments {
        bind_address,
        port,
        state_file,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_arguments_long() {
        let args = vec![
            String::from("binary_name"),
            String::from("--bind-address=255.255.255.255"),
            String::from("--port=1234"),
            String::from("--state=override.toml"),
        ];

        let config = parse_arguments(args).unwrap();

        assert_eq!(config.bind_address, "255.255.255.255");
        assert_eq!(config.port, 1234);
        assert_eq!(config.state_file, "override.toml");
    }

    #[test]
    fn test_parse_arguments_long_with_space() {
        let args = vec![
            String::from("binary_name"),
            String::from("--bind-address"),
            String::from("255.255.255.255"),
            String::from("--port"),
            String::from("1234"),
            String::from("--state"),
            String::from("override.toml"),
        ];

        let config = parse_arguments(args).unwrap();

        assert_eq!(config.bind_address, "255.255.255.255");
        assert_eq!(config.port, 1234);
        assert_eq!(config.state_file, "override.toml");
    }

    #[test]
    fn test_parse_arguments_short() {
        let args = vec![
            String::from("binary_name"),
            String::from("-b"),
            String::from("255.255.255.255"),
            String::from("-p"),
            String::from("1234"),
            String::from("-s"),
            String::from("override.toml"),
        ];

        let config = parse_arguments(args).unwrap();

        assert_eq!(config.bind_address, "255.255.255.255");
        assert_eq!(config.port, 1234);
        assert_eq!(config.state_file, "override.toml");
    }

    #[test]
    fn test_parse_arguments_none() {
        let args = vec![String::from("binary_name")];

        let config = parse_arguments(args).unwrap();

        assert_eq!(config.bind_address, "0.0.0.0");
        assert_eq!(config.port, 16600);
        assert_eq!(config.state_file, "state.toml");
    }

    #[test]
    fn test_parse_arguments_missing_port_long() {
        let args = vec![String::from("binary_name"), String::from("--port")];

        let config = parse_arguments(args);

        assert!(config.is_err());
        assert_eq!(config.err().unwrap(), "No port number provided.");
    }

    #[test]
    fn test_parse_arguments_missing_port_short() {
        let args = vec![String::from("binary_name"), String::from("-p")];

        let config = parse_arguments(args);

        assert!(config.is_err());
        assert_eq!(config.err().unwrap(), "No port number provided.");
    }

    #[test]
    fn test_parse_arguments_invalid_port() {
        let args = vec![String::from("binary_name"), String::from("--port=invalid")];

        let config = parse_arguments(args);

        assert!(config.is_err());
        assert_eq!(
            config.err().unwrap(),
            "Invalid port number provided \"invalid\", invalid digit found in string."
        );
    }

    #[test]
    fn test_parse_arguments_invalid_argument() {
        let args = vec![String::from("binary_name"), String::from("--invalid")];

        let config = parse_arguments(args);

        assert!(config.is_err());
        assert_eq!(
            config.err().unwrap(),
            "Invalid argument provided: \"--invalid\""
        );
    }
}
