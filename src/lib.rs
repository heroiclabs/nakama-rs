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
pub mod test_helpers;
pub mod web_socket;
pub mod web_socket_adapter;

pub mod api {
    pub use super::api_gen::*;
}
