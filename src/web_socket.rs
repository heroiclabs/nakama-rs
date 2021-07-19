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

use crate::api::{ApiChannelMessage, ApiNotification, ApiRpc};
use crate::session::Session;
use crate::socket::{
    Channel, ChannelJoin, ChannelLeave, ChannelMesageRemove, ChannelMesageUpdate,
    ChannelMessageAck, ChannelMessageSend, ChannelPresenceEvent, Error, Match, MatchCreate,
    MatchData, MatchDataSend, MatchJoin, MatchLeave, MatchPresenceEvent, MatchmakerAdd,
    MatchmakerMatched, MatchmakerRemove, MatchmakerTicket, Party, PartyAccept, PartyClose,
    PartyCreate, PartyData, PartyDataSend, PartyJoin, PartyJoinRequest, PartyJoinRequestList,
    PartyLeader, PartyLeave, PartyMatchmakerAdd, PartyMatchmakerRemove, PartyMatchmakerTicket,
    PartyPresenceEvent, PartyPromote, PartyRemove, Socket, Status, StatusFollow,
    StatusPresenceEvent, StatusUnfollow, StatusUpdate, StreamData, StreamPresenceEvent,
    UserPresence, WebSocketMessageEnvelope, WebSocketMessageEnvelopeHeader,
};
use crate::socket_adapter::SocketAdapter;
use async_trait::async_trait;
use log::{error, trace};
use nanoserde::{DeJson, DeJsonErr, SerJson};
use std::collections::HashMap;
use std::error;
use std::sync::{Arc, Mutex};

use crate::default_client::str_slice_to_owned;
use crate::matchmaker::Matchmaker;
use crate::web_socket_adapter::WebSocketAdapter;
use oneshot;
use oneshot::RecvError;
use std::fmt::{Debug, Display, Formatter};
use rand::rngs::StdRng;

pub enum WebSocketError<A: SocketAdapter> {
    AdapterError(A::Error),
    TimeoutError,
    RecvError(RecvError),
    ApiError(Error),
    DeJsonError(DeJsonErr),
}

impl<A: SocketAdapter> Debug for WebSocketError<A> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            WebSocketError::AdapterError(err) => std::fmt::Debug::fmt(err, f),
            WebSocketError::TimeoutError => std::fmt::Debug::fmt("Timeout", f),
            WebSocketError::RecvError(err) => std::fmt::Debug::fmt(err, f),
            WebSocketError::ApiError(err) => std::fmt::Debug::fmt(err, f),
            WebSocketError::DeJsonError(err) => std::fmt::Debug::fmt(err, f),
        }
    }
}

impl<A: SocketAdapter> Display for WebSocketError<A> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

impl<A: SocketAdapter> error::Error for WebSocketError<A> {}

#[derive(Default)]
struct SharedState {
    cid: i64,
    connected: Vec<oneshot::Sender<()>>,
    responses: HashMap<i64, oneshot::Sender<Result<WebSocketMessageEnvelope, DeJsonErr>>>,
    timeouts: HashMap<i64, i64>,
    on_closed: Option<Box<dyn Fn() + Send + 'static>>,
    on_connected: Option<Box<dyn Fn() + Send + 'static>>,
    on_received_channel_message: Option<Box<dyn Fn(ApiChannelMessage) + Send + 'static>>,
    on_received_channel_presence: Option<Box<dyn Fn(ChannelPresenceEvent) + Send + 'static>>,
    on_received_error: Option<Box<dyn Fn(Error) + Send + 'static>>,
    on_received_matchmaker_matched: Option<Box<dyn Fn(MatchmakerMatched) + Send + 'static>>,
    on_received_match_state: Option<Box<dyn Fn(MatchData) + Send + 'static>>,
    on_received_match_presence: Option<Box<dyn Fn(MatchPresenceEvent) + Send + 'static>>,
    on_received_notification: Option<Box<dyn Fn(ApiNotification) + Send + 'static>>,
    on_received_party_close: Option<Box<dyn Fn(PartyClose) + Send + 'static>>,
    on_received_party_data: Option<Box<dyn Fn(PartyData) + Send + 'static>>,
    on_received_party_join_request: Option<Box<dyn Fn(PartyJoinRequest) + Send + 'static>>,
    on_received_party_leader: Option<Box<dyn Fn(PartyLeader) + Send + 'static>>,
    on_received_party_presence: Option<Box<dyn Fn(PartyPresenceEvent) + Send + 'static>>,
    on_received_status_presence: Option<Box<dyn Fn(StatusPresenceEvent) + Send + 'static>>,
    on_received_stream_presence: Option<Box<dyn Fn(StreamPresenceEvent) + Send + 'static>>,
    on_received_stream_state: Option<Box<dyn Fn(StreamData) + Send + 'static>>,
}

/// A socket to interact with Nakama realtime engine.
pub struct WebSocket<A: SocketAdapter> {
    adapter: Arc<Mutex<A>>,
    shared_state: Arc<Mutex<SharedState>>,
}

impl<A: SocketAdapter> Clone for WebSocket<A> {
    fn clone(&self) -> Self {
        WebSocket {
            adapter: self.adapter.clone(),
            shared_state: self.shared_state.clone(),
        }
    }
}

