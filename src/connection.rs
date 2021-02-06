use anyhow::{Context, Result};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Handle a single client connection until it ends.  This is implemented in terms of
/// AsyncRead and AsyncWrite, so it has no access to metadata such as the client's IP.
pub async fn connection<S: AsyncRead + AsyncWrite + Unpin>(mut socket: S) -> Result<()> {
    let mut buf = [0; 1024];

    log::info!("Handling connection");

    // In a loop, read data from the socket and write the data back.
    loop {
        let n = socket.read(&mut buf).await.context("reading from socket")?;
        if n == 0 {
            // socket closed
            break;
        }

        // Write the data back
        socket
            .write_all(&buf[0..n])
            .await
            .context("writing to socket")?;
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use tokio::io::duplex;

    #[tokio::test]
    async fn test_echo() {
        let (mut client, server) = duplex(64);
        let server_task = tokio::spawn(async move {
            connection(server).await.unwrap();
        });
        let client_task = tokio::spawn(async move {
            client.write_all(b"HELLO").await.unwrap();

            let mut buf = [0u8; 5]; // just enough space for HELLO
            assert_eq!(client.read_exact(&mut buf).await.unwrap(), 5);
            assert_eq!(&buf, b"HELLO");

            // close the connection
            drop(client);
        });

        // join the threads to check that the server task exits when the connection closes
        tokio::join!(server_task).0.unwrap();
        tokio::join!(client_task).0.unwrap();
    }
}
