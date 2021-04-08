use nakama_rs::{
    api_client::ApiClient,
    config,
};

fn main() {
    let mut client = ApiClient::new("defaultkey", "127.0.0.1", config::DEFAULT_PORT, "http");

    client.register("email@provider.com", "password", "Leader");

    while client.in_progress() {
        client.tick()
    }

    client.write_leaderboard_record("wins", 1);

    while client.in_progress() {
        client.tick()
    }

    client.list_leaderboard_records("wins");

    while client.in_progress() {
        client.tick()
    }

    if let Some(leaderboard) = client.leaderboard_records("wins") {
        for record in &leaderboard.records {
            println!("{:?}", record);
        }
    }
}
