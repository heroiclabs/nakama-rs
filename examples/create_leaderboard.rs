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
use nakama_rs::api::ApiOverrideOperator;
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
    let session = client
        .authenticate_custom("00000000", None, true, Default::default())
        .await?;
    println!("{:#?}", session);
    let data = client
        .create_leaderboard(&session, ApiOverrideOperator::SET)
        .await?;
    println!("{:?}", data);
    Ok(())
}

// curl 'http://127.0.0.1:7351/v2/console/api/endpoints/rpc/clientrpc.create_leaderboard' \
// -H 'Accept: application/json, text/plain, */*' \
//   -H 'Accept-Language: zh-CN,zh;q=0.9,en-US;q=0.8,en;q=0.7' \
//   -H 'Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJ1aWQiOiJkYzUwZjcyMy1lZjZmLTQ1N2ItOTM0MS0zMGM0YWI1NDcwMjUiLCJ1c24iOiJ6YXR5Vk5VdWlBIiwiZXhwIjoxNjkwNDI3NDU4fQ.G3VjQtCB4e2_vKP9i28kupcPHdgzPBcNQU6D65kutjE' \
//   -H 'Connection: keep-alive' \
//   -H 'Content-Type: application/json' \
//   -H 'Cookie: ajs_anonymous_id=a9b3295b-da65-4d84-9bab-d8c4acef6341' \
//   -H 'Origin: http://127.0.0.1:7351' \
//   -H 'Referer: http://127.0.0.1:7351/' \
//   -H 'Sec-Fetch-Dest: empty' \
//   -H 'Sec-Fetch-Mode: cors' \
//   -H 'Sec-Fetch-Site: same-origin' \
//   -H 'User-Agent: Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/115.0.0.0 Safari/537.36' \
//   -H 'sec-ch-ua: "Not/A)Brand";v="99", "Google Chrome";v="115", "Chromium";v="115"' \
//   -H 'sec-ch-ua-mobile: ?0' \
//   -H 'sec-ch-ua-platform: "macOS"' \
//   --data-raw '{"user_id":"","body":"{\"operator\":\"set\"}"}' \
//   --compressed
