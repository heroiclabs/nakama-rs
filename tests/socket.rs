// Copyright 2021 The Nakama Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use futures::executor::block_on;
use nakama_rs::client::Client;
use nakama_rs::default_client::DefaultClient;
use nakama_rs::session::Session;
use nakama_rs::socket::Socket;
use nakama_rs::test_helpers::tick_socket;
use nakama_rs::web_socket::WebSocket;
use nakama_rs::web_socket_adapter::WebSocketAdapter;
use std::collections::HashMap;
use std::sync::mpsc;

async fn socket_with_user(id: &str) -> (Session, WebSocket<WebSocketAdapter>) {
    let client = DefaultClient::new_with_adapter();
    let socket = WebSocket::new_with_adapter();
    tick_socket(&socket);

    let session = client
        .authenticate_device(id, Some("SocketTestUser"), true, HashMap::new())
        .await
        .unwrap();
    (session, socket)
}

#[test]
fn test_status_presence_received_after_connect() {
    block_on(async {
        let (mut session, mut socket) = socket_with_user("socket_test_user").await;

        let (tx_presence, rx_presence) = mpsc::channel();
        socket.on_received_status_presence(move |presence| {
            tx_presence
                .send(presence)
                .expect("Failed to send status presence");
        });
        socket.connect(&mut session, true, -1).await;

        let status_presence = rx_presence.recv().expect("Failed to recv status presence");
        println!("Status presence: {:?}", status_presence);
        assert_eq!(status_presence.joins.len(), 1);
        assert_eq!(status_presence.joins[0].username, "SocketTestUser");
    });
}

#[test]
fn test_on_connected_triggered() {
    let (tx, rx) = mpsc::channel::<()>();

    block_on(async {
        let (mut session, mut socket) = socket_with_user("socket_test_user").await;

        socket.on_connected(move || {
            tx.send(()).expect("Failed to send connected status");
        });

        socket.connect(&mut session, true, -1).await;
    });

    rx.recv().expect("Failed to receive connected status");
}
