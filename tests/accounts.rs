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

#[test]
fn test_get_account() {
    let client = DefaultClient::new_with_adapter_and_defaults();

    let result = block_on(async {
        let mut session = client
            .authenticate_device("somedeviceid", Some("TestUser"), true, HashMap::new())
            .await?;

        client.get_account(&session).await
    });

    println!("Result: {:?}", result);
    let account = result.unwrap();
    assert_eq!(account.devices[0].id, "somedeviceid",);
    assert_eq!(account.user.username, "TestUser");
}

#[test]
fn test_update_account() {
    let client = DefaultClient::new_with_adapter_and_defaults();

    let result = block_on(async {
        let mut session = client
            .authenticate_device("somedeviceid", Some("TestUser"), true, HashMap::new())
            .await?;

        client
            .update_account(
                &session,
                "TestUser",
                Some("DisplayName"),
                Some("url://avatar"),
                Some("de"),
                Some("Austria"),
                Some("Europe/Vienna"),
            )
            .await?;

        client.get_account(&session).await
    });

    println!("Result: {:?}", result);
    let account = result.unwrap();
    assert_eq!(account.user.username, "TestUser");
    assert_eq!(account.user.display_name, "DisplayName");
    assert_eq!(account.user.avatar_url, "url://avatar");
    assert_eq!(account.user.lang_tag, "de");
    assert_eq!(account.user.location, "Austria");
    assert_eq!(account.user.timezone, "Europe/Vienna");
}
