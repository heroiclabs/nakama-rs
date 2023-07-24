use nakama_rs::http_adapter::RestHttpAdapter;
use nakama_rs::web_socket_adapter::WebSocketAdapter;
use nakama_rs::{Client, DefaultClient, WebSocket};
use std::collections::HashMap;
use std::sync::Arc;

#[tokio::test]
async fn get_pokemon() -> anyhow::Result<()> {
    let http_adapter = RestHttpAdapter::new("http://127.0.0.1", 7350);
    let client = DefaultClient::new(http_adapter, "defaultkey", "");
    let data = client
        .authenticate_custom("12311111", None, true, HashMap::new())
        .await?;
    println!("{:?}", data);
    // let adapter = WebSocketAdapter::new();
    // let web_socket = Arc::new(WebSocket::new(adapter));
    // web_socket.c
    Ok(())
}
