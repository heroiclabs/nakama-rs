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
fn test_authenticate_device_id_too_short() {
    let client = DefaultClient::new_with_adapter_and_defaults();

    let result = block_on(async {
        let vars = [("Hello", "World!")].iter().cloned().collect();
        client
            .authenticate_device("too_short", None, true, vars)
            .await
    });

    println!("Result: {:?}", result);
    assert_eq!(result.is_err(), true)
}

#[test]
fn test_authenticate_device_id() {
    let client = DefaultClient::new_with_adapter_and_defaults();

    let result = block_on(async {
        let mut vars = HashMap::new();
        vars.insert("Hello", "World!");
        client
            .authenticate_device("long_enough_device_id", None, true, vars)
            .await
    });

    println!("Result: {:?}", result);
    assert_eq!(result.is_ok(), true)
}

#[test]
fn test_authenticating_with_unknown_credentials() {
    let client = DefaultClient::new_with_adapter_and_defaults();
    let result = block_on(async {
        client
            .authenticate_email(
                "test@unknown.com",
                "userspassword",
                None,
                false,
                HashMap::new(),
            )
            .await
    });

    println!("Result: {:?}", result);
    assert_eq!(result.is_err(), true)
}

#[test]
fn test_link_email() {
    let client = DefaultClient::new_with_adapter_and_defaults();
    let result = block_on(async {
        let session = client
            .authenticate_device("usersdeviceid", None, true, HashMap::new())
            .await?;

        client
            .link_email(&session, "test@user.com", "userspassword")
            .await?;

        client
            .authenticate_email(
                "test@user.com",
                "userspassword",
                None,
                false,
                HashMap::new(),
            )
            .await
    });

    println!("Session: {:?}", result);
    assert_eq!(result.is_ok(), true)
}

#[test]
fn test_unlink_email() {
    let client = DefaultClient::new_with_adapter_and_defaults();
    let result = block_on(async {
        let session = client
            .authenticate_device("usersdeviceid", None, true, HashMap::new())
            .await?;

        client
            .link_email(&session, "test@user.com", "userspassword")
            .await?;
        client
            .unlink_email(&session, "test@user.com", "userspassword")
            .await?;

        client
            .authenticate_email(
                "test@user.com",
                "userspassword",
                None,
                false,
                HashMap::new(),
            )
            .await
    });

    println!("Result: {:?}", result);
    assert_eq!(result.is_err(), true)
}
