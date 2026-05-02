use std::sync::{Arc, Mutex};

use crate::command::Command;
use crate::storage::{Store, Aof};

#[derive(Clone)]
pub struct Engine {
    store: Arc<Mutex<Store>>,
    aof: Aof,
}

impl Engine {
    pub async fn new() -> Result<Self, std::io::Error> {
        let store = Arc::new(Mutex::new(Store::new()));
        let aof = Aof::new("appendonly.aof", store.clone()).await?;

        Ok(Self { store, aof })
    }

    pub async fn execute(&self, cmd: Command) -> String {
        // Write to AOF first for persistence
        if let Err(e) = self.aof.append(&cmd) {
            eprintln!("Failed to append to AOF: {}", e);
        }

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

    pub async fn trigger_rewrite(&self) -> Result<(), std::io::Error> {
        self.aof.rewrite(self.store.clone()).await
    }
}
