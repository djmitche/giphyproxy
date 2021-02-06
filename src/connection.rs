use crate::http::{parse_head, ParseHeadResult};
use anyhow::{bail, Context, Result};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

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

/// Handle a single client connection until it ends.  This is implemented in terms of
/// AsyncRead and AsyncWrite, so it has no access to metadata such as the client's IP.
pub async fn connection<S: AsyncRead + AsyncWrite + Unpin>(mut socket: S) -> Result<()> {
    log::info!("Handling connection");

    // TODO: wrap socket in a buffering impl so we don't read a byte at a time from the input

    let (_host, _port) = handle_connect(&mut socket).await?;

    // temporarily emulate the backend service

    let mut buf = [0u8; 1024];
    let n = socket.read(&mut buf).await.context("reading from socket")?;
    if n == 0 {
        // socket closed
        return Ok(());
    }

    // Write the data back
    socket
        .write_all(b"HTTP/1.1 200 OK\r\n\r\nHello, world.")
        .await
        .context("writing to socket")?;

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use tokio::io::duplex;

    #[tokio::test]
    async fn test_connect() {
        let (mut client, server) = duplex(64);
        let server_task = tokio::spawn(async move {
            connection(server).await.unwrap();
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

            // close the connection
            drop(client);
        });

        // join the threads to check that the server task exits when the connection closes
        tokio::join!(server_task).0.unwrap();
        tokio::join!(client_task).0.unwrap();
    }
}