fn handle_message(shared_state: &Arc<Mutex<SharedState>>, msg: &String) {
    trace!("handle_message: Received message: {:?}", msg);
    let result: Result<WebSocketMessageEnvelope, DeJsonErr> = DeJson::deserialize_json(&msg);
    let mut shared_state = shared_state.lock().unwrap();
    match result {
        Ok(event) => {
            if let Some(ref cid) = event.cid {
                trace!("handle_message: Received message with cid");
                let cid = cid.parse::<i64>().unwrap();
                if let Some(response_event) = shared_state.responses.remove(&cid) {
                    let result = response_event.send(Ok(event));
                    if let Err(err) = result {
                        error!("handle_message: send error: {}", err);
                    }
                }
                return;
            }
            if let Some(message) = event.channel_message {
                if let Some(ref cb) = shared_state.on_received_channel_message {
                    cb(message)
                }
                return;
            }
            if let Some(message) = event.channel_presence_event {
                if let Some(ref cb) = shared_state.on_received_channel_presence {
                    cb(message)
                }
                return;
            }
            if let Some(message) = event.error {
                if let Some(ref cb) = shared_state.on_received_error {
                    cb(message)
                }
                return;
            }
            if let Some(message) = event.matchmaker_matched {
                if let Some(ref cb) = shared_state.on_received_matchmaker_matched {
                    cb(message)
                }
                return;
            }
            if let Some(message) = event.match_data {
                if let Some(ref cb) = shared_state.on_received_match_state {
                    cb(message)
                }
                return;
            }
            if let Some(message) = event.match_presence_event {
                if let Some(ref cb) = shared_state.on_received_match_presence {
                    cb(message)
                }
                return;
            }
            if let Some(mut message) = event.notifications {
                if let Some(ref cb) = shared_state.on_received_notification {
                    for message in message.notifications.drain(..) {
                        cb(message)
                    }
                }
                return;
            }
            if let Some(message) = event.party_close {
                if let Some(ref cb) = shared_state.on_received_party_close {
                    cb(message)
                }
                return;
            }
            if let Some(message) = event.party_data {
                if let Some(ref cb) = shared_state.on_received_party_data {
                    cb(message)
                }
                return;
            }
            if let Some(message) = event.party_join_request {
                if let Some(ref cb) = shared_state.on_received_party_join_request {
                    cb(message)
                }
                return;
            }
            if let Some(message) = event.party_leader {
                if let Some(ref cb) = shared_state.on_received_party_leader {
                    cb(message)
                }
                return;
            }
            if let Some(message) = event.party_presence_event {
                if let Some(ref cb) = shared_state.on_received_party_presence {
                    cb(message)
                }
                return;
            }
            if let Some(message) = event.status_presence_event {
                if let Some(ref cb) = shared_state.on_received_status_presence {
                    cb(message)
                }
                return;
            }
            if let Some(message) = event.stream_presence_event {
                if let Some(ref cb) = shared_state.on_received_stream_presence {
                    cb(message)
                }
                return;
            }
            if let Some(message) = event.stream_data {
                if let Some(ref cb) = shared_state.on_received_stream_state {
                    cb(message)
                }
                return;
            }
        }
        Err(err) => {
            error!("handle_message: Failed to parse json: {}", err);
            let result: Result<WebSocketMessageEnvelopeHeader, DeJsonErr> =
                DeJson::deserialize_json(&msg);
            match result {
                Ok(event) => {
                    // Inform the future about the API error
                    if let Some(ref cid) = event.cid {
                        trace!("handle_message: Received error message with cid");
                        let cid = cid.parse::<i64>().unwrap();
                        if let Some(response_event) = shared_state.responses.remove(&cid) {
                            // Send DeJsonErr
                            let result = response_event.send(Err(err));
                            if let Err(err) = result {
                                error!("handle_message: Received send error: {}", err)
                            }
                        }
                        return;
                    }
                }
                Err(_) => {
                    // We can't parse more information. Forward the json parse error
                    error!("{:?}", err)
                }
            }
        }
    }
}

impl WebSocket<WebSocketAdapter> {
    pub fn new_with_adapter(rng: StdRng) -> Self {
        let adapter = WebSocketAdapter::new(rng);
        WebSocket::new(adapter)
    }
}

impl<A: SocketAdapter + Send> WebSocket<A> {
    pub fn new(adapter: A) -> Self {
        let web_socket = WebSocket {
            adapter: Arc::new(Mutex::new(adapter)),
            shared_state: Arc::new(Mutex::new(SharedState {
                ..Default::default()
            })),
        };

        web_socket
            .adapter
            .lock()
            .expect("panic inside other mutex!")
            .on_received({
                let shared_state = web_socket.shared_state.clone();
                move |msg| match msg {
                    Err(error) => {
                        error!("on_received: {}", error);
                        return;
                    }
                    Ok(msg) => {
                        trace!("on_received: {}", msg);
                        handle_message(&shared_state, &msg);
                    }
                }
            });

        {
            let mut adapter = web_socket.adapter.lock().unwrap();
            adapter.on_closed({
                let shared_state = web_socket.shared_state.clone();
                move || {
                    if let Some(ref cb) = shared_state.lock().unwrap().on_closed {
                        cb()
                    }
                }
            });

            adapter.on_connected({
                let shared_state = web_socket.shared_state.clone();
                move || {
                    if let Some(ref cb) = shared_state.lock().unwrap().on_connected {
                        cb()
                    }

                    shared_state
                        .lock()
                        .unwrap()
                        .connected
                        .drain(..)
                        .for_each(|sender| {
                            let result = sender.send(());
                            if let Err(err) = result {
                                error!("on_connected: Received send error: {}", err)
                            }
                        });
                }
            });
        }

        web_socket
    }

    pub fn tick(&self) {
        self.adapter
            .lock()
            .expect("panic inside other mutex!")
            .tick();

        let mut shared_state = self.shared_state.lock().unwrap();

        // TODO: Use a clock!
        let (timeout_finished, timeouts) = shared_state
            .timeouts
            .iter()
            .map(|(k, v)| (*k, *v - 16))
            .partition(|&(_, timeout)| {
                return timeout <= 0;
            });
        shared_state.timeouts = timeouts;
        timeout_finished.iter().for_each(|(k, _)| {
            shared_state.responses.remove(k);
        })
    }

