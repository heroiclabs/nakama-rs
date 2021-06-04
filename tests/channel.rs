use futures::executor::block_on;
use nakama_rs::socket::Socket;
use nakama_rs::test_helpers;
use std::thread::sleep;
use std::time::Duration;

#[test]
fn test_channel_room_creation() {
    let future = async {
        let (socket1, ..) =
            test_helpers::sockets_with_users("socketchannel1", "socketchannel2").await;
        let channel = socket1.join_chat("MyRoom", 1, false, false).await;
        assert_eq!(channel.unwrap().room_name, "MyRoom".to_owned())
    };

    block_on(future);
}

#[test]
fn test_channel_direct_message_creation() {
    let future = async {
        let (socket1, mut socket2, account1, account2) =
            test_helpers::sockets_with_users("socketchannel1", "socketchannel2").await;
        socket1
            .join_chat(&account2.user.id, 2, false, false)
            .await
            .expect("Failed to join chat");
        // The user will receive a notification that a user wants to chat and can then join.
        let _ = socket2.join_chat(&account1.user.id, 2, false, false).await;
        socket2.on_received_channel_presence(|presence| {
            println!("{:?}", presence);
        });
        // TODO: asyncify the callbacks for tests
        sleep(Duration::from_secs(1));
    };

    block_on(future);
}

// #[test]
// fn test_channel_group_chat_creation() {
//     todo!()
// }

#[test]
fn test_channel_leave() {
    block_on(async {
        let (socket1, ..) =
            test_helpers::sockets_with_users("socketchannel1", "socketchannel2").await;
        let channel = socket1.join_chat("MyRoom", 1, false, false).await.unwrap();
        socket1
            .leave_chat(&channel.id)
            .await
            .expect("Failed to leave chat");
    });
}

#[test]
fn test_channel_message() {
    block_on(async {
        let (socket1, ..) =
            test_helpers::sockets_with_users("socketchannel1", "socketchannel2").await;
        let channel = socket1.join_chat("MyRoom", 1, true, false).await.unwrap();
        let ack = socket1
            .write_chat_message(&channel.id, r#"{"text":"Hello, World!"}"#)
            .await
            .unwrap();

        let ack = socket1
            .update_chat_message(&channel.id, &ack.message_id, r#"{"text":"Bye, World!"}"#)
            .await
            .unwrap();

        let ack = socket1
            .remove_chat_message(&channel.id, &ack.message_id)
            .await
            .unwrap();

        println!("{:?}", ack);
    })
}
