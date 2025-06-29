mod server;
mod clientconn;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = server::COTORServer::new().await?;
    server.start().await?;
    server.stop().await;
    Ok(())
}
