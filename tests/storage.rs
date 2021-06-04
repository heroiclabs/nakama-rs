use futures::executor::block_on;
use nakama_rs::api::{ApiDeleteStorageObjectId, ApiReadStorageObjectId, ApiWriteStorageObject};
use nakama_rs::client::Client;
use nakama_rs::default_client::DefaultClient;
use nakama_rs::http_adapter::RestHttpAdapter;
use nakama_rs::session::Session;
use nakama_rs::test_helpers;

async fn client_with_storage_object() -> (DefaultClient<RestHttpAdapter>, Session) {
    let (client, mut session) = test_helpers::authenticated_client("storageclientid").await;
    client
        .write_storage_objects(
            &mut session,
            &[
                ApiWriteStorageObject {
                    collection: "Cards".to_owned(),
                    key: "card1".to_owned(),
                    permission_read: 2,
                    permission_write: 1,
                    value: r#"{"value":"A powerful card"}"#.to_owned(),
                    version: "".to_owned(),
                },
                ApiWriteStorageObject {
                    collection: "Cards".to_owned(),
                    key: "card2".to_owned(),
                    permission_read: 2,
                    permission_write: 1,
                    value: r#"{"value":"A weak card"}"#.to_owned(),
                    version: "".to_owned(),
                },
            ],
        )
        .await
        .unwrap();

    (client, session)
}

#[test]
fn test_write_storage() {
    block_on(async {
        let (client, mut session) = test_helpers::authenticated_client("storageclientid").await;
        let result = client
            .write_storage_objects(
                &mut session,
                &[
                    ApiWriteStorageObject {
                        collection: "Cards".to_owned(),
                        key: "card1".to_owned(),
                        permission_read: 2,
                        permission_write: 1,
                        value: r#"{"value":"A powerful card"}"#.to_owned(),
                        version: "".to_owned(),
                    },
                    ApiWriteStorageObject {
                        collection: "Cards".to_owned(),
                        key: "card2".to_owned(),
                        permission_read: 2,
                        permission_write: 1,
                        value: r#"{"value":"A weak card"}"#.to_owned(),
                        version: "".to_owned(),
                    },
                ],
            )
            .await;

        println!("{:?}", result);
        assert_eq!(result.is_ok(), true);
    });
}

#[test]
fn test_read_storage() {
    block_on(async {
        let (client, mut session) = client_with_storage_object().await;
        let user_id = client.get_account(&mut session).await.unwrap().user.id;

        let result = client
            .read_storage_objects(
                &mut session,
                &[
                    ApiReadStorageObjectId {
                        collection: "Cards".to_owned(),
                        key: "card1".to_owned(),
                        user_id: user_id.clone(),
                    },
                    ApiReadStorageObjectId {
                        collection: "Cards".to_owned(),
                        key: "card2".to_owned(),
                        user_id,
                    },
                ],
            )
            .await;

        println!("{:?}", result);
        assert_eq!(result.is_ok(), true);
    });
}

#[test]
fn test_delete_storage() {
    block_on(async {
        let (client, mut session) = client_with_storage_object().await;

        let result = client
            .delete_storage_objects(
                &mut session,
                &[
                    ApiDeleteStorageObjectId {
                        collection: "Cards".to_owned(),
                        key: "card1".to_owned(),
                        version: "".to_owned(),
                    },
                    ApiDeleteStorageObjectId {
                        collection: "Cards".to_owned(),
                        key: "card2".to_owned(),
                        version: "".to_owned(),
                    },
                ],
            )
            .await;

        println!("{:?}", result);
        assert_eq!(result.is_ok(), true);
    });
}

#[test]
fn test_list_storage_objects() {
    block_on(async {
        let (client, mut session) = client_with_storage_object().await;

        let result1 = client
            .list_storage_objects(&mut session, "Cards", Some(1), None)
            .await
            .unwrap();
        assert_eq!(result1.cursor.len() > 0, true);
        let result2 = client
            .list_storage_objects(&mut session, "Cards", None, Some(&result1.cursor))
            .await;

        println!("{:?}", result2);
        assert_eq!(result2.is_ok(), true);
        assert_eq!(result2.unwrap().cursor, "".to_owned());
    });
}

#[test]
fn test_list_users_storage_objects() {
    block_on(async {
        let (client, mut session) = client_with_storage_object().await;
        let user_id = client.get_account(&mut session).await.unwrap().user.id;

        let result1 = client
            .list_users_storage_objects(&mut session, "Cards", &user_id, Some(1), None)
            .await
            .unwrap();
        assert_eq!(result1.cursor.len() > 0, true);
        let result2 = client
            .list_users_storage_objects(
                &mut session,
                "Cards",
                &user_id,
                None,
                Some(&result1.cursor),
            )
            .await;

        println!("{:?}", result2);
        assert_eq!(result2.is_ok(), true);
        assert_eq!(result2.unwrap().cursor, "".to_owned());
    });
}
