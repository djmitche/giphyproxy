mod backend;
mod connection;
mod http;
mod listen;

use anyhow::Result;
use listen::start_listening;
use std::time::Duration;
use tokio::time;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    // TODO: bound IP and port should be configurable via env vars (11-factor style)
    start_listening("127.0.0.1:8080").await?;

    // sleep forever, as the listener runs in another task
    loop {
        time::sleep(Duration::from_secs(3600)).await;
    }
}

#[cfg(test)]
mod test {
    use super::*;

    /// Test the whole process, for a single request.  This
    /// test requires that
    ///  * giphy be up and available
    ///  * port 8080 be unused locally
    #[tokio::test]
    async fn giphy_test() {
        let _ = env_logger::builder().is_test(true).try_init();

        // start the server
        start_listening("127.0.0.1:8080").await.unwrap();

        // connect with a "real" HTTP client
        let client = reqwest::Client::builder()
            .proxy(reqwest::Proxy::https("http://127.0.0.1:8080").unwrap())
            .build()
            .unwrap();

        let res = client
            .get("https://api.giphy.com/v1/gifs/search")
            .send()
            .await
            .unwrap();

        // giphy expects an API key, which we don't supply -- but getting back the 401
        // is good evidence that the proxy worked!
        assert_eq!(res.status(), reqwest::StatusCode::UNAUTHORIZED);
    }
}
