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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_arguments_long() {
        let args = vec![String::from("binary_name"), String::from("--port=1234")];

        let config = parse_arguments(args).unwrap();

        assert_eq!(config.port, 1234);
    }

    #[test]
    fn test_parse_arguments_long_with_space() {
        let args = vec![
            String::from("binary_name"),
            String::from("--port"),
            String::from("1234"),
        ];

        let config = parse_arguments(args).unwrap();

        assert_eq!(config.port, 1234);
    }

    #[test]
    fn test_parse_arguments_short() {
        let args = vec![
            String::from("binary_name"),
            String::from("-p"),
            String::from("1234"),
        ];

        let config = parse_arguments(args).unwrap();

        assert_eq!(config.port, 1234);
    }

    #[test]
    fn test_parse_arguments_no_port() {
        let args = vec![String::from("binary_name")];

        let config = parse_arguments(args).unwrap();

        assert_eq!(config.port, 16600);
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