    fn make_envelope_with_cid(&self) -> (WebSocketMessageEnvelope, i64) {
        let cid = {
            let mut state = self.shared_state.lock().expect("Panic inside other mutex!");
            state.cid += 1;
            state.cid
        };

        (
            WebSocketMessageEnvelope {
                cid: Some(cid.to_string()),
                ..Default::default()
            },
            cid,
        )
    }

    fn make_envelope(&self) -> WebSocketMessageEnvelope {
        WebSocketMessageEnvelope {
            ..Default::default()
        }
    }

    #[inline]
    fn send(&self, data: &str, reliable: bool) -> Result<(), WebSocketError<A>> {
        trace!("send: Sending message: {:?}", data);
        self.adapter
            .lock()
            .expect("panic inside other mutex!")
            .send(data, reliable)
            .map_err(|err| WebSocketError::AdapterError(err))
    }

    async fn wait_response(
        &self,
        cid: i64,
    ) -> Result<WebSocketMessageEnvelope, <Self as Socket>::Error> {
        let (tx, rx) = oneshot::channel::<Result<WebSocketMessageEnvelope, DeJsonErr>>();

        {
            let mut shared_state = self.shared_state.lock().unwrap();
            shared_state.responses.insert(cid, tx);
            shared_state.timeouts.insert(cid, 2000);
        }

        let result = rx.await.map_err(|err| WebSocketError::RecvError(err))?;
        match result {
            Ok(message) => {
                if let Some(error) = message.error {
                    return Err(WebSocketError::ApiError(error));
                }
                return Ok(message);
            }
            Err(error) => {
                return Err(WebSocketError::DeJsonError(error));
            }
        }
    }
}

#[async_trait]
impl<A: SocketAdapter + Send> Socket for WebSocket<A> {
    type Error = WebSocketError<A>;

    /// Register a callback that is dispatched when the socket is closed.
    fn on_closed<T>(&mut self, callback: T)
    where
        T: Fn() + Send + 'static,
    {
        self.shared_state.lock().unwrap().on_closed = Some(Box::new(callback));
    }

    /// Register a callback that is dispatched when the socket is connected
    fn on_connected<T>(&mut self, callback: T)
    where
        T: Fn() + Send + 'static,
    {
        self.shared_state.lock().unwrap().on_connected = Some(Box::new(callback));
    }

    /// Register a callback that is dispatched when a chat message was received
    fn on_received_channel_message<T>(&mut self, callback: T)
    where
        T: Fn(ApiChannelMessage) + Send + 'static,
    {
        self.shared_state
            .lock()
            .unwrap()
            .on_received_channel_message = Some(Box::new(callback));
    }

    /// Register a callback that is dispatched when a presence change for joins and leaves in a chat channel was received.
    fn on_received_channel_presence<T>(&mut self, callback: T)
    where
        T: Fn(ChannelPresenceEvent) + Send + 'static,
    {
        self.shared_state
            .lock()
            .unwrap()
            .on_received_channel_presence = Some(Box::new(callback));
    }

    /// Register a callback that is dispatched when an error is received.
    fn on_received_error<T>(&mut self, callback: T)
    where
        T: Fn(Error) + Send + 'static,
    {
        self.shared_state.lock().unwrap().on_received_error = Some(Box::new(callback));
    }

    /// Register a callback that is dispatched when a matchmaker matched the user.
    fn on_received_matchmaker_matched<T>(&mut self, callback: T)
    where
        T: Fn(MatchmakerMatched) + Send + 'static,
    {
        self.shared_state
            .lock()
            .unwrap()
            .on_received_matchmaker_matched = Some(Box::new(callback));
    }

    /// Register a callback that is dispatched when receiving a match state message
    fn on_received_match_state<T>(&mut self, callback: T)
    where
        T: Fn(MatchData) + Send + 'static,
    {
        self.shared_state.lock().unwrap().on_received_match_state = Some(Box::new(callback));
    }

    /// Register a callback that is dispatched when players join or leave a match.
    fn on_received_match_presence<T>(&mut self, callback: T)
    where
        T: Fn(MatchPresenceEvent) + Send + 'static,
    {
        self.shared_state.lock().unwrap().on_received_match_presence = Some(Box::new(callback));
    }

    /// Register a callback that is dispatched when a notification is received
    fn on_received_notification<T>(&mut self, callback: T)
    where
        T: Fn(ApiNotification) + Send + 'static,
    {
        self.shared_state.lock().unwrap().on_received_notification = Some(Box::new(callback));
    }

    /// Register a callback that is dispatched when a party is closed.
    fn on_received_party_close<T>(&mut self, callback: T)
    where
        T: Fn(PartyClose) + Send + 'static,
    {
        self.shared_state.lock().unwrap().on_received_party_close = Some(Box::new(callback));
    }

    /// Register a callback that is dispatched when a party data is received.
    fn on_received_party_data<T>(&mut self, callback: T)
    where
        T: Fn(PartyData) + Send + 'static,
    {
        self.shared_state.lock().unwrap().on_received_party_data = Some(Box::new(callback));
    }

    /// Register a callback that is dispatched when a party join request is received.
    fn on_received_party_join_request<T>(&mut self, callback: T)
    where
        T: Fn(PartyJoinRequest) + Send + 'static,
    {
        self.shared_state
            .lock()
            .unwrap()
            .on_received_party_join_request = Some(Box::new(callback));
    }

    /// Register a callback that is dispatched when a party leader message is received.
    fn on_received_party_leader<T>(&mut self, callback: T)
    where
        T: Fn(PartyLeader) + Send + 'static,
    {
        self.shared_state.lock().unwrap().on_received_party_leader = Some(Box::new(callback));
    }

