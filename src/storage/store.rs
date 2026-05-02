use std::collections::HashMap;
use std::time::Instant;

#[derive(Clone)]
pub struct Value {
    pub data: Vec<u8>,
    pub expires_at: Option<Instant>,
}

pub struct Store {
    data: HashMap<String, Value>,
}

impl Store {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn set(&mut self, key: String, value: Vec<u8>, ttl: Option<u64>) {
        let expires_at = ttl.map(|secs| Instant::now() + std::time::Duration::from_secs(secs));

        self.data.insert(
            key,
            Value {
                data: value,
                expires_at,
            },
        );
    }

    pub fn get(&mut self, key: &str) -> Option<Vec<u8>> {
        if let Some(val) = self.data.get(key) {
            // check expiration
            if let Some(expiry) = val.expires_at {
                if Instant::now() > expiry {
                    self.data.remove(key);
                    return None;
                }
            }
            return Some(val.data.clone());
        }
        None
    }

    pub fn cleanup_expired(&mut self) {
        let now = Instant::now();
        self.data.retain(|_, v| {
            match v.expires_at {
                Some(expiry) => expiry > now,
                None => true,
            }
        });
    }

    pub fn get_snapshot(&self) -> Vec<(String, Value)> {
        self.data.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    }
}