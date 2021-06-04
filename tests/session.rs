use futures::executor::block_on;
use nakama_rs::client::Client;
use nakama_rs::default_client::DefaultClient;
use std::collections::HashMap;

#[test]
fn test_session_variables() {
    let client = DefaultClient::new_with_adapter();

    let result = block_on(async {
        let mut vars = HashMap::new();
        vars.insert("ident".to_owned(), "hidden".to_owned());
        let mut session = client
            .authenticate_device("somenewdeviceid", None, true, vars)
            .await?;

        client.get_account(&mut session).await
    });

    println!("Result: {:?}", result);
    // TODO: parse "vrs" from the token payload
    // let account = result.unwrap();
}