    /// Register a callback that is dispatched when users join or leave a party.
    fn on_received_party_presence<T>(&mut self, callback: T)
    where
        T: Fn(PartyPresenceEvent) + Send + 'static,
    {
        self.shared_state.lock().unwrap().on_received_party_presence = Some(Box::new(callback));
    }

    /// Register a callback that is dispatched when users update their online status.
    fn on_received_status_presence<T>(&mut self, callback: T)
    where
        T: Fn(StatusPresenceEvent) + Send + 'static,
    {
        self.shared_state
            .lock()
            .unwrap()
            .on_received_status_presence = Some(Box::new(callback));
    }

    /// Register a callback that is dispatched when users join or leave a realtime stream.
    fn on_received_stream_presence<T>(&mut self, callback: T)
    where
        T: Fn(StreamPresenceEvent) + Send + 'static,
    {
        self.shared_state
            .lock()
            .unwrap()
            .on_received_stream_presence = Some(Box::new(callback));
    }

    /// Register a callback that is dispatched when realtime stream data is received.
    fn on_received_stream_state<T>(&mut self, callback: T)
    where
        T: Fn(StreamData) + Send + 'static,
    {
        self.shared_state.lock().unwrap().on_received_stream_state = Some(Box::new(callback));
    }

    /// Accept a join request.
    ///
    /// If the party is private, the party leader can accept or reject join requests.
    ///
    /// The user presence can be received by calling [`Self::list_party_join_requests`].
    ///
    /// # Example
    /// ```
    /// #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # run_in_socket_example(async move |client, session, socket| {
    /// let join_requests = socket.list_party_join_requests("party_id").await.expect("Failed to list join requests");
    /// for presence in join_requests.presences.iter() {
    ///     socket.accept_party_member("party_id", presence).await.expect("Failed to accept party member")
    /// }
    /// # })
    /// ```
    async fn accept_party_member(
        &self,
        party_id: &str,
        user_presence: &UserPresence,
    ) -> Result<(), Self::Error> {
        let (mut envelope, cid) = self.make_envelope_with_cid();
        envelope.party_accept = Some(PartyAccept {
            party_id: party_id.to_owned(),
            presence: user_presence.clone(),
        });

        let json = envelope.serialize_json();
        self.send(&json, false)?;

        self.wait_response(cid).await?;
        Ok(())
    }

    /// Join the matchmaker pool and search for opponents on the server.
    async fn add_matchmaker_manual(
        &self,
        query: &str,
        min_count: Option<i32>,
        max_count: Option<i32>,
        string_properties: HashMap<String, String>,
        numeric_properties: HashMap<String, f64>,
    ) -> Result<MatchmakerTicket, Self::Error> {
        let (mut envelope, cid) = self.make_envelope_with_cid();
        envelope.matchmaker_add = Some(MatchmakerAdd {
            query: query.to_owned(),
            min_count: min_count.unwrap_or(2),
            max_count: max_count.unwrap_or(8),
            numeric_properties,
            string_properties,
        });

        let json = envelope.serialize_json();
        self.send(&json, false)?;

        let envelope = self.wait_response(cid).await?;

        Ok(envelope.matchmaker_ticket.unwrap())
    }

    /// Join the matchmaker pool and search for opponents on the server.
    async fn add_matchmaker(
        &self,
        matchmaker: &Matchmaker,
    ) -> Result<MatchmakerTicket, Self::Error> {
        self.add_matchmaker_manual(
            &matchmaker.query,
            Some(matchmaker.min_count),
            Some(matchmaker.max_count),
            matchmaker.string_properties.clone(),
            matchmaker.numeric_properties.clone(),
        )
        .await
    }

    /// Join the matchmaker pool and search for opponents on the server as a party.
    async fn add_matchmaker_party(
        &self,
        party_id: &str,
        query: &str,
        min_count: i32,
        max_count: i32,
        string_properties: HashMap<String, String>,
        numeric_properties: HashMap<String, f64>,
    ) -> Result<PartyMatchmakerTicket, Self::Error> {
        let (mut envelope, cid) = self.make_envelope_with_cid();
        envelope.party_matchmaker_add = Some(PartyMatchmakerAdd {
            query: query.to_owned(),
            min_count: min_count,
            max_count: max_count,
            numeric_properties,
            string_properties,
            party_id: party_id.to_owned(),
        });

        let json = envelope.serialize_json();
        self.send(&json, false)?;

        let envelope = self.wait_response(cid).await?;

        Ok(envelope.party_matchmaker_ticket.unwrap())
    }

    /// Close the party.
    ///
    /// Only the party leader can close a party.
    ///
    /// # Example
    /// ```
    /// #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # run_in_socket_example(async move |client, session, socket| {
    /// socket.close_party("party_id").await.expect("Failed to close party");
    /// # })
    /// ```
    async fn close_party(&self, party_id: &str) -> Result<(), Self::Error> {
        let (mut envelope, cid) = self.make_envelope_with_cid();
        envelope.party_close = Some(PartyClose {
            party_id: party_id.to_owned(),
        });

        let json = envelope.serialize_json();
        self.send(&json, false)?;

        self.wait_response(cid).await?;

        Ok(())
    }

    /// Close the socket connection to the server.
    async fn close(&self) -> Result<(), Self::Error> {
        todo!()
    }

