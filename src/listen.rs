use crate::backend::SingleHostBackend;
use crate::connection::connection;
use anyhow::Result;
use tokio::net::TcpListener;

/// Listen for connections on the given IP and port, handling each one with `connection`.
///
/// This function returns when the port is bound, with the listener running in a separate task.
pub async fn start_listening(ip_and_port: &str) -> Result<()> {
    log::info!("Listening on {}", ip_and_port);
    let listener = TcpListener::bind(ip_and_port).await?;

    tokio::spawn(async move {
        loop {
            let (socket, _) = listener.accept().await.expect("socket.accept failed");
            let backend = SingleHostBackend::new("api.giphy.com", 443);

            tokio::spawn(async move {
                if let Err(e) = connection(socket, backend).await {
                    log::error!("connection handler failed: {:?}", e);
                }
            });
        }
    });

    Ok(())
}
