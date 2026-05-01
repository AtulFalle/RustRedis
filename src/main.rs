mod command;
mod connection;
mod server;
mod storage;
mod engine;
mod protocol;

#[tokio::main]
async fn main() {
    println!("Starting Redis-like server...");

    if let Err(e) = server::start("127.0.0.1:6379").await {
        eprintln!("Server error: {}", e);
    }
}
