// Copyright 2021 The Nakama Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::socket_adapter::SocketAdapter;
use log::{debug, error, trace};
use qws;
use qws::{CloseCode, Handshake};
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, SendError, Sender};

enum Message {
    StringMessage(String),
    Connected,
    Error(qws::Error),
}

pub struct WebSocketAdapter {
    on_connected: Option<Box<dyn Fn() + Send + 'static>>,
    on_closed: Option<Box<dyn Fn() + Send + 'static>>,
    on_received: Option<Box<dyn Fn(Result<String, WebSocketAdapterError>) + Send + 'static>>,

    rx_message: Option<Receiver<Message>>,
    tx_message: Option<qws::Sender>,
}

// Client on the websocket thread
struct WebSocketClient {
    tx: Sender<Message>,
}

impl WebSocketClient {
    fn send(&self, message: Message) -> Result<(), SendError<Message>> {
        self.tx.send(message)
    }
}

impl qws::Handler for WebSocketClient {
    fn on_shutdown(&mut self) {
        trace!("WebSocketClient::on_shutdown called");
    }

    fn on_open(&mut self, shake: Handshake) -> qws::Result<()> {
        if let Some(addr) = shake.remote_addr()? {
            let result = self.send(Message::Connected);
            match result {
                Ok(_) => {
                    debug!("Connection with {} now open", addr);
                }
                Err(err) => {
                    error!("Failed to send {}", err);
                }
            }
        }
        Ok(())
    }

    fn on_message(&mut self, msg: qws::Message) -> qws::Result<()> {
        match msg {
            qws::Message::Text(data) => {
                let result = self.send(Message::StringMessage(data));
                if let Err(err) = result {
                    error!("Handler::on_message: {}", err);
                }
            }
            qws::Message::Binary(_) => {
                trace!("Handler::on_message: Received binary data");
            }
        }
        Ok(())
    }

    fn on_close(&mut self, code: CloseCode, reason: &str) {
        debug!("Connection closing due to ({:?}) {}", code, reason);
    }

    // Copied from trait for now
    fn on_error(&mut self, err: qws::Error) {
        // Ignore connection reset errors by default, but allow library clients to see them by
        // overriding this method if they want
        if let qws::ErrorKind::Io(ref err) = err.kind {
            if let Some(104) = err.raw_os_error() {
                return;
            }
        }

        let result = self.send(Message::Error(err));
        if let Err(err) = result {
            error!("on_error: SendError: {}", err);
        }
    }
}

impl WebSocketAdapter {
    pub fn new() -> WebSocketAdapter {
        WebSocketAdapter {
            on_connected: None,
            on_closed: None,
            on_received: None,

            rx_message: None,
            tx_message: None,
        }
    }
}

#[derive(Debug)]
pub enum WebSocketAdapterError {
    IOError,
    WebSocketError(qws::Error),
}

impl From<qws::Error> for WebSocketAdapterError {
    fn from(err: qws::Error) -> Self {
        WebSocketAdapterError::WebSocketError(err)
    }
}

impl Display for WebSocketAdapterError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

impl Error for WebSocketAdapterError {}

impl SocketAdapter for WebSocketAdapter {
    type Error = WebSocketAdapterError;

    fn on_connected<T>(&mut self, callback: T)
    where
        T: Fn() + Send + 'static,
    {
        self.on_connected = Some(Box::new(callback));
    }

    fn on_closed<T>(&mut self, callback: T)
    where
        T: Fn() + Send + 'static,
    {
        self.on_closed = Some(Box::new(callback))
    }

    fn on_received<T>(&mut self, callback: T)
    where
        T: Fn(Result<String, WebSocketAdapterError>) + Send + 'static,
    {
        self.on_received = Some(Box::new(callback));
    }

    fn is_connected(&self) -> bool {
        todo!()
    }

    fn is_connecting(&self) -> bool {
        todo!();
    }

    fn close(&mut self) {
        todo!()
    }

    fn connect(&mut self, addr: &str, _timeout: i32) {
        let (tx, rx) = mpsc::channel();
        let (tx_init, rx_init) = mpsc::channel();

        let addr = addr.to_owned();

        std::thread::spawn({
            move || {
                qws::connect(addr, |out| {
                    let response = tx_init.send(out);
                    if let Err(err) = response {
                        error!("connect (Thread): Error sending data {}", err);
                    }
                    return WebSocketClient { tx: tx.clone() };
                })
            }
        });

        self.tx_message = rx_init.recv().ok();

        self.rx_message = Some(rx);
    }

    fn send(&self, data: &str, _reliable: bool) -> Result<(), Self::Error> {
        if let Some(ref sender) = self.tx_message {
            println!("Sending {:?}", data);
            return sender
                .send(qws::Message::Text(data.to_owned()))
                .map_err(|err| err.into());
        }

        Ok(())
    }

    fn tick(&self) {
        if let Some(ref rx) = self.rx_message {
            while let Ok(data) = rx.try_recv() {
                match data {
                    Message::StringMessage(msg) => {
                        if let Some(ref cb) = self.on_received {
                            cb(Ok(msg));
                        }
                    }
                    Message::Connected => {
                        if let Some(ref cb) = self.on_connected {
                            cb();
                        }
                    }
                    Message::Error(err) => {
                        if let Some(ref cb) = self.on_received {
                            cb(Err(err.into()));
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::thread::sleep;
    use std::time::Duration;

    #[test]
    fn test() {
        let mut socket_adapter = WebSocketAdapter::new();
        socket_adapter.connect("ws://echo.websocket.org", 0);
        socket_adapter.on_received(move |data| println!("{:?}", data));
        sleep(Duration::from_secs(1));

        println!("Sending!");
        socket_adapter.send("Hello", false).unwrap();
        sleep(Duration::from_secs(1));
        println!("Tick!");
        socket_adapter.tick();
        sleep(Duration::from_secs(1));
        println!("Tick!");
        socket_adapter.tick();
    }
}
