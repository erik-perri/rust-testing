use crate::structures;
use std::fs::File;

use crate::utilities::lock_file;
use crate::utilities::random_sha1_to_string;

pub fn load_node_state(path: &str) -> Result<(structures::NodeState, File), String> {
    if !std::path::Path::new(path).exists() {
        let node_state = structures::NodeState {
            node_id: random_sha1_to_string(),
        };

        save_node_state(path, &node_state)?;

        return Ok((node_state, lock_file(path)?));
    }

    let contents =
        std::fs::read(path).map_err(|error| format!("Failed to read state file: {}", error))?;

    let node_state: structures::NodeState = bincode::deserialize(&contents)
        .map_err(|error| format!("Failed to deserialize state: {}", error))?;

    Ok((node_state, lock_file(path)?))
}

pub fn save_node_state(path: &str, state: &structures::NodeState) -> Result<(), String> {
    let contents = bincode::serialize(state)
        .map_err(|error| format!("Failed to serialize state: {}", error))?;

    std::fs::write(path, contents).map_err(|error| format!("Failed to write state: {}", error))?;

    Ok(())
}
