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

use crate::api::{ApiAccount, ApiGroup};
pub use crate::client::Client;
use crate::default_client::{DefaultClient, DefaultClientError};
use crate::http_adapter::RestHttpAdapter;
use crate::session::Session;
pub use crate::socket::Socket;
use crate::web_socket::WebSocket;
use crate::web_socket_adapter::WebSocketAdapter;
use core::time::Duration;
use futures::executor::block_on;
use std::future::Future;
use std::thread::{sleep, spawn};

pub fn run_in_example<
    T,
    F: Future<Output = Result<T, DefaultClientError<RestHttpAdapter>>>,
    C: Fn(DefaultClient<RestHttpAdapter>, Session) -> F,
>(
    f: C,
) {
    let client = DefaultClient::new_with_adapter_and_defaults();
    block_on(async {
        let session = client
            .authenticate_device("exampletestid", None, true, HashMap::new())
            .await
            .expect("Failed to authenticate user");
        f(client, session).await.expect("Test failed");
    })
}

pub fn run_in_socket_example<
    T,
    F: Future<Output = Result<T, DefaultClientError<RestHttpAdapter>>>,
    C: Fn(DefaultClient<RestHttpAdapter>, Session, WebSocket<WebSocketAdapter>) -> F,
>(
    f: C,
) {
    let client = DefaultClient::new_with_adapter_and_defaults();
    let socket = WebSocket::new_with_adapter();
    block_on(async {
        let session = client
            .authenticate_device("exampletestid", None, true, HashMap::new())
            .await
            .expect("Failed to authenticate user");
        f(client, session, socket).await.expect("Test failed");
    })
}

pub async fn remove_group_if_exists<C: Client>(client: &C, session: &Session, group_name: &str) {
    let groups = client
        .list_groups(&session, Some(group_name), None, None)
        .await;
    if let Ok(groups) = groups {
        if groups.groups.len() > 0 {
            client
                .delete_group(&session, &groups.groups[0].id)
                .await
                .unwrap();
        }
    }
}

pub async fn re_create_group<C: Client>(
    client: &C,
    session: &Session,
    group_name: &str,
) -> ApiGroup {
    remove_group_if_exists(client, &session, group_name).await;
    client
        .create_group(&session, group_name, None, None, None, Some(true), None)
        .await
        .unwrap()
}

pub async fn authenticated_client(id_one: &str) -> (DefaultClient<RestHttpAdapter>, Session) {
    let client = DefaultClient::new_with_adapter_and_defaults();
    let session = client
        .authenticate_device(id_one, Some(id_one.clone()), true, HashMap::new())
        .await
        .unwrap();

    return (client, session);
}

pub async fn clients_with_users(
    id_one: &str,
    id_two: &str,
    id_three: &str,
) -> (DefaultClient<RestHttpAdapter>, Session, Session, Session) {
    let client = DefaultClient::new_with_adapter_and_defaults();
    let session = client
        .authenticate_device(id_one, Some(id_one.clone()), true, HashMap::new())
        .await
        .unwrap();
    let session2 = client
        .authenticate_device(id_two, Some(id_two.clone()), true, HashMap::new())
        .await
        .unwrap();
    let session3 = client
        .authenticate_device(id_three, Some(id_three.clone()), true, HashMap::new())
        .await
        .unwrap();

    return (client, session, session2, session3);
}

pub async fn sockets_with_users(
    id_one: &str,
    id_two: &str,
) -> (
    WebSocket<WebSocketAdapter>,
    WebSocket<WebSocketAdapter>,
    ApiAccount,
    ApiAccount,
) {
    let client = DefaultClient::new_with_adapter_and_defaults();
    let socket = WebSocket::new_with_adapter();
    let socket2 = WebSocket::new_with_adapter();
    tick_socket(&socket);
    tick_socket(&socket2);

    let session = client
        .authenticate_device(id_one, Some(id_one.clone()), true, HashMap::new())
        .await
        .unwrap();
    let session2 = client
        .authenticate_device(id_two, Some(id_two.clone()), true, HashMap::new())
        .await
        .unwrap();

    let account1 = client.get_account(&session).await.unwrap();
    let account2 = client.get_account(&session2).await.unwrap();

    socket.connect(&session, true, -1).await;
    socket2.connect(&session2, true, -1).await;

    (socket, socket2, account1, account2)
}

pub fn tick_socket(socket: &WebSocket<WebSocketAdapter>) {
    spawn({
        let socket = socket.clone();
        move || loop {
            socket.tick();
            sleep(Duration::from_millis(16));
        }
    });
}
