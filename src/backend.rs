use anyhow::Result;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;

/// A backend represents a service to which this app can proxy.
#[async_trait::async_trait]
pub trait Backend {
    type Socket: AsyncRead + AsyncWrite + Unpin + Send + 'static;

    /// Connect to the backend using the given host and port, and return a connected
    /// socket.
    async fn connect(&self, host: &str, port: u16) -> Result<Self::Socket>;
}

/// A backend which only allows connections to the Giphy API
pub struct GiphyBackend;

#[async_trait::async_trait]
impl Backend for GiphyBackend {
    type Socket = TcpStream;

    async fn connect(&self, _host: &str, _port: u16) -> Result<Self::Socket> {
        todo!()
    }
}

#[cfg(test)]
mod test {}
