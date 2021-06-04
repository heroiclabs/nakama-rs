use futures::executor::block_on;
use nakama_rs::client::Client;
use nakama_rs::default_client::DefaultClient;
use std::collections::HashMap;

#[test]
fn test_authenticate_device_id_too_short() {
    let client = DefaultClient::new_with_adapter();

    let result = block_on(async {
        client
            .authenticate_device("too_short", None, true, HashMap::new())
            .await
    });

    println!("Result: {:?}", result);
    assert_eq!(result.is_err(), true)
}

#[test]
fn test_authenticate_device_id() {
    let client = DefaultClient::new_with_adapter();

    let result = block_on(async {
        client
            .authenticate_device("long_enough_device_id", None, true, HashMap::new())
            .await
    });

    println!("Result: {:?}", result);
    assert_eq!(result.is_ok(), true)
}

#[test]
fn test_authenticating_with_unknown_credentials() {
    let client = DefaultClient::new_with_adapter();
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
    let client = DefaultClient::new_with_adapter();
    let result = block_on(async {
        let mut session = client
            .authenticate_device("usersdeviceid", None, true, HashMap::new())
            .await?;

        client
            .link_email(&mut session, "test@user.com", "userspassword")
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
    let client = DefaultClient::new_with_adapter();
    let result = block_on(async {
        let mut session = client
            .authenticate_device("usersdeviceid", None, true, HashMap::new())
            .await?;

        client
            .link_email(&mut session, "test@user.com", "userspassword")
            .await?;
        client
            .unlink_email(&mut session, "test@user.com", "userspassword")
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
