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
