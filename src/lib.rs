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

//! # Rust client guide
//!
//! !!! Tip "Contribute"
//!     The Rust client is <a href="https://!github.com/heroiclabs/nakama-rs" target="\_blank">open source</a> on GitHub. Report issues and contribute code to help us improve it.
//!
//! ## Setup
//!
//! Add `nakama-rs` as a dependency to `Cargo.toml`.
//! ```Cargo.toml
//! [dependencies]
//! nakama-rs = "0.2.0"
//! ```
//!
//! The client object is used to interact with the server.
//!
//! ```
//! use nakama_rs::DefaultClient;
//! let client = DefaultClient::new_with_adapter("http://127.0.0.1", 7350, "defaultkey", "");
//! ```
//!
//! ## Authenticate
//!
//! With the client you can authenticate against the server. You can register or login a [user](user-accounts.md) with one of the [authenticate options](authentication.md).
//!
//! To authenticate you should follow our recommended pattern in your client code:
//!
//! &nbsp;&nbsp; 1\. Build an instance of the client.
//!
//! ```
//! # use nakama_rs::DefaultClient;
//! let client = DefaultClient::new_with_adapter("http://127.0.0.1", 7350, "defaultkey", "");
//! ```
//!
//! &nbsp;&nbsp; 2\. Authenticate a user wi.
//!
//! ```
//! # use nakama_rs::{DefaultClient, Client};
//! # use std::collections::HashMap;
//! # let client = DefaultClient::new_with_adapter("http://127.0.0.1", 7350, "defaultkey", "");
//! let email = "hello@example.com";
//! let password = "somesupersecretpassword";
//! let session = client.authenticate_email(email, password, None, true, HashMap::new()).await;
//! ```
//!
//! In the code above we use `authenticate_email` but for other authentication options have a look at the [code examples](authentication.md#authenticate). This [full example](#full-example) covers all our recommended steps.
//!
//! ## Sessions
//! TODO
//!
//! When authenticated the server responds with an auth token (JWT) which contains useful properties and gets deserialized into a `Session` object.
//!
//! ```csharp
//! Debug.Log(session.AuthToken); //! raw JWT token
//! Debug.LogFormat("Session user id: '{0}'", session.UserId);
//! Debug.LogFormat("Session user username: '{0}'", session.Username);
//! Debug.LogFormat("Session has expired: {0}", session.IsExpired);
//! Debug.LogFormat("Session expires at: {0}", session.ExpireTime); //! in seconds.
//! ```
//!
//! It is recommended to store the auth token from the session and check at startup if it has expired. If the token has expired you must reauthenticate. The expiry time of the token can be changed as a [setting](install-configuration.md#common-properties) in the server.
//!
//! ```csharp
//! const string prefKeyName = "nakama.session";
//! ISession session;
//! var authToken = PlayerPrefs.GetString(prefKeyName);
//! if (string.IsNullOrEmpty(authToken) || (session = Session.Restore(authToken)).IsExpired)
//! {
//!     Debug.Log("Session has expired. Must reauthenticate!");
//! };
//! Debug.Log(session);
//! ```
//!
//! ## Send requests
//!
//! The client includes lots of builtin APIs for various features of the game server. These are accessed with async methods. It can also call custom logic through RPC functions on the server.
//!
//! All requests are sent with a session object which authorizes the client.
//!
//! ```
//! # use nakama_rs::test_helpers::run_in_example;
//! # use nakama_rs::Client;
//! # run_in_example(async |client| {
//!     let account = client.get_account(&session).await?;
//!     println!("User id: '{}'", account.user.id);
//!     println!("User username: '{}'", account.user.username);
//!     println!("Account virtual wallet: '{}'", account.wallet);
//! # });
//! ```
//!
//! The other sections of the documentation include more code examples on the client.
//!
//! ## Socket messages
//!
//! The client can create one or more sockets with the server. Each socket can have it's own event listeners registered for responses received from the server.
//!
//! ```
//! # use nakama_rs::test_helpers::run_in_example;
//! # use nakama_rs::{WebSocket, Socket};
//! # run_in_example(async |_client, session| {
//!     let mut socket = WebSocket::new_with_adapter();
//!     socket.on_connected(|| println!("Socket connected."));
//!     socket.on_closed(|| println!("Socket closed."));
//!     socket.connect(&session).await;
//! # });
//! ```
//!
//! You can connect to the server over a realtime socket connection to send and receive [chat messages](social-realtime-chat.md), get [notifications](social-in-app-notifications.md), and [matchmake](gameplay-matchmaker.md) into a [multiplayer match](gameplay-multiplayer-realtime.md). You can also execute remote code on the server via [RPC](runtime-code-basics.md).
//!
//! To join a chat channel and receive messages:
//!
//! ```
//! # use nakama_rs::test_helpers::{run_in_example, run_in_socket_example};
//! # use nakama_rs::Socket;
//! # run_in_socket_example(|client, session, mut socket| {
//!     let room_name = "Heroes";
//!     socket.on_received_channel_message(|message| {
//!         println!("Message has channel id: {}", message.channel_id);
//!         println!("Message content: {}", message.content);
//!     });
//!     let channel = socket.join_chat(room_name, 1, true, true).await?;
//!     let send_ack = socket.write_chat_message(&channel.id, r#"{ "text": "Hello World!" }"#).await?;
//!     println!("{:?}", send_ack);
//! # });
//! ```
//!
//! There are more examples for chat channels [here](social-realtime-chat.md).
//!
//! ## Handle events
//!
//! A socket object has event handlers which are called on various messages received from the server.
//!
//! ```
//! # use nakama_rs::test_helpers::run_in_socket_example;
//! use nakama_rs::Socket;
//! # run_in_socket_example(|_,_,mut socket| {
//!     socket.on_received_channel_presence(|mut presence_events| {
//!         presence_events.leaves.drain(..).for_each(|left| {
//!             println!("User '{}' left.", left.username) ;
//!         });
//!         presence_events.joins.drain(..).for_each(|joined| {
//!             println!("User '{}' joined.", joined.username) ;
//!         });
//!     })
//! # });
//! ```
//!
//! Event handlers only need to be implemented for the features you want to use.
//!
//! | Callbacks | Description |
//! | --------- | ----------- |
//! | on_connected | Receive an event when the socket connects. |
//! | on_closed | Receives an event for when the client is disconnected from the server. |
//! | on_received_error | Receives events about server errors. |
//! | on_received_notifiation | Receives live [in-app notifications](social-in-app-notifications.md) sent from the server. |
//! | on_received_channel_message | Receives [realtime chat](social-realtime-chat.md) messages sent by other users. |
//! | on_received_channel_presence | Received join and leave events within [chat](social-realtime-chat.md). |
//! | on_received_match_state | Receives [realtime multiplayer](gameplay-multiplayer-realtime.md) match data. |
//! | on_received_match_presence | Receives join and leave events within [realtime multiplayer](gameplay-multiplayer-realtime.md). |
//! | on_received_matchmaker_matched | Received when the [matchmaker](gameplay-matchmaker.md) has found a suitable match. |
//! | on_received_status_presence | Receives status updates when subscribed to a user [status feed](social-status.md). |
//! | on_received_stream_presence | Receives [stream](advanced-streams.md) join and leave event. |
//! | on_received_stream_state | Receives [stream](advanced-streams.md) data sent by the server. |
//!
//! ## Logs and errors
//!
//! The [server](install-configuration.md#log) and the client can generate logs which are helpful to debug code. The Nakama Rust SDK uses the
//! [log crate](https://crates.io/crates/log). The executable can set up any logger implementation compatible with the `log` facade. See the library for
//! a list of possible crates. As an example, the [`simple_logger`](https://crates.io/crates/simple_logger) is used.
//!
//! ```
//! # use simple_logger::SimpleLogger;
//! # use log::LevelFilter;
//! SimpleLogger::new()
//!     .with_level(LevelFilter::Off)
//!     .with_module_level("nakama_rs", LevelFilter::Trace)
//!     .init()
//!     .unwrap();
//! ```
//!

mod api_gen;
mod api_gen_enum;

pub mod client;
pub mod client_adapter;
pub mod config;
pub mod default_client;
pub mod http_adapter;
pub mod matchmaker;
pub mod session;
pub mod socket;
pub mod socket_adapter;
#[cfg(feature = "test")]
pub mod test_helpers;
pub mod web_socket;
pub mod web_socket_adapter;

pub use client::Client;
pub use default_client::DefaultClient;

pub use socket::Socket;
pub use web_socket::WebSocket;

pub mod api {
    pub use super::api_gen::*;
}
