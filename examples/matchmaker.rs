use nakama_rs::{api_client::ApiClient, config};
use nakama_rs::matchmaker::Matchmaker;

fn main() {
    let mut client = ApiClient::new("defaultkey", "127.0.0.1", config::DEFAULT_PORT, "http");
    let mut client2 = ApiClient::new("defaultkey", "127.0.0.1", config::DEFAULT_PORT, "http");

    client.register("email@provider.com", "password", "PlayerA");
    client2.register("email2@provider.com", "password", "PlayerB");

    while client.in_progress() {
        client.tick()
    }

    while client2.in_progress() {
        client2.tick()
    }

    let mut matchmaker = Matchmaker::new(2, 2);
    matchmaker
        .add_string_property("region", "Europe")
        .add_query_item("region").term("Europe").required().build();

    client.socket_add_matchmaker(&matchmaker);
    client2.socket_add_matchmaker(&matchmaker);

    while client.matchmaker_token.is_none() || client2.matchmaker_token.is_none() {
        client.tick();
        client2.tick();
    }

    println!("{:?} {:?}", client.matchmaker_token, client2.matchmaker_token);
    assert_eq!(client.matchmaker_token, client2.matchmaker_token);
}