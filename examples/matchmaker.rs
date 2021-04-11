use nakama_rs::{api_client::ApiClient, config};
use nakama_rs::matchmaker::{Matchmaker, QueryItemBuilder};

fn tick_clients(clients: &mut Vec<ApiClient>) {
    while clients.iter().any(|client| client.in_progress()) {
       clients.iter_mut().for_each(|client| {
           client.tick();
       })
    }
}

fn wait_tokens(clients: &mut Vec<ApiClient>) {
    while clients.iter().any(|client| client.matchmaker_token.is_none()) {
        clients.iter_mut().for_each(|client| {
            client.tick();
        })
    }
}

fn match_rank_range() {
    let mut clients: Vec<ApiClient> = (1..5).map(|_| {
        ApiClient::new("defaultkey", "127.0.0.1", config::DEFAULT_PORT, "http")
    }).collect();

    clients.iter_mut().enumerate().for_each(|(i, client)| {
        client.register(&format!("email{}@provider.com", i), "password", &format!("Player{}", i));
    });

    tick_clients(&mut clients);

    println!("Client 1 with rank 5 searching for a match with rank between 3 and 7");
    let mut matchmaker = Matchmaker::new();
    matchmaker
        .add_numeric_property("rank", 5.0)
        .add_query_item(&QueryItemBuilder::new("rank").geq(3).required().build())
        .add_query_item(&QueryItemBuilder::new("rank").leq(7).required().build());

    println!("Client 2 with rank 4 searching for a match with rank between 2 and 6");
    let mut matchmaker2 = Matchmaker::new();
    matchmaker2
        .add_numeric_property("rank", 4.0)
        .add_query_item(&QueryItemBuilder::new("rank").geq(2).required().build())
        .add_query_item(&QueryItemBuilder::new("rank").leq(6).required().build());

    println!("Client 3 with rank 10 searching for a match with rank between 8 and 12");
    let mut matchmaker3 = Matchmaker::new();
    matchmaker3
        .add_numeric_property("rank", 10.0)
        .add_query_item(&QueryItemBuilder::new("rank").geq(8).required().build())
        .add_query_item(&QueryItemBuilder::new("rank").leq(12).required().build());

    println!("Client 4 with rank 9 searching for a match with rank between 7 and 11");
    let mut matchmaker4 = Matchmaker::new();
    matchmaker4
        .add_numeric_property("rank", 9.0)
        .add_query_item(&QueryItemBuilder::new("rank").geq(7).required().build())
        .add_query_item(&QueryItemBuilder::new("rank").leq(11).required().build());

    clients[0].socket_add_matchmaker(&matchmaker);
    clients[1].socket_add_matchmaker(&matchmaker2);
    clients[2].socket_add_matchmaker(&matchmaker3);
    clients[3].socket_add_matchmaker(&matchmaker4);

    wait_tokens(&mut clients) ;

    assert_eq!(clients[0].matchmaker_token, clients[1].matchmaker_token);
    println!("Successfully ranked the clients with rank 4 and 5");
    assert_eq!(clients[2].matchmaker_token, clients[3].matchmaker_token);
    println!("Successfully ranked the clients with rank 9 and 10");
}

fn match_region() {
    let mut client = ApiClient::new("defaultkey", "127.0.0.1", config::DEFAULT_PORT, "http");
    let mut client2 = ApiClient::new("defaultkey", "127.0.0.1", config::DEFAULT_PORT, "http");

    client.register("email1@provider.com", "password", "Player1");
    client2.register("email2@provider.com", "password", "Player2");

    while client.in_progress() {
        client.tick()
    }

    while client2.in_progress() {
        client2.tick()
    }

    println!("Both clients Searching for a match in Europe");
    let mut matchmaker = Matchmaker::new();
    matchmaker
        .add_string_property("region", "Europe")
        .add_query_item(&QueryItemBuilder::new("region").term("Europe").required().build());

    client.socket_add_matchmaker(&matchmaker);
    client2.socket_add_matchmaker(&matchmaker);

    while client.matchmaker_token.is_none() || client2.matchmaker_token.is_none() {
        client.tick();
        client2.tick();
    }

    assert_eq!(client.matchmaker_token, client2.matchmaker_token);
    println!("Both clients found a match in Europe");
}

fn main() {
    match_rank_range();
    match_region();
}
