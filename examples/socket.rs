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
use nakama_rs::socket::{Socket, StatusPresenceEvent};
use nakama_rs::test_helpers::tick_socket;
use nakama_rs::web_socket::WebSocket;
use std::collections::HashMap;
use std::sync::mpsc;

// This example demonstrates how to connect to a socket
fn main() {
    block_on(async {
        let client = DefaultClient::new_with_adapter_and_defaults();
        let mut socket = WebSocket::new_with_adapter();
        tick_socket(&socket);

        let (tx_presence, rx_presence) = mpsc::channel::<StatusPresenceEvent>();

        let session = client
            .authenticate_device("socket_example_id", None, true, HashMap::new())
            .await
            .expect("Failed to authenticate");

        socket.on_received_status_presence(move |presence| {
            tx_presence
                .send(presence)
                .expect("Failed to send status presence");
        });

        socket.connect(&session, true, -1).await;

        let status_presence = rx_presence
            .recv()
            .expect("Failed to receive status presence");
        println!("Status presence: {:?}", status_presence);
    })
}
