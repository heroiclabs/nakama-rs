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
use nakama_rs::api::ApiOverrideOperator;
use nakama_rs::client::Client;
use nakama_rs::test_helpers;

#[test]
fn test_write_leaderboard_record() {
    block_on(async {
        let (client, mut session) = test_helpers::authenticated_client("leaderboardclient1").await;
        let result = client
            .write_leaderboard_record(&mut session, "wins", 1, None, None, None)
            .await;
        println!("{:?}", result);
        assert_eq!(result.is_ok(), true);
    });
}

#[test]
fn test_write_leaderboard_record_subscore_and_override_operator() {
    block_on(async {
        let (client, mut session) = test_helpers::authenticated_client("leaderboardclient1").await;
        let result = client
            .write_leaderboard_record(
                &mut session,
                "wins",
                1,
                Some(50),
                Some(ApiOverrideOperator::SET),
                None,
            )
            .await;
        println!("{:?}", result);
        assert_eq!(result.is_ok(), true);
        assert_eq!(result.unwrap().subscore, Some("50".to_owned()));
    });
}

#[test]
fn test_delete_leaderboard_record() {
    block_on(async {
        let (client, mut session) = test_helpers::authenticated_client("leaderboardclient1").await;
        client
            .write_leaderboard_record(
                &mut session,
                "wins",
                1,
                Some(50),
                Some(ApiOverrideOperator::SET),
                None,
            )
            .await
            .expect("Failed to write leaderboard");
        let result = client.delete_leaderboard_record(&mut session, "wins").await;
        println!("{:?}", result);
        assert_eq!(result.is_ok(), true);
    });
}

#[test]
fn test_list_leaderboard_records() {
    block_on(async {
        let (client, mut session) = test_helpers::authenticated_client("leaderboardclient1").await;
        let (_, mut session2) = test_helpers::authenticated_client("leaderboardclient2").await;
        client
            .write_leaderboard_record(
                &mut session,
                "wins",
                1,
                Some(50),
                Some(ApiOverrideOperator::SET),
                None,
            )
            .await
            .expect("Failed to write leaderboard record");
        client
            .write_leaderboard_record(
                &mut session2,
                "wins",
                2,
                Some(50),
                Some(ApiOverrideOperator::SET),
                None,
            )
            .await
            .expect("Failed to write leaderboard record");
        let result1 = client
            .list_leaderboard_records(&mut session, "wins", &[], None, Some(1), None)
            .await
            .unwrap();
        let result2 = client
            .list_leaderboard_records(
                &mut session,
                "wins",
                &[],
                None,
                Some(100),
                Some(&result1.next_cursor),
            )
            .await
            .unwrap();
        println!("{:?}", result2);
        assert_eq!(result1.prev_cursor.is_empty(), true);
        assert_eq!(result2.records.len() >= 1, true);
        assert_eq!(result2.next_cursor.is_empty(), true);
    });
}

#[test]
fn test_list_leaderboard_records_around_owner() {
    block_on(async {
        let (client, mut session) = test_helpers::authenticated_client("leaderboardclient1").await;
        let (_, mut session2) = test_helpers::authenticated_client("leaderboardclient2").await;
        client
            .write_leaderboard_record(
                &mut session,
                "wins",
                1,
                Some(50),
                Some(ApiOverrideOperator::SET),
                None,
            )
            .await
            .expect("Failed to write leaderboard record");
        client
            .write_leaderboard_record(
                &mut session2,
                "wins",
                2,
                Some(50),
                Some(ApiOverrideOperator::SET),
                None,
            )
            .await
            .expect("Failed to write leaderboard record");
        let user_id = client.get_account(&mut session).await.unwrap().user.id;
        let result = client
            .list_leaderboard_records_around_owner(&mut session, "wins", &user_id, None, Some(1))
            .await
            .unwrap();
        println!("{:?}", result);
        assert_eq!(result.records.len() >= 1, true);
    });
}
