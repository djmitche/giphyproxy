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
    use httptest::{matchers::*, responders::*, Expectation, Server};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn test_connect_check() {
        let backend = SingleHostBackend::new("good-host", 443);
        assert!(backend.connect("other-host", 443).await.is_err());
        assert!(backend.connect("good-host", 80).await.is_err());
    }

    #[tokio::test]
    async fn test_connect_good() {
        let server = Server::run();
        server.expect(
            Expectation::matching(request::method_path("GET", "/foo"))
                .respond_with(status_code(200)),
        );

        let url = server.url("/foo");
        let host = url.host().unwrap();
        let port = url.port_u16().unwrap();
        let backend = SingleHostBackend::new(host, port);
        let mut stream = backend.connect(host, port).await.unwrap();

        stream
            .write_all(b"GET /foo HTTP/1.0\r\n\r\n")
            .await
            .unwrap();

        let mut response = vec![];
        stream.read_to_end(&mut response).await.unwrap();
        // (httptest will bail out if we did something wrong)
    }
}