    /// Connect to the server.
    ///
    /// If `appear_online` is false, no status updates will be sent to other clients.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # use std::collections::HashMap;
    /// # run_in_socket_example(async move |client, session, socket| {
    /// socket.connect(&session, true, -1).await;
    /// # Ok(())
    /// # });
    /// ```
    async fn connect(&self, session: &Session, appear_online: bool, connect_timeout: i32) {
        let ws_url = "ws://127.0.0.1";
        let port = 7350;

        let ws_addr = format!(
            "{}:{}/ws?lang=en&status={}&token={}",
            ws_url,
            port,
            appear_online,
            session.get_auth_token(),
        );

        let (tx, rx) = oneshot::channel();

        self.shared_state.lock().unwrap().connected.push(tx);

        self.adapter
            .lock()
            .unwrap()
            .connect(&ws_addr, connect_timeout);

        let result = rx.await;
        if let Err(err) = result {
            error!("connect: RecvError: {}", err);
        }
    }

    /// Create a multiplayer match on the server
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # use std::collections::HashMap;
    /// # run_in_socket_example(async move |client, session, socket| {
    /// let multiplayer_match = socket.create_match().await
    ///     .expect("Failed to create match");
    /// println!("Created match with id: {}", multiplayer_match.match_id);
    /// # Ok(())
    /// # });
    /// ```
    async fn create_match(&self) -> Result<Match, Self::Error> {
        let (mut envelope, cid) = self.make_envelope_with_cid();
        envelope.match_create = Some(MatchCreate {});

        let json = envelope.serialize_json();
        self.send(&json, false)?;

        let envelope = self.wait_response(cid).await?;

        Ok(envelope.new_match.unwrap())
    }

    /// Create a multiplayer party.
    ///
    /// If the party is open, anybody can join.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # use std::collections::HashMap;
    /// # run_in_socket_example(async move |client, session, socket| {
    /// let party = socket.create_party(true, 10).await
    ///     .expect("Failed to create a party");
    /// println!("Created party with id: {}", party.party_id);
    /// # Ok(())
    /// # });
    /// ```
    async fn create_party(&self, open: bool, max_size: i32) -> Result<Party, Self::Error> {
        let (mut envelope, cid) = self.make_envelope_with_cid();
        envelope.party_create = Some(PartyCreate { max_size, open });

        let json = envelope.serialize_json();
        self.send(&json, false)?;

        let result_envelope = self.wait_response(cid).await?;
        Ok(result_envelope.party.unwrap())
    }

    /// Follow users to receive their status updates
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # run_in_socket_example(async move |client, session, socket| {
    /// socket.follow_users(&["userid1"], &["username2"]).await
    ///     .expect("Failed to follow users");
    /// # Ok(())
    /// # });
    /// ```
    async fn follow_users(
        &self,
        user_ids: &[&str],
        usernames: &[&str],
    ) -> Result<Status, Self::Error> {
        let (mut envelope, cid) = self.make_envelope_with_cid();
        envelope.status_follow = Some(StatusFollow {
            user_ids: str_slice_to_owned(user_ids),
            usernames: str_slice_to_owned(usernames),
        });

        let json = envelope.serialize_json();
        self.send(&json, false)?;

        let result_envelope = self.wait_response(cid).await?;
        Ok(result_envelope.status.unwrap())
    }

    /// Join a chat channel on the server.
    ///
    /// There are three channel types. Room (1), DirectMessage (2) and Group (3).
    /// The `room_name` is the user id for a direct message, the group id for a group message and the room name otherwise.
    ///
    /// Set `persistence` to `false` to only receive messages while online.
    /// Otherwise messages sent through channels are saved to the database and available in the message history later.
    ///
    /// Set `hidden` to `true` to hide your channel presence when connecting, so you will not generate join/leave notifications
    /// and will not appear in listings of channel members. You can still send and receive messages.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # use std::collections::HashMap;
    /// # run_in_socket_example(async move |client, session, socket| {
    /// let chat = socket.join_chat("RoomName", 1, false, false).await
    ///     .expect("Failed join chat");
    /// println!("Joined chat: {}", chat.room_name);
    /// # Ok(())
    /// # });
    /// ```
    async fn join_chat(
        &self,
        room_name: &str,
        channel_type: i32,
        persistence: bool,
        hidden: bool,
    ) -> Result<Channel, Self::Error> {
        let (mut envelope, cid) = self.make_envelope_with_cid();
        envelope.channel_join = Some(ChannelJoin {
            channel_type,
            hidden,
            persistence,
            target: room_name.to_owned(),
        });

        let json = envelope.serialize_json();
        self.send(&json, false)?;

        let result_envelope = self.wait_response(cid).await?;
        Ok(result_envelope.channel.unwrap())
    }

    /// Join a party on the server.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # use std::collections::HashMap;
    /// # run_in_socket_example(async move |client, session, socket| {
    /// socket.join_party("party_id").await
    ///     .expect("Failed join party");
    /// # Ok(())
    /// # });
    /// ```
    async fn join_party(&self, party_id: &str) -> Result<(), Self::Error> {
        let (mut envelope, cid) = self.make_envelope_with_cid();
        envelope.party_join = Some(PartyJoin {
            party_id: party_id.to_owned(),
        });

        let json = envelope.serialize_json();
        self.send(&json, false)?;

        self.wait_response(cid).await?;
        Ok(())
    }

    /// Join a multiplayer match with the matchmaker matched object.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// use std::sync::mpsc::channel;
    /// # run_in_socket_example(async move |client, session, mut socket| {
    /// let (tx_matched, rx_matched) = channel();
    /// socket.on_received_matchmaker_matched(move |matched| {
    ///     tx_matched.send(matched);
    /// });
    /// // Wait for match
    /// let matched = rx_matched.try_recv().expect("Failed to receive match");
    /// socket.join_match(matched).await.expect("Failed to join match");
    /// # Ok(())
    /// # });
    /// ```
    async fn join_match(&self, matched: MatchmakerMatched) -> Result<Match, Self::Error> {
        let (mut envelope, cid) = self.make_envelope_with_cid();
        envelope.match_join = Some(MatchJoin {
            token: matched.token,
            match_id: matched.match_id,
            metadata: HashMap::new(),
        });

        let json = envelope.serialize_json();
        self.send(&json, false)?;

        let result_envelope = self.wait_response(cid).await?;
        Ok(result_envelope.new_match.unwrap())
    }

