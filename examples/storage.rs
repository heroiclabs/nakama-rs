use nakama_rs::*;
use nakama_rs::api_client::ApiClient;

fn main() {
    let mut client = ApiClient::new("defaultkey", "127.0.0.1", config::DEFAULT_PORT, "http");
    // Note that the minimum password length is 8 characters!
    client.register("some@user.com", "password", "Username");

    while client.in_progress() {
        client.tick()
    }

    println!("Fetching objects: ");
    client.list_storage_objects("some_collection");

    while client.in_progress() {
        client.tick()
    }

    println!("Received {:?} objects\n", client.get_num_storage_objects("some_collection"));

    println!("Fetching one object: ");
    client.fetch_storage_object("some_collection", "k");

    while client.in_progress() {
        client.tick()
    }

    println!("Object: {:?}\n", client.get_storage_object("some_collection", "k"));

    println!("Writing object:");
    client.write_storage_object("some_collection", "k", "{ \\\"Hello\\\": \\\"Sky!\\\" }");

    while client.in_progress() {
        client.tick()
    }

    println!("Object: {:?}\n", client.get_storage_object("some_collection", "k"));

    println!("Creating object (did not exist before):");
    client.create_storage_object("some_collection", "new", "{ \\\"New\\\": \\\"Value\\\" }");

    while client.in_progress() {
        client.tick()
    }

    println!("Object: {:?}\n", client.get_storage_object("some_collection", "new"));

    println!("Deleting newly created object:");
    client.delete_storage_object("some_collection", "new");

    while client.in_progress() {
        client.tick()
    }

    println!("Object not found anymore: {:?}", client.get_storage_object("some_collection", "new"));
}
