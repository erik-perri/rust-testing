use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct AppState {
    pub node_id: String,
}

impl AppState {
    pub fn initialize_or_create(path: &str) -> Result<Self, String> {
        if !std::path::Path::new(path).exists() {
            return Ok(Self {
                node_id: generate_sha1(),
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

fn generate_sha1() -> String {
    let mut rng = thread_rng();
    let mut bytes: [u8; 64] = [0; 64];
    rng.fill(&mut bytes[..]);

    let mut hasher = Sha1::new();
    hasher.update(bytes);

    let sha1 = hasher.finalize();

    format!("{:x}", sha1)
}