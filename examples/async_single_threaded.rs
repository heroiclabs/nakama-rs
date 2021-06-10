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
use std::thread::sleep;
use std::time::Duration;

use log::LevelFilter;
use simple_logger::SimpleLogger;

use cassette::{pin_mut, yield_now, Cassette};

use crate::State::SentMessage;
use nakama_rs::client::Client;
use nakama_rs::default_client::DefaultClient;
use nakama_rs::http_adapter::RestHttpAdapter;
use nakama_rs::socket::Socket;
use nakama_rs::web_socket::WebSocket;
use nakama_rs::web_socket_adapter::WebSocketAdapter;

#[derive(Eq, PartialEq, Clone, Debug)]
enum State {
    Connecting,
    Connected,
    JoiningChat,
    JoinedChat,
    SendingMessage,
    SentMessage,
    Exiting,
}

use std::cell::RefCell;
use State::*;

fn main() {
    SimpleLogger::new()
        .with_level(LevelFilter::Off)
        .with_module_level("nakama_rs", LevelFilter::Trace)
        .init()
        .unwrap();

    let http_adapter = RestHttpAdapter::new("http://127.0.0.1", 7350);
    let client = DefaultClient::new(http_adapter);
    let adapter = WebSocketAdapter::new();
    let adapter2 = WebSocketAdapter::new();
    let web_socket = WebSocket::new(adapter);
    let web_socket2 = WebSocket::new(adapter2);

    let state = RefCell::new(Connecting);

    let network_future = {
        async {
            let mut channel = None;
            loop {
                let s = state.borrow().clone();
                match s {
                    Connecting => {
                        let session = client
                            .authenticate_device("testdeviceid", None, true, HashMap::new())
                            .await;
                        let session2 = client
                            .authenticate_device("testdeviceid2", None, true, HashMap::new())
                            .await;
                        let mut session = session.unwrap();
                        let mut session2 = session2.unwrap();
                        web_socket.connect(&mut session, true, -1).await;
                        web_socket2.connect(&mut session2, true, -1).await;
                        state.replace(Connected);
                    }
                    JoiningChat => {
                        web_socket
                            .join_chat("MyRoom", 1, false, false)
                            .await
                            .expect("Failed to join chat");
                        channel = Some(
                            web_socket2
                                .join_chat("MyRoom", 1, false, false)
                                .await
                                .unwrap(),
                        );
                        state.replace(JoinedChat);
                    }
                    SendingMessage => {
                        web_socket2
                            .write_chat_message(
                                &channel.take().unwrap().id,
                                "{\"text\":\"Hello World!\"}",
                            )
                            .await
                            .expect("Failed to write chat message");
                        state.replace(SentMessage);
                    }
                    _ => {
                        yield_now().await;
                    }
                }
            }
        }
    };

    pin_mut!(network_future);
    let mut cassette = Cassette::new(network_future);

    loop {
        sleep(Duration::from_millis(16));

        web_socket.tick();
        web_socket2.tick();
        cassette.poll_on();

        let s = state.borrow().clone();
        match s {
            Connecting => {}
            Connected => {
                // Usually the state transition would be done on e.g. a button click
                state.replace(JoiningChat);
            }
            JoiningChat => {}
            JoinedChat => {
                state.replace(SendingMessage);
            }
            SendingMessage => {}
            SentMessage => {
                state.replace(Exiting);
            }
            Exiting => {
                return;
            }
        }
    }
}
