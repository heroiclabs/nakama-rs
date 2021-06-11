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

use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct Session {
    inner: Arc<Mutex<Inner>>,
}

#[derive(Debug)]
struct Inner {
    auth_token: String,
    refresh_token: Option<String>,
}

impl Session {
    pub fn new(auth_token: &str, refresh_token: &str) -> Session {
        Session {
            inner: Arc::new(Mutex::new(Inner {
                auth_token: auth_token.to_owned(),
                refresh_token: if refresh_token.len() == 0 {
                    None
                } else {
                    Some(refresh_token.to_owned())
                },
            })),
        }
    }

    pub fn replace(&self, auth_token: &str, refresh_token: &str) {
        let mut inner = self.inner.lock().unwrap();
        inner.auth_token = auth_token.to_owned();
        inner.refresh_token = if refresh_token.len() == 0 {
            None
        } else {
            Some(refresh_token.to_owned())
        };
    }

    pub fn get_auth_token(&self) -> String {
        self.inner.lock().unwrap().auth_token.clone()
    }

    pub fn get_refresh_token(&self) -> Option<String> {
        self.inner.lock().unwrap().refresh_token.clone()
    }
}
