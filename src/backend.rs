use anyhow::{bail, Result};
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

/// A backend which only allows connections to a single host/port
pub struct SingleHostBackend {
    host: String,
    port: u16,
}

impl SingleHostBackend {
    pub fn new<H: Into<String>>(host: H, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
        }
    }
}

#[async_trait::async_trait]
impl Backend for SingleHostBackend {
    type Socket = TcpStream;

    async fn connect(&self, host: &str, port: u16) -> Result<Self::Socket> {
        if host != self.host || port != self.port {
            // TODO: test this
            bail!("Connection to disallowed host/port");
        }

        // connect to giphy and return the resulting stream
        Ok(TcpStream::connect(format!("{}:{}", host, port)).await?)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn test_connect_check() {
        let backend = SingleHostBackend::new("good-host", 443);
        assert!(backend.connect("other-host", 443).await.is_err());
        assert!(backend.connect("good-host", 80).await.is_err());
    }

    #[tokio::test]
    async fn test_connect_good() {
        let _ = env_logger::builder().is_test(true).try_init();

        // a tcp server that reads HELLO and writes back WORLD on a port on localhost
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();

            let mut result = vec![];
            socket.read_to_end(&mut result).await.unwrap();
            assert_eq!(&result, b"HELLO");

            socket.write_all(b"WORLD").await.unwrap();
            socket.shutdown().await.unwrap();
        });

        let backend = SingleHostBackend::new("127.0.0.1", port);
        let mut stream = backend.connect("127.0.0.1", port).await.unwrap();

        stream.write_all(b"HELLO").await.unwrap();
        stream.shutdown().await.unwrap();

        let mut response = vec![];
        stream.read_to_end(&mut response).await.unwrap();
        assert_eq!(&response, b"WORLD");
    }
}
