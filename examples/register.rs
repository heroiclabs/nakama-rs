use nakama_rs::*;
use nakama_rs::api_client::ApiClient;

fn main() {
    let mut client = ApiClient::new("defaultkey", "127.0.0.1", config::DEFAULT_PORT, "http");
    // Note that the minimum password length is 8 characters!
    client.register("some@user.com", "password", "Username");

    loop {
        client.tick();
        if !client.in_progress() {
            break
        }
    }

    println!("Username: {:?}", client.username());
}
