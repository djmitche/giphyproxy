mod listen;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // TODO: bound IP and port should be configurable via env vars (11-factor style)
    Ok(listen::listen("127.0.0.1:8080").await?)
}