    /// Join a multiplayer match with ID
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # use std::collections::HashMap;
    /// # run_in_socket_example(async move |client, session, mut socket| {
    /// socket.join_match_by_id("match_id", HashMap::new()).await.expect("Failed to join match");
    /// # Ok(())
    /// # });
    /// ```
    async fn join_match_by_id(
        &self,
        match_id: &str,
        metadata: HashMap<String, String>,
    ) -> Result<Match, Self::Error> {
        let (mut envelope, cid) = self.make_envelope_with_cid();
        envelope.match_join = Some(MatchJoin {
            match_id: Some(match_id.to_owned()),
            token: None,
            metadata,
        });

        let json = envelope.serialize_json();
        self.send(&json, false)?;

        let result_envelope = self.wait_response(cid).await?;
        Ok(result_envelope.new_match.unwrap())
    }

    /// Leave a chat channel
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # use std::collections::HashMap;
    /// # run_in_socket_example(async move |client, session, socket| {
    /// socket.leave_chat("chat_id").await.expect("Failed to leave chat");
    /// # Ok(())
    /// # });
    /// ```
    async fn leave_chat(&self, channel_id: &str) -> Result<(), Self::Error> {
        let mut envelope = self.make_envelope();
        envelope.channel_leave = Some(ChannelLeave {
            channel_id: channel_id.to_owned(),
        });

        let json = envelope.serialize_json();
        self.send(&json, false)
    }

    /// Leave a match
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # use std::collections::HashMap;
    /// # run_in_socket_example(async move |client, session, socket| {
    /// socket.leave_match("match_id").await.expect("Failed to leave match");
    /// # Ok(())
    /// # });
    /// ```
    async fn leave_match(&self, match_id: &str) -> Result<(), Self::Error> {
        let mut envelope = self.make_envelope();
        envelope.match_leave = Some(MatchLeave {
            match_id: match_id.to_owned(),
        });

        let json = envelope.serialize_json();
        self.send(&json, false)
    }

    /// Leave a party
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # use std::collections::HashMap;
    /// # run_in_socket_example(async move |client, session, socket| {
    /// socket.leave_party("party_id").await.expect("Failed to leave party");
    /// # Ok(())
    /// # });
    /// ```
    async fn leave_party(&self, party_id: &str) -> Result<(), Self::Error> {
        let (mut envelope, cid) = self.make_envelope_with_cid();
        envelope.party_leave = Some(PartyLeave {
            party_id: party_id.to_owned(),
        });

        let json = envelope.serialize_json();
        self.send(&json, false)?;

        self.wait_response(cid).await?;
        Ok(())
    }

    /// List party join requests
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # use std::collections::HashMap;
    /// # run_in_socket_example(async move |client, session, socket| {
    /// let join_requests = socket.list_party_join_requests("party_id").await.expect("Failed to list party join requests");
    /// for presence in join_requests.presences.iter() {
    ///     socket.accept_party_member("party_id", &presence).await.expect("Failed to accept party member");
    /// }
    /// # Ok(())
    /// # });
    /// ```
    async fn list_party_join_requests(
        &self,
        party_id: &str,
    ) -> Result<PartyJoinRequest, Self::Error> {
        let (mut envelope, cid) = self.make_envelope_with_cid();
        envelope.party_join_request_list = Some(PartyJoinRequestList {
            party_id: party_id.to_owned(),
        });

        let json = envelope.serialize_json();
        self.send(&json, false)?;

        let result_envelope = self.wait_response(cid).await?;
        Ok(result_envelope.party_join_request.unwrap())
    }

    /// Promote a party member
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # use std::collections::HashMap;
    /// use std::sync::mpsc::channel;
    /// # run_in_socket_example(async move |client, session, mut socket| {
    /// let (tx_party_presence, rx_party_presence) = channel();
    /// socket.on_received_party_presence(move |presence| {
    ///     tx_party_presence.send(presence);
    /// });
    /// let presence = rx_party_presence.try_recv().expect("Failed to receive party presence")
    ///     .joins.remove(0);
    /// socket.promote_party_member("party_id", presence).await.expect("Failed to list party join requests");
    /// # Ok(())
    /// # });
    /// ```
    async fn promote_party_member(
        &self,
        party_id: &str,
        party_member: UserPresence,
    ) -> Result<(), Self::Error> {
        let (mut envelope, cid) = self.make_envelope_with_cid();
        envelope.party_promote = Some(PartyPromote {
            party_id: party_id.to_owned(),
            presence: party_member,
        });

        let json = envelope.serialize_json();
        self.send(&json, false)?;

        self.wait_response(cid).await?;
        Ok(())
    }

    /// Remove a chat message from a channel
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # use std::collections::HashMap;
    /// # run_in_socket_example(async move |client, session, socket| {
    /// socket.remove_chat_message("channel_id", "message_id").await.expect("Failed to remove chat message");
    /// # Ok(())
    /// # });
    /// ```
    async fn remove_chat_message(
        &self,
        channel_id: &str,
        message_id: &str,
    ) -> Result<ChannelMessageAck, Self::Error> {
        let (mut envelope, cid) = self.make_envelope_with_cid();
        envelope.channel_message_remove = Some(ChannelMesageRemove {
            channel_id: channel_id.to_owned(),
            message_id: message_id.to_owned(),
        });

        let json = envelope.serialize_json();
        self.send(&json, false)?;

        let result_envelope = self.wait_response(cid).await?;
        Ok(result_envelope.channel_message_ack.unwrap())
    }

