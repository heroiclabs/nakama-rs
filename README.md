<a href="https://crates.io/crates/nakama-rs">
    <img src="https://img.shields.io/crates/v/nakama-rs.svg" alt="Nakama-rs" />
</a>

Simple Bindings to the Nakama library!

Read the [documentation](https://docs.rs/nakama-rs).

# Why would you use this library?

* Easy access to Nakama's API.
* Usable with the async/future library of your choice.
* Minimal dependencies.
* Safe: no `unsafe` calls!

# Usage
Add the following to you Cargo.toml file:
```
nakama-rs = "*"
```

Use it like so:
```rust
use nakama_rs::*;
fn main() {
    let mut client = ApiClient::new("defaultkey", "127.0.0.1", 7350, "http");
    client.authenticate("email@email.com", "password");
    client.tick();
}
```

For more examples, see the documentation and examples. To run the examples, you need a local Nakama instance running.
The easiest way is to run `docker-compose up` in the `examples/` folder.
For information on how to set up docker for usage with Nakama, see [Docker quickstart](https://heroiclabs.com/docs/install-docker-quickstart/).

