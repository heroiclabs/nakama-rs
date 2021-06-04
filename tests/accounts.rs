use futures::executor::block_on;
use nakama_rs::client::Client;
use nakama_rs::default_client::DefaultClient;
use std::collections::HashMap;

#[test]
fn test_get_account() {
    let client = DefaultClient::new_with_adapter();

    let result = block_on(async {
        let mut session = client
            .authenticate_device("somedeviceid", Some("TestUser"), true, HashMap::new())
            .await?;

        client.get_account(&mut session).await
    });

    println!("Result: {:?}", result);
    let account = result.unwrap();
    assert_eq!(account.devices[0].id, "somedeviceid",);
    assert_eq!(account.user.username, "TestUser");
}

#[test]
fn test_update_account() {
    let client = DefaultClient::new_with_adapter();

    let result = block_on(async {
        let mut session = client
            .authenticate_device("somedeviceid", Some("TestUser"), true, HashMap::new())
            .await?;

        client
            .update_account(
                &mut session,
                "TestUser",
                Some("DisplayName"),
                Some("url://avatar"),
                Some("de"),
                Some("Austria"),
                Some("Europe/Vienna"),
            )
            .await?;

        client.get_account(&mut session).await
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
