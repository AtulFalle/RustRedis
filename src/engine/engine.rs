use std::sync::{Arc, Mutex};

use crate::command::Command;
use crate::storage::Store;

#[derive(Clone)]
pub struct Engine {
    store: Arc<Mutex<Store>>,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            store: Arc::new(Mutex::new(Store::new())),
        }
    }

    pub async fn execute(&self, cmd: Command) -> String {
        match cmd {
            Command::Set { key, value } => {
                let mut store = self.store.lock().unwrap();
                store.set(key, value, None);
                "OK".to_string()
            }

            Command::Get { key } => {
                let mut store = self.store.lock().unwrap();
                match store.get(&key) {
                    Some(val) => String::from_utf8_lossy(&val).to_string(),
                    None => "(nil)".to_string(),
                }
            }
        }
    }

    pub fn cleanup(&self) {
        let mut store = self.store.lock().unwrap();
        store.cleanup_expired();
    }
}
