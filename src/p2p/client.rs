use super::provider::{P2PProvider, SimpleEnvelopeP2PProvider};
use super::types::{NetMessage, TimeMessage};
use crate::chain::Int64;
use crate::serializer::Serializer;
use crate::Result;

/// P2P protocol client.
///
/// ```no_run
/// # async fn run() -> vex::Result<()> {
/// use vex::p2p::{P2PClient, NetMessage};
///
/// let mut client = P2PClient::connect("127.0.0.1:9876").await?;
/// while let Some(message) = client.next_message().await? {
///     if let NetMessage::SignedBlock(block) = message {
///         println!("block {}", block.block_num());
///     }
/// }
/// # Ok(())
/// # }
/// ```
pub struct P2PClient<P: P2PProvider> {
    pub provider: P,
}

impl P2PClient<SimpleEnvelopeP2PProvider<tokio::net::TcpStream>> {
    /// Connect to a node over TCP using the default length-prefixed framing.
    pub async fn connect(addr: impl tokio::net::ToSocketAddrs) -> Result<Self> {
        let stream = tokio::net::TcpStream::connect(addr).await?;
        stream.set_nodelay(true)?;
        Ok(P2PClient::new(SimpleEnvelopeP2PProvider::new(stream)))
    }
}

impl<P: P2PProvider> P2PClient<P> {
    pub fn new(provider: P) -> Self {
        P2PClient { provider }
    }

    /// Encode and send a message.
    pub async fn send(&mut self, message: &NetMessage) -> Result<()> {
        let data = Serializer::encode(message);
        self.provider.write(&data).await
    }

    /// Read and decode the next message, `None` when the connection is closed.
    pub async fn next_message(&mut self) -> Result<Option<NetMessage>> {
        match self.provider.read().await? {
            Some(data) => Ok(Some(Serializer::decode(data)?)),
            None => Ok(None),
        }
    }

    /// Send a time message (heartbeat), call this periodically to keep the
    /// connection alive (e.g. from a `tokio::time::interval`).
    pub async fn send_heartbeat(&mut self) -> Result<()> {
        let message = NetMessage::Time(TimeMessage {
            org: Int64(crate::chain::now_ms()),
            rec: Int64(0),
            xmt: Int64(0),
            dst: Int64(0),
        });
        self.send(&message).await
    }

    /// Gracefully close the connection.
    pub async fn end(&mut self) -> Result<()> {
        self.provider.end().await
    }
}
