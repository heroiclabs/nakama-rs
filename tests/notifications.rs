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
use nakama_rs::test_helpers;

#[test]
fn test_list_notifications() {
    block_on(async {
        let (client, mut session) = test_helpers::authenticated_client("notificationsuserid").await;
        client
            .rpc(&mut session, "echo", Some("Hello World!"))
            .await
            .expect("Failed to call echo rpc");
        client
            .rpc(&mut session, "echo", Some("Hello World Two!"))
            .await
            .expect("Failed to call echo rpc");

        let result = client
            .list_notifications(&mut session, Some(1), None)
            .await
            .expect("Failed to list notifications");
        let result = client
            .list_notifications(&mut session, Some(1), Some(&result.cacheable_cursor))
            .await;
        assert_eq!(result.is_ok(), true);
        println!("{:?}", result);
    });
}

#[test]
fn test_delete_notifications() {
    block_on(async {
        let (client, mut session) = test_helpers::authenticated_client("notificationsuserid").await;
        client
            .rpc(&mut session, "echo", Some("Hello World!"))
            .await
            .expect("Failed to call echo rpc");
        let notifications = client
            .list_notifications(&mut session, Some(1), None)
            .await
            .expect("Failed to fetch notifications");
        let id = &notifications.notifications[0].id;

        let result = client.delete_notifications(&mut session, &[id]).await;
        assert_eq!(result.is_ok(), true);
        println!("{:?}", result);
    });
}

#[test]
fn test_delete_all_notifications() {
    block_on(async {
        let (client, mut session) = test_helpers::authenticated_client("notificationsuserid").await;

        loop {
            let notifications = client
                .list_notifications(&mut session, Some(5), None)
                .await
                .expect("Failed to fetch notifications");

            if notifications.notifications.len() == 0 {
                return;
            }

            let ids: Vec<&str> = notifications
                .notifications
                .iter()
                .map(|notification| &*notification.id)
                .collect();

            client
                .delete_notifications(&mut session, ids.as_ref())
                .await
                .expect("Failed to delete notifications");
        }
    });
}
