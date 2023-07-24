use nakama_rs::http_adapter::RestHttpAdapter;
use nakama_rs::matchmaker::Matchmaker;
use nakama_rs::socket_adapter::SocketAdapter;
use nakama_rs::test_helpers::tick_socket;
use nakama_rs::web_socket_adapter::WebSocketAdapter;
use nakama_rs::{Client, DefaultClient, Socket, WebSocket};
use std::collections::HashMap;
use std::thread::sleep;
use std::time::Duration;

pub const DEFAULT_PORT: u32 = 7350;
pub const DEFAULT_HOST: &str = "http://127.0.0.1";
pub const DEFAULT_SERVER_KEY: &str = "defaultkey";
pub const DEFAULT_SERVER_PASSWORD: &str = "";

#[tokio::test]
async fn matchmaker_add() -> anyhow::Result<()> {
    let adapter = RestHttpAdapter::new(DEFAULT_HOST, DEFAULT_PORT);
    let client = DefaultClient::new(adapter, DEFAULT_SERVER_KEY, DEFAULT_SERVER_PASSWORD);
    let sess = client
        .authenticate_custom("123213132", None, true, HashMap::new())
        .await?;
    // let socket = WebSocket::new_with_adapter();

    // // tick_socket(&socket);

    let mut socket_adapter = WebSocketAdapter::new();
    socket_adapter.connect("ws://127.0.0.1:7348", -1);
    let socket = WebSocket::new(socket_adapter);
    socket.connect("ws://127.0.0.1:7350", &sess, true, -1);
    let matchmaker = Matchmaker::new();
    let ticket = socket.add_matchmaker(&matchmaker).await?;
    println!("{:?}", ticket);
    // socket_adapter.on_received(move |data| println!("{:?}", data));
    // sleep(Duration::from_secs(1));
    // println!("Sending!");
    // match socket_adapter.send("Hello", false) {
    //     Err(e) => {
    //         println!("{}", e.to_string());
    //     }
    //     _ => {}
    // }
    // sleep(Duration::from_secs(1));
    // println!("Tick!");
    // socket_adapter.tick();
    // sleep(Duration::from_secs(1));
    // println!("Tick!");
    // socket_adapter.tick();
    Ok(())
}
