use crate::command::Command;
use crate::connection::Connection;
use crate::engine::Engine;
use crate::protocol::Frame;
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{Duration, sleep};

pub async fn start(addr: &str) -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(addr).await?;
    let engine = Engine::new();

    println!("Server listening on {}", addr);

    // Background cleanup task
    let engine_cleanup = engine.clone();
    tokio::spawn(async move {
        loop {
            engine_cleanup.cleanup();
            sleep(Duration::from_secs(5)).await;
        }
    });

    loop {
        let (socket, _) = listener.accept().await?;
        let engine_clone = engine.clone();

        tokio::spawn(async move {
            if let Err(e) = handle_connection(socket, engine_clone).await {
                eprintln!("Connection error: {}", e);
            }
        });
    }
}

async fn handle_connection(
    socket: TcpStream,
    engine: Engine,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut connection = Connection::new(socket);

    println!("Client connected");

    while let Some(frame) = connection.read_frame().await? {
        match Command::from_frame(frame) {
            Ok(cmd) => {
                let result = engine.execute(cmd).await;

                let response = if result == "(nil)" {
                    Frame::Null
                } else {
                    Frame::Bulk(bytes::Bytes::from(result))
                };

                connection.write_frame(response).await?;
            }

            Err(err) => {
                connection.write_frame(Frame::Error(err)).await?;
            }
        }
    }

    println!("Client disconnected");

    Ok(())
}
