use futures::executor::block_on;
use log::LevelFilter;
use nakama_rs::socket::Socket;
use nakama_rs::test_helpers;
use std::sync::mpsc;

#[test]
fn create_and_close_party() {
    block_on(async {
        let (socket1, _, _, _) =
            test_helpers::sockets_with_users("partyuserone", "partyusertwo").await;

        let party = socket1.create_party(true, 2).await.unwrap();
        socket1
            .close_party(&party.party_id)
            .await
            .expect("Failed to close party");
    })
}

#[test]
fn join_and_leave_party() {
    block_on(async {
        let (socket1, socket2, _, _) =
            test_helpers::sockets_with_users("partyuserone", "partyusertwo").await;

        let party = socket1.create_party(true, 2).await.unwrap();
        socket2
            .join_party(&party.party_id)
            .await
            .expect("Failed to join party");
        socket2
            .leave_party(&party.party_id)
            .await
            .expect("Failed to leave party");
    })
}

#[test]
fn promote_and_remove_party_member() {
    block_on(async {
        let (tx, rx) = mpsc::channel();
        let (mut socket1, socket2, ..) =
            test_helpers::sockets_with_users("partyuserone", "partyusertwo").await;

        socket1.on_received_party_presence(move |presence| {
            tx.send(presence).expect("Failed to send party presence");
        });

        let party = socket1.create_party(true, 2).await.unwrap();
        // Wait for first party presence event
        rx.recv().expect("Failed to recv party presence");

        socket2.join_party(&party.party_id).await.unwrap();
        // Wait for joined user
        let mut joined_presence = rx.recv().unwrap();
        let user_presence = joined_presence.joins.remove(0);

        socket1
            .promote_party_member(&party.party_id, user_presence.clone())
            .await
            .unwrap();
        socket2
            .remove_party_member(&party.party_id, party.leader)
            .await
            .unwrap();
    })
}

#[test]
fn test_private_group() {
    block_on(async {
        let (socket1, socket2, ..) =
            test_helpers::sockets_with_users("partyuserone", "partyusertwo").await;

        let party = socket1.create_party(false, 2).await.unwrap();
        socket2.join_party(&party.party_id).await.unwrap();
        let mut join_requests = socket1
            .list_party_join_requests(&party.party_id)
            .await
            .unwrap();
        socket1
            .accept_party_member(&party.party_id, &join_requests.presences[0])
            .await
            .unwrap();
        socket1
            .promote_party_member(&party.party_id, join_requests.presences.remove(0))
            .await
            .unwrap();
        socket2
            .remove_party_member(&party.party_id, party.leader)
            .await
            .unwrap();
    });
}

#[test]
fn test_send_party_data() {
    block_on(async {
        let (tx, rx) = mpsc::channel();
        let (socket1, mut socket2, _, _) =
            test_helpers::sockets_with_users("partyuserone", "partyusertwo").await;

        let party = socket1.create_party(true, 2).await.unwrap();
        socket2.join_party(&party.party_id).await.unwrap();

        socket2.on_received_party_data(move |data| {
            tx.send(data).expect("Failed to send data");
        });
        socket1
            .send_party_data(&party.party_id, 1, &[1, 2, 3, 4])
            .await
            .expect("Failed to send party data");

        println!("{:?}", rx.recv());
    })
}
