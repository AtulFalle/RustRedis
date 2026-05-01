use bytes::{Buf, Bytes, BytesMut};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::protocol::Frame;

pub struct Connection {
    stream: TcpStream,
    buffer: BytesMut,
}

impl Connection {
    pub fn new(stream: TcpStream) -> Self {
        Self {
            stream,
            buffer: BytesMut::with_capacity(1024),
        }
    }

    // =========================
    // READ RESP FRAME
    // =========================
    pub async fn read_frame(
        &mut self,
    ) -> Result<Option<Frame>, Box<dyn std::error::Error + Send + Sync>> {
        loop {
            if let Some(frame) = self.parse_frame()? {
                return Ok(Some(frame));
            }

            let bytes_read = self.stream.read_buf(&mut self.buffer).await?;

            if bytes_read == 0 {
                if self.buffer.is_empty() {
                    return Ok(None);
                } else {
                    return Err("connection closed unexpectedly".into());
                }
            }
        }
    }

    fn parse_frame(&mut self) -> Result<Option<Frame>, Box<dyn std::error::Error + Send + Sync>> {
        if self.buffer.is_empty() {
            return Ok(None);
        }

        match self.buffer[0] {
            b'*' => self.parse_array(),
            _ => {
                // wait for more data instead of failing immediately
                Ok(None)
            }
        }
    }

    fn parse_array(&mut self) -> Result<Option<Frame>, Box<dyn std::error::Error + Send + Sync>> {
        let mut buf = &self.buffer[..];

        buf.advance(1); // skip '*'

        let len = self.read_number(&mut buf)? as usize;

        let mut frames = Vec::with_capacity(len);

        for _ in 0..len {
            frames.push(self.parse_bulk_string(&mut buf)?);
        }

        let consumed = self.buffer.len() - buf.len();
        self.buffer.advance(consumed);

        Ok(Some(Frame::Array(frames)))
    }

    fn parse_bulk_string(
        &self,
        buf: &mut &[u8],
    ) -> Result<Frame, Box<dyn std::error::Error + Send + Sync>> {
        if buf[0] != b'$' {
            return Err("Expected bulk string".into());
        }

        buf.advance(1);

        let len = self.read_number(buf)? as usize;

        let data = buf[..len].to_vec();
        buf.advance(len + 2); // data + \r\n

        Ok(Frame::Bulk(Bytes::from(data)))
    }

    fn read_number(
        &self,
        buf: &mut &[u8],
    ) -> Result<i64, Box<dyn std::error::Error + Send + Sync>> {
        let mut num = 0;

        while buf[0] != b'\r' {
            num = num * 10 + (buf[0] - b'0') as i64;
            buf.advance(1);
        }

        buf.advance(2); // \r\n

        Ok(num)
    }

    // =========================
    // WRITE RESP FRAME
    // =========================
    pub async fn write_frame(
        &mut self,
        frame: Frame,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match frame {
            Frame::Simple(s) => {
                self.stream.write_all(b"+").await?;
                self.stream.write_all(s.as_bytes()).await?;
                self.stream.write_all(b"\r\n").await?;
            }

            Frame::Error(e) => {
                self.stream.write_all(b"-").await?;
                self.stream.write_all(e.as_bytes()).await?;
                self.stream.write_all(b"\r\n").await?;
            }

            Frame::Bulk(data) => {
                let len = data.len();
                self.stream
                    .write_all(format!("${}\r\n", len).as_bytes())
                    .await?;
                self.stream.write_all(&data).await?;
                self.stream.write_all(b"\r\n").await?;
            }

            Frame::Null => {
                self.stream.write_all(b"$-1\r\n").await?;
            }

            Frame::Array(_) => {
                return Err("Array write not implemented".into());
            }
        }

        Ok(())
    }
}
