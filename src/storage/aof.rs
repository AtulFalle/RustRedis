use crate::command::Command;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::fs::{File, OpenOptions};
use tokio::io::AsyncWriteExt;

use crate::storage::Store;

pub struct AofState {
    pub buffer: Vec<u8>,
    pub file: Option<File>,
    pub rewrite_buffer: Option<Vec<u8>>,
}

#[derive(Clone)]
pub struct Aof {
    state: Arc<Mutex<AofState>>,
    path: String,
}

impl Aof {
    pub async fn new(path: &str, store: Arc<Mutex<Store>>) -> Result<Self, std::io::Error> {
        if let Ok(contents) = tokio::fs::read(path).await {
            Self::restore(&contents, &store);
        }

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await?;

        let state = Arc::new(Mutex::new(AofState {
            buffer: Vec::new(),
            file: Some(file),
            rewrite_buffer: None,
        }));

        let aof = Self {
            state,
            path: path.to_string(),
        };

        aof.start_background_task();

        Ok(aof)
    }

    fn restore(data: &[u8], store: &Arc<Mutex<Store>>) {
        let mut i = 0;
        while i < data.len() {
            let Some(parts) = Self::parse_set_record(data, &mut i) else {
                break;
            };
            let key = String::from_utf8_lossy(parts[1]).to_string();
            let value = parts[2].to_vec();

            let mut s = store.lock().unwrap();
            s.set(key, value, None);
        }
    }

