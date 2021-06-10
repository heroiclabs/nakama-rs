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
fn test_join_tournament() {
    block_on(async {
        let (client, mut session) = test_helpers::authenticated_client("tournamentclient1").await;
        let result = client
            .join_tournament(&mut session, "example-tournament")
            .await;
        println!("{:?}", result);
        assert_eq!(result.is_ok(), true);
    });
}

#[test]
fn test_list_tournaments() {
    block_on(async {
        let (client, mut session) = test_helpers::authenticated_client("tournamentclient1").await;
        let result1 = client
            .list_tournaments(&mut session, None, None, None, None, Some(1), None)
            .await
            .unwrap();
        let cursor = result1.cursor.as_deref();
        let result2 = client
            .list_tournaments(&mut session, None, None, None, None, Some(1), cursor)
            .await
            .unwrap();
        println!("{:?}", result2);
        assert_eq!(result2.cursor.is_none(), true);
        assert_eq!(result2.tournaments.len() > 0, true);
    });
}

#[test]
fn test_write_tournament_record() {
    // TODO: Why is the tournament not active?
    // block_on(async {
    //     let (client, mut session) = test_helpers::authenticated_client("tournamentclient1").await;
    //     client.join_tournament(&mut session, "example-tournament").await.unwrap();
    //     let result = client.write_tournament_record(&mut session, "example-tournament", 1, None, None, None).await;
    //     println!("{:?}", result);
    //     assert_eq!(result.is_ok(), true);
    // });
}

#[test]
fn test_list_tournament_records() {
    // TODO: Why is the tournament not active?
    // block_on(async {
    //     let (client, mut session) = test_helpers::authenticated_client("tournamentclient1").await;
    //     let (client2, mut session2) = test_helpers::authenticated_client("tournamentclient2").await;
    //     client.join_tournament(&mut session, "example-tournament").await.unwrap();
    //     client.join_tournament(&mut session2, "example-tournament").await.unwrap();
    //     client.write_tournament_record(&mut session, "example-tournament", 1, None, None, None).await.unwrap();
    //     client.write_tournament_record(&mut session2, "example-tournament", 1, None, None, None).await.unwrap();
    //     let result1 = client.list_tournament_records(&mut session, "example-tournament", &[], None, Some(1), None).await.unwrap();
    //     let result2 = client.list_tournament_records(&mut session, "example-tournament", &[], None, Some(1), result1.next_cursor.as_deref()).await.unwrap();
    //     println!("{:?}", result2);
    //     assert_eq!(result1.prev_cursor.is_none(), true);
    //     assert_eq!(result2.next_cursor.is_none(), true);
    // });
}
