use crate::backend::SingleHostBackend;
use crate::connection::connection;
use anyhow::Result;
use tokio::net::TcpListener;

/// Listen for connections on the given IP and port, handling each one with `connection`.
pub async fn listen(ip_and_port: &str) -> Result<()> {
    log::info!("Listening on {}", ip_and_port);
    let listener = TcpListener::bind(ip_and_port).await?;

    loop {
        let (socket, _) = listener.accept().await?;
        let backend = SingleHostBackend::new("api.giphy.com", 443);

        tokio::spawn(async move {
            if let Err(e) = connection(socket, backend).await {
                log::error!("connection handler failed: {:?}", e);
            }
        });
    }
}
