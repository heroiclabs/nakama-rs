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
use std::error::Error;
use std::fmt::{Display, Formatter};

use crate::api;
use crate::client_adapter::ClientAdapter;
use async_trait::async_trait;
use isahc::prelude::*;
use nanoserde::{DeJson, DeJsonErr};
use std::io;

#[derive(Debug)]
pub enum RestHttpError {
    HttpError(isahc::Error),
    IoError(io::Error),
    JsonError(DeJsonErr),
    ClientError(u16, String),
    ServerError(u16, String),
    OtherError(String),
}

impl Display for RestHttpError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

impl Error for RestHttpError {}

#[derive(Clone)]
pub struct RestHttpAdapter {
    server: String,
    port: u32,
}

impl RestHttpAdapter {
    pub fn new(server: &str, port: u32) -> RestHttpAdapter {
        RestHttpAdapter {
            server: server.to_owned(),
            port,
        }
    }
}

#[async_trait]
impl ClientAdapter for RestHttpAdapter {
    type Error = RestHttpError;
    async fn send<T: DeJson + Send>(&self, request: RestRequest<T>) -> Result<T, RestHttpError> {
        let auth_header = match request.authentication {
            api::Authentication::Basic { username, password } => {
                format!(
                    "Basic {}",
                    base64::encode(&format!("{}:{}", username, password))
                )
            }
            api::Authentication::Bearer { token } => {
                format!("Bearer {}", token)
            }
        };

        let url = format!(
            "{}:{}{}?{}",
            self.server, self.port, request.urlpath, request.query_params
        );

        let client = isahc::HttpClientBuilder::new()
            .default_header("Authorization", &auth_header)
            .build()
            .map_err(|err| RestHttpError::HttpError(err))?;

        let mut response = match request.method {
            api::Method::Post => client.post_async(&url, request.body).await,
            api::Method::Put => client.put_async(&url, request.body).await,
            api::Method::Get => client.get_async(&url).await,
            api::Method::Delete => client.delete_async(&url).await,
        }
        .map_err(|err| RestHttpError::HttpError(err))?;

        match response.status().as_u16() {
            status if status >= 200 && status < 300 => {
                let response = response
                    .text()
                    .await
                    .map_err(|err| RestHttpError::IoError(err))?;

                nanoserde::DeJson::deserialize_json(&response)
                    .map_err(|json_err| RestHttpError::JsonError(json_err))
            }
            status if status >= 400 && status < 500 => {
                let response = response
                    .text()
                    .await
                    .map_err(|err| RestHttpError::IoError(err))?;
                Err(RestHttpError::ClientError(status, response))
            }
            status if status >= 500 => {
                let response = response
                    .text()
                    .await
                    .map_err(|err| RestHttpError::IoError(err))?;
                Err(RestHttpError::ServerError(status, response))
            }
            _ => Err(RestHttpError::OtherError("Unknown status".to_owned())),
        }
    }
}
