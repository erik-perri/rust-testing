use crate::hash::random_sha1_to_string;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct AppState {
    pub node_id: String,
}

impl AppState {
    pub fn initialize_or_create(path: &str) -> Result<Self, String> {
        if !std::path::Path::new(path).exists() {
            return Ok(Self {
                node_id: random_sha1_to_string(),
            });
        }

        let contents = std::fs::read(path).map_err(|error| error.to_string())?;

        let state: AppState = bincode::deserialize(&contents).map_err(|error| error.to_string())?;

        Ok(state)
    }

    pub fn save_to(&self, path: &str) -> Result<(), String> {
        let contents = bincode::serialize(&self).map_err(|error| error.to_string())?;

        std::fs::write(path, contents).map_err(|error| error.to_string())?;

        Ok(())
    }
}