    fn parse_set_record<'a>(data: &'a [u8], i: &mut usize) -> Option<Vec<&'a [u8]>> {
        if !data.get(*i..)?.starts_with(b"*3\r\n") {
            return None;
        }
        *i += 4;

        let mut parts = Vec::with_capacity(3);
        for _ in 0..3 {
            parts.push(Self::parse_bulk(data, i)?);
        }

        if parts[0].eq_ignore_ascii_case(b"SET") {
            Some(parts)
        } else {
            None
        }
    }

    fn parse_bulk<'a>(data: &'a [u8], i: &mut usize) -> Option<&'a [u8]> {
        if *data.get(*i)? != b'$' {
            return None;
        }
        *i += 1;

        let mut len = 0usize;
        let mut saw_digit = false;
        while let Some(&byte) = data.get(*i) {
            if byte == b'\r' {
                break;
            }
            if !byte.is_ascii_digit() {
                return None;
            }
            saw_digit = true;
            len = len.checked_mul(10)?.checked_add((byte - b'0') as usize)?;
            *i += 1;
        }

        if !saw_digit || data.get(*i..*i + 2)? != b"\r\n" {
            return None;
        }
        *i += 2;

        let part = data.get(*i..*i + len)?;
        *i += len;
        if data.get(*i..*i + 2)? != b"\r\n" {
            return None;
        }
        *i += 2;

        Some(part)
    }

    fn start_background_task(&self) {
        let state_clone = self.state.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));

            loop {
                interval.tick().await;

                let (buf, file_opt) = {
                    let mut state = match state_clone.lock() {
                        Ok(guard) => guard,
                        Err(_) => break, // Exit background task if Mutex is poisoned
                    };

                    if state.buffer.is_empty() {
                        continue;
                    }

                    let buf = std::mem::take(&mut state.buffer);
                    let file = state.file.take();
                    (buf, file)
                };

                if let Some(mut file) = file_opt {
                    if let Err(e) = file.write_all(&buf).await {
                        eprintln!("AOF write error: {}", e);
                    } else if let Err(e) = file.sync_data().await {
                        eprintln!("AOF flush error: {}", e);
                    }

                    // Return the file back to the state
                    if let Ok(mut state) = state_clone.lock() {
                        state.file = Some(file);
                    } else {
                        break;
                    }
                }
            }
        });
    }

    pub fn append(&self, cmd: &Command) -> Result<(), String> {
        if let Some(data) = self.serialize_command(cmd) {
            let mut state = self
                .state
                .lock()
                .map_err(|_| "AOF mutex poisoned".to_string())?;
            state.buffer.extend_from_slice(&data);
            if let Some(rewrite_buf) = &mut state.rewrite_buffer {
                rewrite_buf.extend_from_slice(&data);
            }
        }
        Ok(())
    }

    fn serialize_command(&self, cmd: &Command) -> Option<Vec<u8>> {
        match cmd {
            Command::Set { key, value } => {
                let mut buf = Vec::new();
                buf.extend_from_slice(b"*3\r\n$3\r\nSET\r\n");

                let key_bytes = key.as_bytes();
                buf.extend_from_slice(format!("${}\r\n", key_bytes.len()).as_bytes());
                buf.extend_from_slice(key_bytes);
                buf.extend_from_slice(b"\r\n");

                buf.extend_from_slice(format!("${}\r\n", value.len()).as_bytes());
                buf.extend_from_slice(value);
                buf.extend_from_slice(b"\r\n");

                Some(buf)
            }
            Command::Get { .. } => None,
            // Catch-all to ensure future non-write commands are not persisted
            #[allow(unreachable_patterns)]
            _ => None,
        }
    }

    pub async fn flush_now(&self) -> Result<(), std::io::Error> {
        let (buf, file_opt) = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "mutex poisoned"))?;
            if state.file.is_none() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::WouldBlock,
                    "AOF file is currently unavailable",
                ));
            }
            let buf = std::mem::take(&mut state.buffer);
            let file = state.file.take();
            (buf, file)
        };

        if let Some(mut file) = file_opt {
            if !buf.is_empty() {
                file.write_all(&buf).await?;
            }
            file.sync_all().await?;
            // Return the file back to the state
            if let Ok(mut state) = self.state.lock() {
                state.file = Some(file);
            }
        }

        Ok(())
    }

    async fn flush_when_available(&self) -> Result<(), std::io::Error> {
        loop {
            match self.flush_now().await {
                Ok(()) => return Ok(()),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
                Err(e) => return Err(e),
            }
        }
    }

    pub async fn rewrite(&self, store: Arc<Mutex<Store>>) -> Result<(), std::io::Error> {
        // Prevent concurrent rewrites
        {
            let state = self.state.lock().unwrap();
            if state.rewrite_buffer.is_some() {
                return Ok(());
            }
        }

        // Start collecting writes before taking the snapshot, so concurrent SETs
        // are either in the snapshot, in this buffer, or both.
        {
            let mut state = self.state.lock().unwrap();
            state.rewrite_buffer = Some(Vec::new());
        }

        // 1. Flush current buffer synchronously to disk for crash safety while
        // the rewrite is in progress.
        if let Err(e) = self.flush_when_available().await {
            if let Ok(mut state) = self.state.lock() {
                state.rewrite_buffer = None;
            }
            return Err(e);
        }

        // 2. Take a consistent snapshot of the in-memory store
        let snapshot = {
            let s = store.lock().unwrap();
            s.get_snapshot()
        };

        let path = self.path.clone();
        let state_clone = self.state.clone();

        // 4. Spawn background task to write new AOF file
        tokio::spawn(async move {
            let tmp_path = format!("{}.tmp", path);
            let mut tmp_file = match File::create(&tmp_path).await {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("AOF Rewrite failed to create tmp file: {}", e);
                    if let Ok(mut state) = state_clone.lock() {
                        state.rewrite_buffer = None;
                    }
                    return;
                }
            };

            // Write snapshot
            for (k, v) in snapshot {
                if let Some(exp) = v.expires_at {
                    if std::time::Instant::now() > exp {
                        continue;
                    }
                }

                let mut buf = Vec::new();
                buf.extend_from_slice(b"*3\r\n$3\r\nSET\r\n");

                let key_bytes = k.as_bytes();
                buf.extend_from_slice(format!("${}\r\n", key_bytes.len()).as_bytes());
                buf.extend_from_slice(key_bytes);
                buf.extend_from_slice(b"\r\n");

                buf.extend_from_slice(format!("${}\r\n", v.data.len()).as_bytes());
                buf.extend_from_slice(&v.data);
                buf.extend_from_slice(b"\r\n");

                if let Err(e) = tmp_file.write_all(&buf).await {
                    eprintln!("AOF Rewrite failed to write tmp file: {}", e);
                    if let Ok(mut state) = state_clone.lock() {
                        state.rewrite_buffer = None;
                    }
                    return;
                }
            }

            // Flush rewrite buffer to tmp_file iteratively to minimize lock duration
            loop {
                let chunk = {
                    let mut state = state_clone.lock().unwrap();
                    std::mem::take(state.rewrite_buffer.as_mut().unwrap())
                };
                if chunk.is_empty() {
                    break;
                }
                if let Err(e) = tmp_file.write_all(&chunk).await {
                    eprintln!("AOF Rewrite failed to write chunk: {}", e);
                    if let Ok(mut state) = state_clone.lock() {
                        state.rewrite_buffer = None;
                    }
                    return;
                }
            }

            if let Err(e) = tmp_file.sync_all().await {
                eprintln!("AOF Rewrite failed to sync tmp file: {}", e);
                if let Ok(mut state) = state_clone.lock() {
                    state.rewrite_buffer = None;
                }
                return;
            }
            drop(tmp_file); // Close it for atomic replace

            // Steal the old file descriptor to pause background flush, and get the final chunk of writes
            let (old_file, final_chunk) = loop {
                let res = {
                    let mut state = state_clone.lock().unwrap();
                    if state.file.is_some() {
                        let f = state.file.take().unwrap();
                        let chunk = state.rewrite_buffer.take().unwrap();
                        Some((f, chunk))
                    } else {
                        None
                    }
                };
                if let Some(r) = res {
                    break r;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            };

            // Drop the old file descriptor so we can rename on Windows safely
            drop(old_file);

            // Append final_chunk to tmp_file
            if !final_chunk.is_empty() {
                if let Ok(mut tmp_file) = OpenOptions::new().append(true).open(&tmp_path).await {
                    let _ = tmp_file.write_all(&final_chunk).await;
                    let _ = tmp_file.sync_all().await;
                }
            }

            // Replace the old AOF file in a Windows-compatible way.
            if let Err(e) = Self::replace_file(&tmp_path, &path).await {
                eprintln!("AOF Rewrite rename failed: {}", e);
                // Reopen the old file to resume background flush on failure
                if let Ok(old_file_reopen) = OpenOptions::new().append(true).open(&path).await {
                    if let Ok(mut state) = state_clone.lock() {
                        state.file = Some(old_file_reopen);
                    }
                }
                return;
            }

            // Open new file and place it back into state
            if let Ok(final_file) = OpenOptions::new().append(true).open(&path).await {
                if let Ok(mut state) = state_clone.lock() {
                    state.file = Some(final_file);

                    // Note: Any new writes that arrived while we were writing final_chunk and renaming
                    // went directly into state.buffer, and they will be flushed to the new file
                    // automatically by the regular background task!
                }
            } else {
                eprintln!("AOF Rewrite failed to open new file");
            }
        });

        Ok(())
    }

    async fn replace_file(tmp_path: &str, path: &str) -> Result<(), std::io::Error> {
        let backup_path = format!("{}.bak", path);
        let _ = tokio::fs::remove_file(&backup_path).await;

        let mut backup_created = false;
        if tokio::fs::metadata(path).await.is_ok() {
            tokio::fs::rename(path, &backup_path).await?;
            backup_created = true;
        }

        match tokio::fs::rename(tmp_path, path).await {
            Ok(()) => {
                if backup_created {
                    let _ = tokio::fs::remove_file(&backup_path).await;
                }
                Ok(())
            }
            Err(err) => {
                if backup_created {
                    let _ = tokio::fs::rename(&backup_path, path).await;
                }
                Err(err)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_path(name: &str) -> String {
        let unique = format!(
            "{}_{}_{}",
            name,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        std::env::temp_dir()
            .join(unique)
            .to_string_lossy()
            .to_string()
    }

    #[test]
    fn restore_ignores_malformed_bulk_lengths_without_panicking() {
        let store = Arc::new(Mutex::new(Store::new()));

        Aof::restore(b"*3\r\n$3\r\nSET\r\n$x\r\nkey\r\n$5\r\nvalue\r\n", &store);

        assert_eq!(store.lock().unwrap().get("key"), None);
    }

    #[tokio::test]
    async fn flush_now_keeps_buffer_when_file_is_unavailable() {
        let aof = Aof {
            state: Arc::new(Mutex::new(AofState {
                buffer: b"pending".to_vec(),
                file: None,
                rewrite_buffer: None,
            })),
            path: unique_path("missing-aof"),
        };

        let err = aof.flush_now().await.unwrap_err();

        assert_eq!(err.kind(), std::io::ErrorKind::WouldBlock);
        assert_eq!(aof.state.lock().unwrap().buffer, b"pending");
    }

    #[tokio::test]
    async fn replace_file_replaces_existing_target() {
        let path = unique_path("appendonly-aof");
        let tmp_path = format!("{}.tmp", path);

        tokio::fs::write(&path, b"old").await.unwrap();
        tokio::fs::write(&tmp_path, b"new").await.unwrap();

        Aof::replace_file(&tmp_path, &path).await.unwrap();

        assert_eq!(tokio::fs::read(&path).await.unwrap(), b"new");
        assert!(tokio::fs::metadata(&tmp_path).await.is_err());

        let _ = tokio::fs::remove_file(&path).await;
    }
}
