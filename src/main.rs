mod backend;
mod connection;
mod http;
mod listen;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    // TODO: bound IP and port should be configurable via env vars (11-factor style)
    Ok(listen::listen("127.0.0.1:8080").await?)
}
