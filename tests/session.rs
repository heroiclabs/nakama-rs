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

use futures::executor::block_on;
use nakama_rs::client::Client;
use nakama_rs::default_client::DefaultClient;
use std::collections::HashMap;
use std::thread::sleep;
use std::time::Duration;

#[test]
fn test_session_variables() {
    let client = DefaultClient::new_with_adapter_and_defaults();

    block_on(async {
        let mut vars = HashMap::new();
        vars.insert("ident", "hidden");
        let session = client
            .authenticate_device("somenewdeviceid", None, true, vars)
            .await
            .expect("Failed to authenticate");

        let session_vars = session.vars();
        assert_eq!(session_vars.get("ident"), Some(&"hidden".to_owned()));
        assert_eq!(session.is_expired(), false);
        // Session in development mode will expire in 60 seconds
        assert_eq!(session.will_expire_soon(), true);
    });
}

#[test]
fn test_session_refresh() {
    block_on(async {
        let client = DefaultClient::new_with_adapter_and_defaults();
        let session = client
            .authenticate_device("somenewdeviceid", None, true, HashMap::new())
            .await
            .expect("Failed to authenticate");
        let auth_token = session.get_auth_token().clone();
        sleep(Duration::from_secs(1));
        // The default session expiration (60s) will cause a refresh
        client
            .get_account(&session)
            .await
            .expect("failed to get account");
        assert_ne!(auth_token, session.get_auth_token());
    })
}
