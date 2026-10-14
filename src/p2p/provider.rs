use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::{Error, Result};

/// Transport for the P2P protocol, responsible for re-assembling full message payloads.
#[async_trait]
pub trait P2PProvider: Send {
    /// Write one complete encoded message.
    async fn write(&mut self, encoded_message: &[u8]) -> Result<()>;
    /// Read the next complete encoded message, `None` when the connection is closed.
    async fn read(&mut self) -> Result<Option<Vec<u8>>>;
    /// Gracefully shut down the connection.
    async fn end(&mut self) -> Result<()>;
}

/// Provider framing messages with a little-endian u32 length prefix over any
/// async stream (e.g. a `tokio::net::TcpStream`).
#[derive(Debug)]
pub struct SimpleEnvelopeP2PProvider<S> {
    stream: S,
    /// Maximum accepted incoming message size in bytes.
    pub max_read_length: usize,
}

impl<S> SimpleEnvelopeP2PProvider<S> {
    pub const DEFAULT_MAX_READ_LENGTH: usize = 8 * 1024 * 1024;

    pub fn new(stream: S) -> Self {
        SimpleEnvelopeP2PProvider {
            stream,
            max_read_length: Self::DEFAULT_MAX_READ_LENGTH,
        }
    }

    pub fn into_inner(self) -> S {
        self.stream
    }
}

#[async_trait]
impl<S: AsyncRead + AsyncWrite + Unpin + Send> P2PProvider for SimpleEnvelopeP2PProvider<S> {
    async fn write(&mut self, data: &[u8]) -> Result<()> {
        let mut buf = Vec::with_capacity(4 + data.len());
        buf.extend_from_slice(&(data.len() as u32).to_le_bytes());
        buf.extend_from_slice(data);
        self.stream.write_all(&buf).await?;
        self.stream.flush().await?;
        Ok(())
    }

    async fn read(&mut self) -> Result<Option<Vec<u8>>> {
        let mut len = [0u8; 4];
        match self.stream.read_exact(&mut len).await {
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
            Err(e) => return Err(e.into()),
        }
        let len = u32::from_le_bytes(len) as usize;
        if len > self.max_read_length {
            return Err(Error::Msg("Incoming Message too long".into()));
        }
        let mut buf = vec![0u8; len];
        self.stream.read_exact(&mut buf).await?;
        Ok(Some(buf))
    }

    async fn end(&mut self) -> Result<()> {
        self.stream.shutdown().await?;
        Ok(())
    }
}
