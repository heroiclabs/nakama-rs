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

use crate::api::RestRequest;
use async_trait::async_trait;
use nanoserde::DeJson;
use std::error::Error;

pub trait ClientAdapterError: Error {
   fn is_server_error(&self) -> bool;
   fn is_client_error(&self) -> bool;
}

#[async_trait]
pub trait ClientAdapter {
    type Error: ClientAdapterError;
    async fn send<T: DeJson + Send>(&self, request: RestRequest<T>) -> Result<T, Self::Error>;
}
