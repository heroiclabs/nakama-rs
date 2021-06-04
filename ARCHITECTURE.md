# Architecture

The Rust Nakama Client consists of high level traits with implementations for various frameworks and protocols.

Most traits are defined using `async_trait` of the `async_trait` crate, because currently using `async` in traits is not supported.
This means every function call has an overhead of one heap allocation.

### Client
The `Client` trait declares async functions to call the Nakama server endpoints. It has an
associated `Error` type because the trait cannot know what errors the implementation can generate.

### ClientAdapter
The `ClientAdapter` trait declares a single function `send` as an abstract interface to
send data to the Nakama server. For now, there is only a single implementation `RestHttpAdapter` that
uses `REST` to communicate with the Nakama server.

In the future, a gRPC adapter can be added.

### DefaultClient
The `DefaultClient` is an implementation of `Client`. It has a type parameter specifying the 
underlying `ClientAdapter` to use. The `DefaultClient` is stateless and can be sent and accessed between threads. This
also means that futures awaiting on its async functions can be sent between threads. 

### Socket
The `Socket` trait declares async functions to communicate with the realtime multiplayer engine.
It also declares functions to specify callbacks for received messages that have no corresponding request.

### SocketAdapter
The `SocketAdapter` trait declares low-level functions to communicate with the realtime multiplayer engine.
Handling messages is done using callbacks. In order to execute the callbacks, the `tick` function needs to be called.

### WebSocketAdapter
`WebSocketAdapter` is an implementation of `SocketAdapter` using the `qws` library.

### WebSocket
`WebSocket` is an implementation of `Socket`. It has a type parameter specifying the underlying `SocketAdapter` implementation to use.
`WebSocket` can be sent and accessed between threads.

Because some messages are handled using callbacks, the `tick` function needs to be called on
a thread. The callbacks will be invoked on the calling thread. The callback functions can be registered on
any thread, but the callback needs to be able to be sent between threads.

## WASM Support
For WASM, the following properties need to be considered:
- WASM is single-threaded
- WASM should generate a small library

The library does not use any async runtime. By replacing the `SocketAdapter` and `ClientAdapter` with implementation
for WASM, it should be possible to target WASM.

An async runtime targeting WASM would poll futures every frame instead of using a thread pool.
