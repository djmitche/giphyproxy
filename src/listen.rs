use anyhow::{Context, Result};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// Handle a single client connection until it ends.
pub async fn connection(mut socket: TcpStream) -> Result<()> {
    let mut buf = [0; 1024];

    log::info!("Handling connection"); // NOTE: remote IP is not logged

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

/// Listen for connections on the given IP and port, handling each one with `connection`.
pub async fn listen(ip_and_port: &str) -> Result<()> {
    log::info!("Listening on {}", ip_and_port);
    let listener = TcpListener::bind(ip_and_port).await?;

    loop {
        let (socket, _) = listener.accept().await?;
        tokio::spawn(async move {
            if let Err(e) = connection(socket).await {
                log::error!("connection handler failed: {:?}", e);
            }
        });
    }
}
