use std::collections::HashMap;

pub struct ValueStore {
    values: HashMap<String, Vec<u8>>,
}

impl ValueStore {
    pub fn new(path: &str) -> Result<Self, String> {
        let values: HashMap<String, Vec<u8>>;

        if !std::path::Path::new(path).exists() {
            values = HashMap::new();

            ValueStore::save_values(path, &values)?;
        } else {
            let contents = std::fs::read(path)
                .map_err(|error| format!("Failed to read values file: {}", error))?;

            values = bincode::deserialize(&contents)
                .map_err(|error| format!("Failed to deserialize values: {}", error))?;
        }

        Ok(Self { values })
    }

    pub fn store(&mut self, key: &str, value: &[u8]) {
        self.values.insert(key.to_string(), value.to_vec());
    }

    pub fn retrieve(&self, key: &str) -> Option<&Vec<u8>> {
        self.values.get(key)
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn save(&self, path: &str) -> Result<(), String> {
        ValueStore::save_values(path, &self.values)
    }

    pub fn save_values(path: &str, values: &HashMap<String, Vec<u8>>) -> Result<(), String> {
        let contents = bincode::serialize(values)
            .map_err(|error| format!("Failed to serialize values: {}", error))?;

        std::fs::write(path, contents)
            .map_err(|error| format!("Failed to write values: {}", error))?;

        Ok(())
    }
}
