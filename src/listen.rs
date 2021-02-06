use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// Handle a single client connection until it ends.
pub async fn connection(mut socket: TcpStream) -> () {
    let mut buf = [0; 1024];

    // In a loop, read data from the socket and write the data back.
    loop {
        let n = match socket.read(&mut buf).await {
            // socket closed
            Ok(n) if n == 0 => return,
            Ok(n) => n,
            Err(e) => {
                eprintln!("failed to read from socket; err = {:?}", e);
                return;
            }
        };

        // Write the data back
        if let Err(e) = socket.write_all(&buf[0..n]).await {
            eprintln!("failed to write to socket; err = {:?}", e);
            return;
        }
    }
}

/// Listen for connections on the given IP and port, handling each one with `connection`.
pub async fn listen(ip_and_port: &str) -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(ip_and_port).await?;

    loop {
        let (socket, _) = listener.accept().await?;
        tokio::spawn(connection(socket));
    }
}
