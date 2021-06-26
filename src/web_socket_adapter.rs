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
use url;
use qws;
use qws::{CloseCode, Handshake};
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, SendError, Sender};
use std::thread::spawn;
use std::ops::Add;
use chrono::{FixedOffset, Duration, DateTime, Utc};
use rand::Rng;
use std::cell::RefCell;

enum Message {
    StringMessage(String),
    Connected,
    Closed,
    Error(qws::Error),
    Reconnect(DateTime<Utc>)
}

pub struct WebSocketAdapter {
    on_connected: Option<Box<dyn Fn() + Send + 'static>>,
    on_closed: Option<Box<dyn Fn() + Send + 'static>>,
    on_received: Option<Box<dyn Fn(Result<String, WebSocketAdapterError>) + Send + 'static>>,

    rx_message: Option<Receiver<Message>>,
    tx_message: Option<qws::Sender>,

    addr: String,
    reconnect_on: RefCell<Option<DateTime<Utc>>>,
}

// Client on the websocket thread
struct WebSocketClient {
    auto_reconnect: bool,
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
        println!("Opening");
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
        if self.auto_reconnect && code == CloseCode::Error {
            let rand_secs = rand::thread_rng().gen_range(5..6);
            self.tx.send(Message::Reconnect(chrono::Utc::now() + Duration::seconds(rand_secs)))
                .expect("Failed to send");
        }

        debug!("Connection closing due to ({:?}) {}", code, reason);
        let result = self.send(Message::Closed);
        if let Err(err) = result {
            error!("Failed to send {}", err);
        }
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

            addr: "".to_owned(),
            reconnect_on: RefCell::new(None),
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
        self.tx_message.as_ref().unwrap().close(CloseCode::Normal)
            .expect("Failed to close socket");
    }

    fn connect(&mut self, addr: &str, _timeout: i32) {
        let (tx, rx) = mpsc::channel();
        let (tx_init, rx_init) = mpsc::channel();

        let addr = addr.to_owned();
        self.addr = addr.clone();

        std::thread::spawn({
            move || {
                qws::connect(addr.clone(), |out| {
                    let response = tx_init.send(out.clone());
                    if let Err(err) = response {
                        error!("connect (Thread): Error sending data {}", err);
                    }
                    return WebSocketClient { tx: tx.clone(), auto_reconnect: true };
                }).expect("Failed to connect")
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
        let reconnect_on = self.reconnect_on.borrow_mut().take();
        if let Some(reconnect_on) = reconnect_on {
            if Utc::now().ge(&reconnect_on) {
                let mut addr = url::Url::parse(&self.addr).expect("Failed to parse url");
                addr.set_port(addr.port().map(|port| port + 1));
                self.tx_message.as_ref().unwrap().connect(addr).expect("Failed to re-connect");
            }
            *self.reconnect_on.borrow_mut() = Some(reconnect_on);
        }

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
                    Message::Closed => {
                        if let Some(ref cb) = self.on_closed {
                            cb();
                        }
                    },
                    Message::Reconnect(reconnect_on) => {
                        *self.reconnect_on.borrow_mut() = Some(reconnect_on);
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
    use oneshot::channel;

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

    #[test]
    fn test_reconnect() {
        let mut socket_adapter = WebSocketAdapter::new();

        spawn(|| {
            let server = qws::listen("127.0.0.1:3000", |out| {
                  move |msg| {
                      out.shutdown();
                      println!("Closing!");
                      Ok(())
                  }
            }).expect("Failed to listen");

            println!("Closed!");
            sleep(Duration::from_secs(5));

            let server = qws::listen("127.0.0.1:3001", |out| {
                move |msg| {
                    out.close(CloseCode::Error);
                    out.shutdown()
                }
            }).expect("Failed to listen");
        });

        let (tx_connected, rx_connected) = mpsc::channel();
        let (tx_received, rx_received) = mpsc::channel();

        socket_adapter.on_connected(move || {
           tx_connected.send(()).expect("Failed to send");
        });
        socket_adapter.on_received(move |data| {
            // println!("{:?}", data);
            tx_received.send(data);
        });
        socket_adapter.connect("ws://127.0.0.1:3000", -1);

        loop {
            sleep(Duration::from_millis(16));
            socket_adapter.tick();
            if let Ok(()) = rx_connected.try_recv() {
                break;
            }
        }

        socket_adapter.send("Hello World!", false);

        // sleep(Duration::from_secs(10));

        loop {
            sleep(Duration::from_millis(16));
            socket_adapter.tick();
            if let Ok(()) = rx_connected.try_recv() {
                break;
            }
        }

        println!("Finished")
    }
}
