use futures::executor::block_on;
use nakama_rs::client::Client;
use nakama_rs::default_client::DefaultClient;
use std::collections::HashMap;

fn main() {
    block_on(async {
        let client = DefaultClient::new_with_adapter();

        let result = client
            .authenticate_device("too_short", None, true, HashMap::new())
            .await;
        println!("{:?}", result);

        let result = client
            .authenticate_device("long_enough_device_id", None, true, HashMap::new())
            .await;

        println!("{:?}", result);
    })
}