    /// Leave the matchmaker pool with the ticket
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # use std::collections::HashMap;
    /// use nakama_rs::matchmaker::Matchmaker;
    /// # run_in_socket_example(async move |client, session, socket| {
    /// let matchmaker = Matchmaker::new();
    /// let ticket = socket.add_matchmaker(&matchmaker).await.expect("Failed to add matchmaker");
    /// socket.remove_matchmaker(&ticket.ticket).await.expect("Failed to remove matchmaker");
    /// # Ok(())
    /// # });
    /// ```
    async fn remove_matchmaker(&self, ticket: &str) -> Result<(), Self::Error> {
        let mut envelope = self.make_envelope();
        envelope.matchmaker_remove = Some(MatchmakerRemove {
            ticket: ticket.to_owned(),
        });

        let json = envelope.serialize_json();
        self.send(&json, false)
    }

    /// Leave the party matchmaker pool with the ticket
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # use std::collections::HashMap;
    /// use nakama_rs::matchmaker::Matchmaker;
    /// # run_in_socket_example(async move |client, session, socket| {
    /// let matchmaker = Matchmaker::new();
    /// let ticket = socket.add_matchmaker_party("party_id", "", 2, 8, HashMap::new(), HashMap::new()).await.expect("Failed to add party matchmaker");
    /// socket.remove_matchmaker_party(&ticket.party_id, &ticket.ticket).await.expect("Failed to remove party matchmaker");
    /// # Ok(())
    /// # });
    /// ```
    async fn remove_matchmaker_party(
        &self,
        party_id: &str,
        ticket: &str,
    ) -> Result<(), Self::Error> {
        let mut envelope = self.make_envelope();
        envelope.party_matchmaker_remove = Some(PartyMatchmakerRemove {
            party_id: party_id.to_owned(),
            ticket: ticket.to_owned(),
        });

        let json = envelope.serialize_json();
        self.send(&json, false)
    }

    /// Remove a party member
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # use std::collections::HashMap;
    /// # run_in_socket_example(async move |client, session, socket| {
    /// socket.remove_party_member("party_id", user_presence).await.expect("Failed to remove a party member");
    /// # Ok(())
    /// # });
    /// ```
    async fn remove_party_member(
        &self,
        party_id: &str,
        presence: UserPresence,
    ) -> Result<(), Self::Error> {
        let (mut envelope, cid) = self.make_envelope_with_cid();
        envelope.party_remove = Some(PartyRemove {
            party_id: party_id.to_owned(),
            presence,
        });

        let json = envelope.serialize_json();
        self.send(&json, false)?;

        self.wait_response(cid).await?;
        Ok(())
    }

    /// Execute RPC on the server
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # use std::collections::HashMap;
    /// # run_in_socket_example(async move |client, session, socket| {
    /// let result = socket.rpc("rpc_func_name", "Hello World!").await.expect("Failed to execute rpc");
    /// println!("Returned payload: {}", result.payload);
    /// # Ok(())
    /// # });
    /// ```
    async fn rpc(&self, func_id: &str, payload: &str) -> Result<ApiRpc, Self::Error> {
        let (mut envelope, cid) = self.make_envelope_with_cid();
        envelope.rpc = Some(ApiRpc {
            id: func_id.to_owned(),
            http_key: "".to_owned(),
            payload: payload.to_owned(),
        });

        let json = envelope.serialize_json();
        self.send(&json, false)?;

        let result_envelope = self.wait_response(cid).await?;
        Ok(result_envelope.rpc.unwrap())
    }

    /// Execute RPC on the server.
    ///
    /// The payload is passed as a byte slice.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # use std::collections::HashMap;
    /// use nanoserde::SerBin;
    /// #[derive(SerBin)]
    /// struct Message {
    ///     content: String,
    /// }
    /// # run_in_socket_example(async move |client, session, socket| {
    /// let data = Message { content: "Hello World!".to_owned() };
    /// let mut bin_data = vec![];
    /// data.ser_bin(&mut bin_data);
    /// let result = socket.rpc_bytes("rpc_func_name", bin_data.as_ref()).await.expect("Failed to execute rpc");
    /// println!("Returned payload: {}", result.payload);
    /// # Ok(())
    /// # });
    /// ```
    async fn rpc_bytes(&self, func_id: &str, _payload: &[u8]) -> Result<ApiRpc, Self::Error> {
        let (mut envelope, cid) = self.make_envelope_with_cid();
        envelope.rpc = Some(ApiRpc {
            id: func_id.to_owned(),
            http_key: "".to_owned(),
            // TODO: How to convert to string
            payload: "".to_owned(),
        });

        let json = envelope.serialize_json();
        self.send(&json, false)?;

        let result_envelope = self.wait_response(cid).await?;
        Ok(result_envelope.rpc.unwrap())
    }

    /// Send data to a multiplayer match on the server.
    ///
    /// If no presences are specified, the data is sent to all presences. Otherwise
    /// only to the specified presences.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # use std::collections::HashMap;
    /// use nanoserde::SerBin;
    /// #[derive(SerBin)]
    /// struct Command {
    ///     velocity: i32,
    /// }
    /// # run_in_socket_example(async move |client, session, socket| {
    /// let data = Command { velocity: 100 };
    /// let mut bin_data = vec![];
    /// data.ser_bin(&mut bin_data);
    /// socket.send_match_state("match_id", 1, bin_data.as_ref(), &[]).await.expect("Failed to send match state");
    /// # Ok(())
    /// # });
    /// ```
    async fn send_match_state(
        &self,
        match_id: &str,
        op_code: i64,
        state: &[u8],
        presences: &[UserPresence],
    ) -> Result<(), Self::Error> {
        let mut envelope = self.make_envelope();
        envelope.match_data_send = Some(MatchDataSend {
            match_id: match_id.to_owned(),
            op_code,
            data: state.to_vec(),
            presences: presences.to_vec(),
            // TODO: Reliable?
            reliable: false,
        });

        let json = envelope.serialize_json();
        self.send(&json, false)
    }

