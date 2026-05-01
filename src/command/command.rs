use crate::protocol::Frame;

#[derive(Debug)]
pub enum Command {
    Set { key: String, value: Vec<u8> },
    Get { key: String },
}

impl Command {
    pub fn from_frame(frame: Frame) -> Result<Self, String> {
        match frame {
            Frame::Array(mut arr) => {
                if arr.is_empty() {
                    return Err("Empty command".into());
                }

                let cmd = match arr.remove(0) {
                    Frame::Bulk(b) => String::from_utf8_lossy(&b).to_uppercase(),
                    _ => return Err("Invalid command".into()),
                };

                match cmd.as_str() {
                    "SET" => {
                        let key = match arr.remove(0) {
                            Frame::Bulk(b) => String::from_utf8_lossy(&b).to_string(),
                            _ => return Err("Invalid key".into()),
                        };

                        let value = match arr.remove(0) {
                            Frame::Bulk(b) => b.to_vec(),
                            _ => return Err("Invalid value".into()),
                        };

                        Ok(Command::Set { key, value })
                    }

                    "GET" => {
                        let key = match arr.remove(0) {
                            Frame::Bulk(b) => String::from_utf8_lossy(&b).to_string(),
                            _ => return Err("Invalid key".into()),
                        };

                        Ok(Command::Get { key })
                    }

                    _ => Err("Unknown command".into()),
                }
            }

            _ => Err("Invalid frame".into()),
        }
    }
}
