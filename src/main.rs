mod command;
mod connection;
mod engine;
mod protocol;
mod server;
mod storage;

#[tokio::main]
async fn main() {
    println!("Starting Redis-like server...");

    if let Err(e) = server::start("127.0.0.1:6379").await {
        eprintln!("Server error: {}", e);
    }
}