    /// Send data to a party.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # use std::collections::HashMap;
    /// use nanoserde::SerBin;
    /// #[derive(SerBin)]
    /// struct Command {
    ///     velocity: i32,
    /// }
    /// # run_in_socket_example(async move |client, session, socket| {
    /// let data = Command { velocity: 100 };
    /// let mut bin_data = vec![];
    /// data.ser_bin(&mut bin_data);
    /// socket.send_party_data("match_id", 1, bin_data.as_ref()).await.expect("Failed to send party data");
    /// # Ok(())
    /// # });
    /// ```
    async fn send_party_data(
        &self,
        party_id: &str,
        op_code: i64,
        data: &[u8],
    ) -> Result<(), Self::Error> {
        let mut envelope = self.make_envelope();
        envelope.party_data_send = Some(PartyDataSend {
            party_id: party_id.to_owned(),
            op_code,
            data: base64::encode(data),
        });

        let json = envelope.serialize_json();
        self.send(&json, false)
    }

    /// Unfollow users to stop receiving status updates.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # use std::collections::HashMap;
    /// # run_in_socket_example(async move |client, session, socket| {
    /// socket.unfollow_users(&["userid1", "userid2"]).await.expect("Failed to unfollow users");
    /// # Ok(())
    /// # });
    /// ```
    async fn unfollow_users(&self, user_ids: &[&str]) -> Result<(), Self::Error> {
        let mut envelope = self.make_envelope();
        envelope.status_unfollow = Some(StatusUnfollow {
            user_ids: str_slice_to_owned(user_ids),
        });

        let json = envelope.serialize_json();
        self.send(&json, false)
    }

    /// Update a chat message on a chat channel
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # use std::collections::HashMap;
    /// use nanoserde::SerJson;
    /// #[derive(SerJson)]
    /// struct ChatMessage {
    ///     content: String,
    /// }
    /// # run_in_socket_example(async move |client, session, socket| {
    /// let content = ChatMessage { content: "Hello World!".to_owned() };
    /// let content = content.serialize_json();
    /// socket.update_chat_message("channel_id", "message_id", &content).await.expect("Failed to update chat message");
    /// # Ok(())
    /// # });
    /// ```
    async fn update_chat_message(
        &self,
        channel_id: &str,
        message_id: &str,
        content: &str,
    ) -> Result<ChannelMessageAck, Self::Error> {
        let (mut envelope, cid) = self.make_envelope_with_cid();
        envelope.channel_message_update = Some(ChannelMesageUpdate {
            channel_id: channel_id.to_owned(),
            message_id: message_id.to_owned(),
            content: content.to_owned(),
        });

        let json = envelope.serialize_json();
        self.send(&json, false)?;

        let result_envelope = self.wait_response(cid).await?;
        Ok(result_envelope.channel_message_ack.unwrap())
    }

    /// Update the users status
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # run_in_socket_example(async move |client, session, socket| {
    /// socket.update_status("NewStatus").await.expect("Failed to update status");
    /// # Ok(())
    /// # });
    /// ```
    async fn update_status(&self, status: &str) -> Result<(), Self::Error> {
        let mut envelope = self.make_envelope();
        envelope.status_update = Some(StatusUpdate {
            status: status.to_owned(),
        });

        let json = envelope.serialize_json();
        self.send(&json, false)
    }

    /// Send a chat message on a chat channel
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # use std::collections::HashMap;
    /// use nanoserde::SerJson;
    /// #[derive(SerJson)]
    /// struct ChatMessage {
    ///     content: String,
    /// }
    /// # run_in_socket_example(async move |client, session, socket| {
    /// let content = ChatMessage { content: "Hello World!".to_owned() };
    /// let content = content.serialize_json();
    /// socket.write_chat_message("channel_id", &content).await.expect("Failed to send chat message");
    /// # Ok(())
    /// # });
    /// ```
    async fn write_chat_message(
        &self,
        channel_id: &str,
        content: &str,
    ) -> Result<ChannelMessageAck, Self::Error> {
        let (mut envelope, cid) = self.make_envelope_with_cid();
        envelope.channel_message_send = Some(ChannelMessageSend {
            channel_id: channel_id.to_owned(),
            content: content.to_owned(),
        });

        let json = envelope.serialize_json();
        self.send(&json, false)?;

        let result_envelope = self.wait_response(cid).await?;
        Ok(result_envelope.channel_message_ack.unwrap())
    }
}

#[cfg(test)]
mod test {
    use nanoserde::SerJson;
    #[derive(SerJson)]
    struct TestStruct {
        a: Option<String>,
        b: Option<String>,
        c: Option<String>,
    }
    #[test]
    fn test_serialization() {
        let test_struct = TestStruct {
            a: Some("string".to_owned()),
            b: Some("hello".to_owned()),
            c: None,
        };
        let test_struct2 = TestStruct {
            a: None,
            b: Some("string".to_owned()),
            c: Some("hello".to_owned()),
        };
        let result = test_struct.serialize_json();
        let result2 = test_struct2.serialize_json();

        // This one is correct
        assert_eq!(result2, "{\"b\":\"string\",\"c\":\"hello\"}");
        assert_eq!(result, "{\"a\":\"string\",\"b\":\"hello\"}");
    }
}
