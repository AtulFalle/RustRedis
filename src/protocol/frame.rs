use bytes::Bytes;

#[derive(Debug)]
pub enum Frame {
    Simple(String),
    Error(String),
    Bulk(Bytes),
    Null,
    Array(Vec<Frame>),
}
