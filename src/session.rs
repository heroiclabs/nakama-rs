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
use nanoserde::DeJson;
use std::collections::HashMap;
use chrono::{DateTime, Utc, TimeZone, Duration};
use std::ops::Add;

#[derive(Debug, Clone)]
pub struct Session {
    inner: Arc<Mutex<Inner>>,
}

#[derive(Debug)]
struct Inner {
    auth_token: String,
    refresh_token: Option<String>,
    expire_time: DateTime<Utc>,
    refresh_expire_time: Option<DateTime<Utc>>,
    username: String,
    uid: String,
    vars: Arc<HashMap<String, String>>,
    auto_refresh: bool,
}

#[derive(Debug, DeJson)]
struct AuthTokenData {
    #[nserde(rename = "exp")]
    expire_time: u64,
    #[nserde(rename = "usn")]
    username: String,
    #[nserde(rename = "uid")]
    uid: String,
    #[nserde(rename = "vrs")]
    #[nserde(default)]
    vars: HashMap<String, String>,
}

#[derive(Debug, DeJson)]
struct RefreshTokenData {
    #[nserde(rename = "exp")]
    expire_time: u64
}

fn jwt_unpack(jwt: &str) -> Option<String> {
    let mut iter = jwt.split('.');
    iter.next();
    let payload = iter.next()?;
    let pad_length = ((payload.len() as f64 / 4.0).ceil() as usize) * 4 - payload.len();
    let payload = format!("{}{}", payload, "=".repeat(pad_length))
        .replace("-", "+")
        .replace("_", "/");
    let decoded = base64::decode(payload).ok()?;
    let utf8 = String::from_utf8(decoded).ok()?;
    Some(utf8)
}

impl Session {
    pub fn new(auth_token: &str, refresh_token: &str) -> Session {
        let auth_token_payload = jwt_unpack(auth_token).expect("Failed to parse session");
        let refresh_expire_time = jwt_unpack(refresh_token)
            .and_then(|refresh_token| {
                let data = RefreshTokenData::deserialize_json(&refresh_token).ok()?;
                Some(Utc.timestamp(data.expire_time as i64, 0))
            });

        let auth_token_data = AuthTokenData::deserialize_json(&auth_token_payload).expect("Failed to parse session");

        Session {
            inner: Arc::new(Mutex::new(Inner {
                auth_token: auth_token.to_owned(),
                refresh_token: if refresh_token.len() == 0 {
                    None
                } else {
                    Some(refresh_token.to_owned())
                },
                refresh_expire_time,
                expire_time: Utc.timestamp(auth_token_data.expire_time as i64, 0),
                username: auth_token_data.username,
                uid: auth_token_data.uid,
                vars: Arc::new(auth_token_data.vars),
                auto_refresh: true,
            })),
        }
    }

    pub fn get_auto_refresh(&self) -> bool {
        self.inner.lock().unwrap().auto_refresh
    }

    pub fn set_auto_refresh(&self, auto_refresh: bool) {
        self.inner.lock().unwrap().auto_refresh = auto_refresh;
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

    pub fn expire_time(&self) -> DateTime<Utc> {
        self.inner.lock().unwrap().expire_time.clone()
    }

    pub fn refresh_expire_time(&self) -> Option<DateTime<Utc>> {
        self.inner.lock().unwrap().refresh_expire_time.clone()
    }

    pub fn has_expired(&self, date_time: DateTime<Utc>) -> bool {
        self.inner.lock().unwrap().expire_time.le(&date_time)
    }

    pub fn is_expired(&self) -> bool {
        self.has_expired(Utc::now())
    }

    pub fn will_expire_soon(&self) -> bool {
        self.has_expired(Utc::now().add(Duration::minutes(5)))
    }

    pub fn has_refresh_expired(&self, date_time: DateTime<Utc>) -> bool {
        self.inner.lock().unwrap().refresh_expire_time.map_or(false, |time| time.le(&date_time))
    }

    pub fn is_refresh_expired(&self) -> bool {
        self.has_refresh_expired(Utc::now())
    }

    pub fn username(&self) -> String {
        self.inner.lock().unwrap().username.clone()
    }

    pub fn user_id(&self) -> String {
        self.inner.lock().unwrap().uid.clone()
    }

    pub fn vars(&self) -> Arc<HashMap<String, String>> {
        self.inner.lock().unwrap().vars.clone()
    }
}

#[cfg(test)]
mod test {
    use crate::session::{jwt_unpack, Session};
    use chrono::{Utc, TimeZone};
    use std::sync::Arc;

    #[test]
    fn test_session() {
        let auth_token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjE2MjM5NjE2NzQsInVzbiI6IlVzZXJuYW1lIiwidWlkIjoiMTIzNDU2NzgiLCJ2cnMiOnsiaGVsbG8iOiJ3b3JsZCIsIm1vcmUiOiJkYXRhIn19._QvIe6v63HduVk9Gf4RIWUPuGsQBJam2WXbms6P-dXg";
        let refresh_token = "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJleHAiOjE2MjM5ODE2NzR9.KnTXR5Gypq92qclwvxcgUSHsWkiJrQsAM2Tt6jmGXvs";

        let session = Session::new(auth_token, refresh_token);
        assert_eq!(session.username(), "Username".to_owned());
        assert_eq!(session.user_id(), "12345678".to_owned());
        assert_eq!(session.vars(), Arc::new([("hello".to_owned(), "world".to_owned()), ("more".to_owned(), "data".to_owned())].iter().cloned().collect()));
        assert_eq!(session.is_expired(), true);
        assert_eq!(session.has_expired(Utc.timestamp(1623961673, 0)), false);
        assert_eq!(session.has_refresh_expired(Utc.timestamp(1623981674, 0)), true);
        assert_eq!(session.has_refresh_expired(Utc.timestamp(1623981673, 0)), false);
    }

    #[test]
    fn test_jwt_unpack() {
        let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRGUiLCJpYXQiOjE1MTYyMzkwMjJ9.1bxegY2QzMgmi4VjfhxtumdUCSGl8ohztW8878wScAA";
        let result = jwt_unpack(token);
        println!("{:?}", result)
    }

    #[test]
    fn test_jwt_unpack2() {
        let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRGVlIiwiaWF0IjoxNTE2MjM5MDIyfQ.wiL_VstOtjUkVMqecTNFVzkBtSYS5Er3i4DCC2vxtEQ";
        let result = jwt_unpack(token);
        println!("{:?}", result)
    }
}