use nakama_rs::{DefaultClient, Client};
use nakama_rs::mock_adapter::MockAdapter;
use nakama_rs::retry::{RetryConfiguration, Delay};
use rand::rngs::StdRng;
use std::pin::Pin;
use std::future::Future;
use futures::executor::block_on;
use std::collections::HashMap;

pub struct MockDelay {

}

impl Delay for MockDelay {
    fn delay(ms: u64) -> Pin<Box<dyn Future<Output=()> + Send>> {
        Box::pin(async move {
            println!("Delaying for {} ms", ms);
            // do nothing
        })
    }
}

#[test]
fn test_retry() {
    let adapter = MockAdapter {};
    let retry_configuration: RetryConfiguration<StdRng, MockDelay> = RetryConfiguration::new();
    let client = DefaultClient::new_with_configuration(adapter, "defaultkey", "", retry_configuration);


    let result = block_on(async {
        client
            .authenticate_device("somedeviceid", Some("TestUser"), true, HashMap::new())
            .await
    });

    println!("Result: {:?}", result);
}