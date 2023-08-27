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

use chrono::Utc;
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;
use std::thread::{sleep, spawn};
use std::time::Duration;

use futures::executor::block_on;
use simple_logger::SimpleLogger;

use log::{trace, LevelFilter};
use nakama_rs::client::Client;
use nakama_rs::default_client::DefaultClient;
use nakama_rs::http_adapter::RestHttpAdapter;
use nakama_rs::matchmaker::Matchmaker;
use nakama_rs::socket::{MatchmakerMatched, Socket};
use nakama_rs::web_socket::WebSocket;
use nakama_rs::web_socket_adapter::WebSocketAdapter;
use nakama_rs::*;
use tokio::sync::mpsc::channel;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    SimpleLogger::new()
        .with_level(LevelFilter::Off)
        .with_module_level("nakama_rs", LevelFilter::Trace)
        .init()
        .unwrap();

    let http_adapter = RestHttpAdapter::new("http://127.0.0.1", 7350);
    let client = DefaultClient::new(http_adapter, "defaultkey", "");
    let adapter = WebSocketAdapter::new();
    let mut web_socket = WebSocket::new(adapter);
    let web1 = web_socket.clone();
    let (mut kill_tick, mut rc_kill) = channel(1);
    let res = tokio::spawn(async move {
        loop {
            // This could also be called in a different thread than the main/game thread. The callbacks
            // will be called in the same thread, invoking e.g. `on_received_channel_message`.
            // Note that `tick` is also necessary to wake futures like `web_socket.join_chat` - it is not only necessary
            // for the callbacks.
            println!("Ticking websockets");
            web1.tick();
            if let Ok(v) = rc_kill.try_recv() {
                return v;
            }

            sleep(Duration::from_millis(500));
        }
    });
    let session = client
        .authenticate_custom(
            &Utc::now().timestamp_nanos().to_string(),
            None,
            true,
            HashMap::new(),
        )
        .await;

    let session = session.unwrap();
    web_socket
        .connect("ws://127.0.0.1:7350", &session, true, -1)
        .await;
    let mut numeric_properties = HashMap::new();
    numeric_properties.insert("mmr".to_string(), 1500.0);
    numeric_properties.insert("room_id".to_string(), 30.0);
    let mut string_properties = HashMap::new();
    string_properties.insert("battle_type".to_string(), "QuickMatch1v1".to_string());
    let ticked = web_socket
        .add_matchmaker(&Matchmaker {
            min_count: 2,
            max_count: 2,
            string_properties,
            numeric_properties,
            query: "+properties.battle_type:QuickMatch1v1 -properties.room_id:30 properties.mmr:>=1400 properties.mmr:<=1600".to_string(),
        })
        .await
        .expect("Failed to join chat");
    // let party = web_socket.create_party(true, 2).await?;
    // println!("********{:?}", party);
    // kill_tick.send(1).await;
    web_socket.on_received_matchmaker_matched(move |x| {
        block_on(kill_tick.send(x));
    });
    let data = res.await?;
    println!("********{:#?}", data);
    // web_socket.remove_matchmaker(&ticked.ticket).await;
    Ok(())
}

// "ws://127.0.0.1:7350/ws?lang=en&status=true&token=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJ1aWQiOiJmOTE5MGFlYy03NGI2LTQ0MDktOTBkYy1hNDBiNGRhZGQzMmYiLCJ1c24iOiJVYm1qd2tMWGpEIiwiZXhwIjoxNjg4OTc4MzQxfQ.-zNwQvuIcu8KphjckTmWg6d5aPMVcXsQV5KHMODYAH0"
// "ws://127.0.0.1:7350/ws?lang=en&status=true&token=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJ1aWQiOiJmOTE5MGFlYy03NGI2LTQ0MDktOTBkYy1hNDBiNGRhZGQzMmYiLCJ1c24iOiJVYm1qd2tMWGpEIiwiZXhwIjoxNjg4OTc4NDI5fQ.FIj8tu2b1pTaTjhSrkCUBje0Quv7QLkDqur4M7fa7JM"
