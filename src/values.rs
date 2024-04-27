use std::collections::HashMap;

pub struct ValueStore {
    values: HashMap<String, Vec<u8>>,
}

impl ValueStore {
    pub fn new(values: HashMap<String, Vec<u8>>) -> Result<Self, String> {
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

    pub fn values(&self) -> HashMap<String, Vec<u8>> {
        self.values.clone()
    }
}
