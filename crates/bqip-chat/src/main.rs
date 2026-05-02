use std::env;
use std::net::SocketAddr;

use bqip_chat::{serve, ChatState};
use bqip_control::TrainingControlConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr_text = match env::var("BQIP_CHAT_ADDR") {
        Ok(value) => value,
        Err(_) => "127.0.0.1:8787".to_string(),
    };
    let addr: SocketAddr = addr_text.parse()?;
    let state = ChatState::new(TrainingControlConfig::default())?;
    serve(addr, state).await?;
    Ok(())
}
