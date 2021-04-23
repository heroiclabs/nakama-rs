use nakama_rs::*;
use nakama_rs::api_client::ApiClient;

fn main() {
    let mut client = ApiClient::new("defaultkey", "127.0.0.1", config::DEFAULT_PORT, "http");
    // Note that the minimum password length is 8 characters!
    client.register("some@user.com", "password", "Username");

    while client.in_progress() {
        client.tick()
    }

    client.create_storage_object("some_collection", "k", "{ \\\"Hello\\\": \\\"World!\\\" }");

    while client.in_progress() {
        client.tick()
    }

    println!("Object: {:?}", client.get_storage_object("some_collection", "k"));
}
