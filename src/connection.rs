use crate::backend::Backend;
use crate::http::{parse_head, ParseHeadResult};
use anyhow::{bail, Context, Result};
use tokio::io::{split, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufStream};

/// Maximum size of a request head; this helps avoid abuse.  It is very low because
/// CONNECT requests should be tiny.  This is allocated on the stack, so increases
/// should be considered carefully.
const MAX_HEAD_SIZE: usize = 1024;

/// Read the HTTP request head from S and write back a response, reading no more than
/// necessary.  Returns the CONNECT host and port.
async fn handle_connect<S: AsyncRead + AsyncWrite + Unpin>(
    socket: &mut S,
) -> Result<(String, u16)> {
    // try to read the head and get the host and port to connect to
    let host;
    let port;

    let mut buf = [0u8; MAX_HEAD_SIZE];
    let mut buf_size = 0;
    loop {
        let n = socket
            .read(&mut buf[buf_size..])
            .await
            .context("reading head from client")?;
        if n == 0 {
            bail!("client hung up while writing HTTP head");
        }
        buf_size += n;

        match parse_head(&buf[..buf_size]) {
            ParseHeadResult::Connect { host: h, port: p } => {
                host = h;
                port = p;
                break;
            }
            ParseHeadResult::Err(e) => return Err(e.context("reading head from client")),
            ParseHeadResult::Incomplete => (), // loop again..
        }
    }

    log::debug!("got CONNECT for {}:{}", host, port);

    // write the response, with no headers..
    socket.write_all(b"HTTP/1.1 200 OK\r\n\r\n").await?;

    Ok((host, port))
}

/// Proxy data bidirectionally between client_socket and backend_socket.
async fn bidirectional_proxy<CS, BS>(client_socket: CS, backend_socket: BS) -> Result<()>
where
    CS: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    BS: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    async fn copy<R: AsyncRead + Unpin, W: AsyncWrite + Unpin>(
        mut read: R,
        read_name: &'static str,
        mut write: W,
        write_name: &'static str,
    ) -> Result<()> {
        let mut buf = [0u8; 1024];
        loop {
            let n = read
                .read(&mut buf)
                .await
                .with_context(|| format!("reading from {}", read_name))?;
            if n == 0 {
                // read socket is closed; we must shut down the write half
                // explicitly (simply dropping it is not enough, as its split
                // half is still running).  We ignore an error here since an
                // error suggests the write side is already shut (e.g., if this
                // socket is completely closed)
                let _ = write.shutdown().await;
                return Ok(());
            }

            // Write the data back
            write
                .write_all(&buf[0..n])
                .await
                .with_context(|| format!("writing to {}", write_name))?;
        }
    }

    // split each socket into read and write halfs, then spawn tasks to
    // copy data between them
    let (client_read, client_write) = split(client_socket);
    let (backend_read, backend_write) = split(backend_socket);

    let copy_client_to_backend = tokio::spawn(async move {
        if let Err(e) = copy(
            client_read,
            "client socket",
            backend_write,
            "backend socket",
        )
        .await
        {
            log::warn!("while proxying: {}", e);
        }
    });

    let copy_backend_to_client = tokio::spawn(async move {
        if let Err(e) = copy(
            backend_read,
            "backend socket",
            client_write,
            "client socket",
        )
        .await
        {
            log::warn!("while proxying: {}", e);
        }
    });

    // wait for those tasks to finish
    let results = tokio::join!(copy_client_to_backend, copy_backend_to_client);
    results.0?;
    results.1?;

    Ok(())
}

/// Handle a single client connection until it ends.  This is implemented in terms of
/// AsyncRead and AsyncWrite, so it has no access to metadata such as the client's IP.
pub async fn connection<S: AsyncRead + AsyncWrite + Unpin + Send + 'static, B: Backend>(
    socket: S,
    backend: B,
) -> Result<()> {
    log::info!("Handling connection");

    // wrap the socket in a bufer so we don't read a byte at a time from the input, but
    // setting writer_capacity to 0 to get immediate writes
    let mut socket = BufStream::with_capacity(8192, 0, socket);

    // read the HTTP request head and write the response
    let (host, port) = handle_connect(&mut socket).await?;

    // connect to the backend
    let backend_socket = backend.connect(&host, port).await?;

    // copy data between the backend and frontend
    Ok(bidirectional_proxy(socket, backend_socket).await?)
}

#[cfg(test)]
mod test {
    use super::*;
    use tokio::io::{duplex, split, DuplexStream};

    /// An echo backend for testing
    pub struct EchoBackend;

    #[async_trait::async_trait]
    impl Backend for EchoBackend {
        type Socket = DuplexStream;
        async fn connect(&self, _host: &str, _port: u16) -> Result<Self::Socket> {
            let (client, mut server) = tokio::io::duplex(1024);

            // spawn a task to echo data
            tokio::spawn(async move {
                let mut buf = [0u8; 1024];
                loop {
                    log::trace!("echo reading");
                    let n = match server.read(&mut buf).await {
                        Ok(n) if n == 0 => {
                            log::trace!("echo got EOF");
                            return;
                        }
                        Ok(n) => n,
                        Err(e) => {
                            eprintln!("failed to read from socket; err = {:?}", e);
                            return;
                        }
                    };
                    log::trace!("echo read {} bytes", n);

                    log::trace!("echo writing");
                    if let Err(e) = server.write_all(&buf[0..n]).await {
                        eprintln!("failed to write to socket; err = {:?}", e);
                        return;
                    }
                    log::trace!("echo wrote {} bytes", n);
                }
            });

            return Ok(client);
        }
    }

    #[tokio::test]
    async fn test_connect() {
        let _ = env_logger::builder().is_test(true).try_init();

        let (mut client, server) = duplex(64);
        let server_task = tokio::spawn(async move {
            connection(server, EchoBackend).await.unwrap();
        });
        let client_task = tokio::spawn(async move {
            client
                .write_all(b"CONNECT foo.com:1234 HTTP/1.1\r\n\r\n")
                .await
                .unwrap();

            const EXPECTED_RESPONSE: &[u8] = b"HTTP/1.1 200 OK\r\n\r\n";
            let mut buf = [0u8; EXPECTED_RESPONSE.len()];
            assert_eq!(
                client.read_exact(&mut buf).await.unwrap(),
                EXPECTED_RESPONSE.len()
            );
            assert_eq!(&buf, EXPECTED_RESPONSE);

            // write some bytes to the echo server
            client.write_all(b"Hello, Internet").await.unwrap();

            // half-close the connection
            log::trace!("client half-closing");
            let (mut read, mut write) = split(client);
            write.shutdown().await.unwrap();

            // expect to read those bytes back from the read side
            let mut buf = vec![];
            read.read_to_end(&mut buf).await.unwrap();
            assert_eq!(&buf, b"Hello, Internet");
        });

        // join the threads to check that the server task exits when the connection closes
        tokio::join!(server_task).0.unwrap();
        tokio::join!(client_task).0.unwrap();
    }
}
