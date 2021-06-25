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

use std::collections::HashMap;
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;
use std::thread::{sleep, spawn};
use std::time::Duration;

use futures::executor::block_on;
use simple_logger::SimpleLogger;

use log::{trace, LevelFilter};
use nakama_rs::client::Client;
use nakama_rs::default_client::DefaultClient;
use nakama_rs::http_adapter::RestHttpAdapter;
use nakama_rs::socket::Socket;
use nakama_rs::web_socket::WebSocket;
use nakama_rs::web_socket_adapter::WebSocketAdapter;

fn main() {
    SimpleLogger::new()
        .with_level(LevelFilter::Off)
        .with_module_level("nakama_rs", LevelFilter::Trace)
        .init()
        .unwrap();

    let http_adapter = RestHttpAdapter::new("http://127.0.0.1", 7350);
    let client = DefaultClient::new(http_adapter, "defaultkey", "");
    let adapter = WebSocketAdapter::new();
    let adapter2 = WebSocketAdapter::new();
    let web_socket = Arc::new(WebSocket::new(adapter));
    let web_socket2 = Arc::new(WebSocket::new(adapter2));

    let (kill_tick, rc_kill) = channel();
    std::thread::spawn({
        let web_socket = web_socket.clone();
        let web_socket2 = web_socket2.clone();
        move || {
            // Wait for 5 seconds because `do_some_chatting` doesn't inform us about being done
            loop {
                // This could also be called in a different thread than the main/game thread. The callbacks
                // will be called in the same thread, invoking e.g. `on_received_channel_message`.
                // Note that `tick` is also necessary to wake futures like `web_socket.join_chat` - it is not only necessary
                // for the callbacks.
                trace!("Ticking websockets");
                web_socket.tick();
                web_socket2.tick();
                if let Ok(_) = rc_kill.try_recv() {
                    return;
                }

                sleep(Duration::from_millis(500));
            }
        }
    });

    block_on({
        async {
            let session = client
                .authenticate_device("testdeviceid", None, true, HashMap::new())
                .await;
            let session2 = client
                .authenticate_device("testdeviceid2", None, true, HashMap::new())
                .await;
            let session = session.unwrap();
            let session2 = session2.unwrap();
            web_socket.connect(&session, true, -1).await;
            web_socket2.connect(&session2, true, -1).await;
        }
    });

    let do_some_chatting = spawn(move || {
        block_on(async move {
            web_socket
                .join_chat("MyRoom", 1, false, false)
                .await
                .expect("Failed to join chat");
            let channel = web_socket2
                .join_chat("MyRoom", 1, false, false)
                .await
                .unwrap();
            web_socket2
                .write_chat_message(&channel.id, "{\"text\":\"Hello World!\"}")
                .await
                .expect("Failed to write chat message");
        })
    });

    do_some_chatting.join().expect("Failed to join thread");

    kill_tick.send(()).expect("Failed to send kill");
}
