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

use crate::api::{ApiChannelMessage, ApiNotification, ApiNotificationList, ApiRpc};
use crate::matchmaker::Matchmaker;
use crate::session::Session;
use async_trait::async_trait;
use nanoserde::{DeJson, DeJsonErr, DeJsonState, SerJson};
use std::collections::HashMap;
use std::error;
use std::str::Chars;

#[derive(DeJson, SerJson, Debug, Clone, Default)]
#[nserde(transparent)]
pub struct Timestamp(String);

#[derive(DeJson, SerJson, Debug, Clone, Default)]
pub struct Channel {
    pub id: String,
    #[nserde(default)]
    pub presences: Vec<UserPresence>,
    #[nserde(rename = "self")]
    pub _self: UserPresence,
    #[nserde(default)]
    pub room_name: String,
    #[nserde(default)]
    pub group_id: String,
    #[nserde(default)]
    pub user_id_one: String,
    #[nserde(default)]
    pub user_id_two: String,
}

pub enum ChannelJoinType {
    Unspecified = 0,
    Room = 1,
    DirectMessage = 2,
    Group = 3,
}

#[derive(DeJson, SerJson, Debug, Clone, Default)]
pub struct ChannelJoin {
    pub hidden: bool,
    pub persistence: bool,
    pub target: String,
    #[nserde(rename = "type")]
    // TODO: Make ChannelJoinType
    pub channel_type: i32,
}

#[derive(DeJson, SerJson, Debug, Clone, Default)]
pub struct ChannelLeave {
    pub channel_id: String,
}

#[derive(DeJson, SerJson, Debug, Clone, Default)]
pub struct ChannelMessageAck {
    pub channel_id: String,
    pub message_id: String,
    // TODO: What is the code?
    pub code: i32,
    pub username: String,
    pub create_time: Timestamp,
    pub update_time: Timestamp,
    pub persistent: bool,
    #[nserde(default)]
    pub room_name: String,
    #[nserde(default)]
    pub group_id: String,
    #[nserde(default)]
    pub user_id_one: String,
    #[nserde(default)]
    pub user_id_two: String,
}

#[derive(DeJson, SerJson, Debug, Clone, Default)]
pub struct ChannelMessageSend {
    pub channel_id: String,
    pub content: String,
}

#[derive(DeJson, SerJson, Debug, Clone, Default)]
pub struct ChannelMesageUpdate {
    pub channel_id: String,
    pub message_id: String,
    pub content: String,
}

#[derive(DeJson, SerJson, Debug, Clone, Default)]
pub struct ChannelMesageRemove {
    pub channel_id: String,
    pub message_id: String,
}

#[derive(DeJson, SerJson, Debug, Clone, Default)]
pub struct ChannelPresenceEvent {
    pub channel_id: String,
    #[nserde(default)]
    pub joins: Vec<UserPresence>,
    #[nserde(default)]
    pub leaves: Vec<UserPresence>,
    #[nserde(default)]
    pub room_name: String,
    #[nserde(default)]
    pub group_id: String,
    #[nserde(default)]
    pub user_id_one: String,
    #[nserde(default)]
    pub user_id_two: String,
}

pub enum ErrorCode {
    RuntimeException = 0,
    UnrecognizedPayload = 1,
    MissingPayload = 2,
    BadInput = 3,
    MatchNotFound = 4,
    MatchJoinRejected = 5,
    RuntimeFunctionNotFound = 6,
    RuntimeFunctionException = 7,
}

#[derive(DeJson, SerJson, Debug, Clone, Default)]
pub struct Error {
    // TODO: Use ErrorCode
    pub code: i32,
    pub message: String,
    #[nserde(default)]
    pub context: HashMap<String, String>,
}

#[derive(DeJson, SerJson, Debug, Clone, Default)]
pub struct Match {
    pub match_id: String,
    pub authoritative: bool,
    pub label: String,
    pub size: i32,
    pub presences: Vec<UserPresence>,
    #[nserde(rename = "self")]
    pub _self: UserPresence,
}

#[derive(DeJson, SerJson, Debug, Clone, Default)]
pub struct MatchCreate {}

#[derive(DeJson, SerJson, Debug, Clone, Default)]
pub struct MatchData {
    pub match_id: String,
    pub presence: UserPresence,
    pub op_code: i64,
    pub data: Vec<u8>,
    pub reliable: bool,
}

#[derive(DeJson, SerJson, Debug, Clone, Default)]
pub struct MatchDataSend {
    pub match_id: String,
    pub op_code: i64,
    pub data: Vec<u8>,
    pub presences: Vec<UserPresence>,
    pub reliable: bool,
}

#[derive(DeJson, SerJson, Debug, Clone, Default)]
pub struct MatchJoin {
    pub match_id: Option<String>,
    pub token: Option<String>,
    pub metadata: HashMap<String, String>,
}

#[derive(DeJson, SerJson, Debug, Clone, Default)]
pub struct MatchLeave {
    pub match_id: String,
}

#[derive(DeJson, SerJson, Debug, Clone, Default)]
pub struct MatchPresenceEvent {
    pub match_id: String,
    #[nserde(default)]
    pub joins: Vec<UserPresence>,
    #[nserde(default)]
    pub leaves: Vec<UserPresence>,
}

#[derive(DeJson, SerJson, Debug, Clone, Default)]
pub struct MatchmakerAdd {
    pub min_count: i32,
    pub max_count: i32,
    pub query: String,
    pub string_properties: HashMap<String, String>,
    pub numeric_properties: HashMap<String, f64>,
}

#[derive(DeJson, SerJson, Debug, Clone, Default)]
pub struct MatchmakerUser {
    pub presence: UserPresence,
    pub party_id: String,
    pub string_properties: HashMap<String, String>,
    pub numeric_properties: HashMap<String, f64>,
}

#[derive(DeJson, SerJson, Debug, Clone, Default)]
pub struct MatchmakerMatched {
    pub ticket: String,
    pub match_id: Option<String>,
    pub token: Option<String>,
    pub users: Vec<MatchmakerUser>,
    #[nserde(rename = "self")]
    pub _self: MatchmakerUser,
}

#[derive(DeJson, SerJson, Debug, Clone, Default)]
pub struct MatchmakerRemove {
    pub ticket: String,
}

#[derive(DeJson, SerJson, Debug, Clone, Default)]
pub struct MatchmakerTicket {
    pub ticket: String,
}

#[derive(DeJson, SerJson, Debug, Clone, Default)]
pub struct Notifications {
    pub notifications: Vec<ApiNotification>,
}

#[derive(DeJson, SerJson, Debug, Clone, Default)]
pub struct Party {
    pub party_id: String,
    #[nserde(default)]
    pub open: bool,
    pub max_size: i32,
    #[nserde(rename = "self")]
    pub _self: UserPresence,
    pub leader: UserPresence,
    pub presences: Vec<UserPresence>,
}

#[derive(DeJson, SerJson, Debug, Clone, Default)]
pub struct PartyCreate {
    pub open: bool,
    pub max_size: i32,
}

#[derive(DeJson, SerJson, Debug, Clone, Default)]
pub struct PartyJoin {
    pub party_id: String,
}

#[derive(DeJson, SerJson, Debug, Clone, Default)]
pub struct PartyLeave {
    pub party_id: String,
}

#[derive(DeJson, SerJson, Debug, Clone, Default)]
pub struct PartyPromote {
    pub party_id: String,
    pub presence: UserPresence,
}

#[derive(DeJson, SerJson, Debug, Clone, Default)]
pub struct PartyLeader {
    pub party_id: String,
    pub presence: UserPresence,
}

#[derive(DeJson, SerJson, Debug, Clone, Default)]
pub struct PartyAccept {
    pub party_id: String,
    pub presence: UserPresence,
}

#[derive(DeJson, SerJson, Debug, Clone, Default)]
pub struct PartyRemove {
    pub party_id: String,
    pub presence: UserPresence,
}

#[derive(DeJson, SerJson, Debug, Clone, Default)]
pub struct PartyClose {
    pub party_id: String,
}

#[derive(DeJson, SerJson, Debug, Clone, Default)]
pub struct PartyJoinRequestList {
    pub party_id: String,
}

#[derive(DeJson, SerJson, Debug, Clone, Default)]
pub struct PartyJoinRequest {
    pub party_id: String,
    #[nserde(default)]
    pub presences: Vec<UserPresence>,
}

#[derive(DeJson, SerJson, Debug, Clone, Default)]
pub struct PartyMatchmakerAdd {
    pub party_id: String,
    pub min_count: i32,
    pub max_count: i32,
    pub query: String,
    pub string_properties: HashMap<String, String>,
    pub numeric_properties: HashMap<String, f64>,
}

#[derive(DeJson, SerJson, Debug, Clone, Default)]
pub struct PartyMatchmakerRemove {
    pub party_id: String,
    pub ticket: String,
}

#[derive(DeJson, SerJson, Debug, Clone, Default)]
pub struct PartyMatchmakerTicket {
    pub party_id: String,
    pub ticket: String,
}

#[derive(SerJson, Debug, Clone, Default)]
pub struct PartyData {
    pub party_id: String,
    pub presence: UserPresence,
    pub op_code: i64,
    pub data: Vec<u8>,
}

#[derive(DeJson, SerJson, Debug, Clone, Default)]
pub struct PartyDataProxy {
    pub party_id: String,
    pub presence: UserPresence,
    // TODO: Why is this sent as a string?
    pub op_code: String,
    pub data: String,
}

impl DeJson for PartyData {
    fn de_json(state: &mut DeJsonState, input: &mut Chars) -> Result<Self, DeJsonErr> {
        let proxy: PartyDataProxy = DeJson::de_json(state, input)?;
        let data = base64::decode(proxy.data);
        match data {
            Ok(data) => Ok(PartyData {
                party_id: proxy.party_id,
                presence: proxy.presence,
                op_code: proxy.op_code.parse().unwrap(),
                data,
            }),
            Err(err) => {
                Err(nanoserde::DeJsonErr {
                    msg: err.to_string(),
                    // TODO: Correct lines
                    col: 0,
                    line: 0,
                })
            }
        }
    }
}

#[derive(DeJson, SerJson, Debug, Clone, Default)]
pub struct PartyDataSend {
    pub party_id: String,
    pub op_code: i64,
    pub data: String,
}

#[derive(DeJson, SerJson, Debug, Clone, Default)]
pub struct PartyPresenceEvent {
    pub party_id: String,
    #[nserde(default)]
    pub joins: Vec<UserPresence>,
    #[nserde(default)]
    pub leaves: Vec<UserPresence>,
}

#[derive(DeJson, SerJson, Debug, Clone, Default)]
pub struct Ping {}

#[derive(DeJson, SerJson, Debug, Clone, Default)]
pub struct Pong {}

#[derive(DeJson, SerJson, Debug, Clone, Default)]
pub struct Status {
    pub presences: Vec<UserPresence>,
}

#[derive(DeJson, SerJson, Debug, Clone, Default)]
pub struct StatusFollow {
    pub user_ids: Vec<String>,
    pub usernames: Vec<String>,
}

#[derive(DeJson, SerJson, Debug, Clone, Default)]
pub struct StatusPresenceEvent {
    #[nserde(default)]
    pub joins: Vec<UserPresence>,
    #[nserde(default)]
    pub leaves: Vec<UserPresence>,
}

#[derive(DeJson, SerJson, Debug, Clone, Default)]
pub struct StatusUnfollow {
    pub user_ids: Vec<String>,
}

#[derive(DeJson, SerJson, Debug, Clone, Default)]
pub struct StatusUpdate {
    pub status: String,
}

#[derive(DeJson, SerJson, Debug, Clone, Default)]
pub struct Stream {
    pub mode: i32,
    pub subject: String,
    pub subcontext: String,
    pub label: String,
}

#[derive(DeJson, SerJson, Debug, Clone, Default)]
pub struct StreamData {
    pub stream: Stream,
    pub sender: UserPresence,
    pub data: String,
    pub reliable: bool,
}

#[derive(DeJson, SerJson, Debug, Clone, Default)]
pub struct StreamPresenceEvent {
    pub stream: Stream,
    #[nserde(default)]
    pub joins: Vec<UserPresence>,
    #[nserde(default)]
    pub leaves: Vec<UserPresence>,
}

#[derive(DeJson, SerJson, Debug, Clone, Default)]
pub struct UserPresence {
    #[nserde(default)]
    pub persistence: bool,
    #[nserde(default)]
    pub session_id: String,
    #[nserde(default)]
    pub status: String,
    #[nserde(default)]
    pub username: String,
    #[nserde(default)]
    pub user_id: String,
}

#[derive(DeJson, SerJson, Debug, Clone, Default)]
pub struct WebSocketMessageEnvelopeHeader {
    pub cid: Option<String>,
}

#[derive(DeJson, SerJson, Debug, Clone, Default)]
pub struct WebSocketMessageEnvelope {
    pub cid: Option<String>,
    pub channel: Option<Channel>,
    pub channel_join: Option<ChannelJoin>,
    pub channel_leave: Option<ChannelLeave>,
    pub channel_message: Option<ApiChannelMessage>,
    pub channel_message_ack: Option<ChannelMessageAck>,
    pub channel_message_remove: Option<ChannelMesageRemove>,
    pub channel_message_send: Option<ChannelMessageSend>,
    pub channel_message_update: Option<ChannelMesageUpdate>,
    pub channel_presence_event: Option<ChannelPresenceEvent>,
    pub error: Option<Error>,
    pub matchmaker_add: Option<MatchmakerAdd>,
    pub matchmaker_matched: Option<MatchmakerMatched>,
    pub matchmaker_remove: Option<MatchmakerRemove>,
    pub matchmaker_ticket: Option<MatchmakerTicket>,
    #[nserde(rename = "match")]
    pub new_match: Option<Match>,
    pub match_create: Option<MatchCreate>,
    pub match_join: Option<MatchJoin>,
    pub match_leave: Option<MatchLeave>,
    pub match_presence_event: Option<MatchPresenceEvent>,
    pub match_data: Option<MatchData>,
    pub match_data_send: Option<MatchDataSend>,
    pub notifications: Option<ApiNotificationList>,
    pub rpc: Option<ApiRpc>,
    pub status: Option<Status>,
    pub status_follow: Option<StatusFollow>,
    pub status_presence_event: Option<StatusPresenceEvent>,
    pub status_unfollow: Option<StatusUnfollow>,
    pub status_update: Option<StatusUpdate>,
    pub stream_presence_event: Option<StreamPresenceEvent>,
    pub stream_data: Option<StreamData>,
    pub party: Option<Party>,
    pub party_create: Option<PartyCreate>,
    pub party_join: Option<PartyJoin>,
    pub party_leave: Option<PartyLeave>,
    pub party_promote: Option<PartyPromote>,
    pub party_leader: Option<PartyLeader>,
    pub party_accept: Option<PartyAccept>,
    pub party_remove: Option<PartyRemove>,
    pub party_close: Option<PartyClose>,
    pub party_join_request_list: Option<PartyJoinRequestList>,
    pub party_join_request: Option<PartyJoinRequest>,
    pub party_matchmaker_add: Option<PartyMatchmakerAdd>,
    pub party_matchmaker_remove: Option<PartyMatchmakerRemove>,
    pub party_matchmaker_ticket: Option<PartyMatchmakerTicket>,
    pub party_data: Option<PartyData>,
    pub party_data_send: Option<PartyDataSend>,
    pub party_presence_event: Option<PartyPresenceEvent>,
}

#[async_trait]
pub trait Socket {
    type Error: error::Error;

    // It would make sense to have a future here
    fn on_closed<T>(&mut self, callback: T)
    where
        T: Fn() + Send + 'static;

    fn on_connected<T>(&mut self, callback: T)
    where
        T: Fn() + Send + Send + 'static;

    fn on_received_channel_message<T>(&mut self, callback: T)
    where
        T: Fn(ApiChannelMessage) + Send + Send + 'static;

    fn on_received_channel_presence<T>(&mut self, callback: T)
    where
        T: Fn(ChannelPresenceEvent) + Send + Send + 'static;

    fn on_received_error<T>(&mut self, callback: T)
    where
        T: Fn(Error) + Send + Send + 'static;

    fn on_received_matchmaker_matched<T>(&mut self, callback: T)
    where
        T: Fn(MatchmakerMatched) + Send + Send + 'static;

    fn on_received_match_state<T>(&mut self, callback: T)
    where
        T: Fn(MatchData) + Send + Send + 'static;

    fn on_received_match_presence<T>(&mut self, callback: T)
    where
        T: Fn(MatchPresenceEvent) + Send + 'static;

    fn on_received_notification<T>(&mut self, callback: T)
    where
        T: Fn(ApiNotification) + Send + 'static;

    fn on_received_party_close<T>(&mut self, callback: T)
    where
        T: Fn(PartyClose) + Send + 'static;

    fn on_received_party_data<T>(&mut self, callback: T)
    where
        T: Fn(PartyData) + Send + 'static;

    fn on_received_party_join_request<T>(&mut self, callback: T)
    where
        T: Fn(PartyJoinRequest) + Send + 'static;

    fn on_received_party_leader<T>(&mut self, callback: T)
    where
        T: Fn(PartyLeader) + Send + 'static;

    fn on_received_party_presence<T>(&mut self, callback: T)
    where
        T: Fn(PartyPresenceEvent) + Send + 'static;

    fn on_received_status_presence<T>(&mut self, callback: T)
    where
        T: Fn(StatusPresenceEvent) + Send + 'static;

    fn on_received_stream_presence<T>(&mut self, callback: T)
    where
        T: Fn(StreamPresenceEvent) + Send + 'static;

    fn on_received_stream_state<T>(&mut self, callback: T)
    where
        T: Fn(StreamData) + Send + 'static;

    async fn accept_party_member(
        &self,
        party_id: &str,
        user_presence: &UserPresence,
    ) -> Result<(), Self::Error>;

    async fn add_matchmaker_manual(
        &self,
        query: &str,
        min_count: Option<i32>,
        max_count: Option<i32>,
        string_properties: HashMap<String, String>,
        numeric_properties: HashMap<String, f64>,
    ) -> Result<MatchmakerTicket, Self::Error>;

    async fn add_matchmaker(
        &self,
        matchmaker: &Matchmaker,
    ) -> Result<MatchmakerTicket, Self::Error>;

    async fn add_matchmaker_party(
        &self,
        party_id: &str,
        query: &str,
        min_count: i32,
        max_count: i32,
        string_properties: HashMap<String, String>,
        numeric_properties: HashMap<String, f64>,
    ) -> Result<PartyMatchmakerTicket, Self::Error>;

    async fn close_party(&self, party_id: &str) -> Result<(), Self::Error>;

    async fn close(&self) -> Result<(), Self::Error>;

    async fn connect(&self, session: &mut Session, appear_online: bool, connect_timeout: i32);

    async fn create_match(&self) -> Result<Match, Self::Error>;

    async fn create_party(&self, open: bool, max_size: i32) -> Result<Party, Self::Error>;

    async fn follow_users(
        &self,
        user_ids: &[&str],
        usernames: &[&str],
    ) -> Result<Status, Self::Error>;

    async fn join_chat(
        &self,
        room_name: &str,
        channel_type: i32,
        persistence: bool,
        hidden: bool,
    ) -> Result<Channel, Self::Error>;

    async fn join_party(&self, party_id: &str) -> Result<(), Self::Error>;

    async fn join_match(&self, matched: MatchmakerMatched) -> Result<Match, Self::Error>;

    async fn join_match_by_id(
        &self,
        match_id: &str,
        metadata: HashMap<String, String>,
    ) -> Result<Match, Self::Error>;

    async fn leave_chat(&self, channel_id: &str) -> Result<(), Self::Error>;

    async fn leave_match(&self, match_id: &str) -> Result<(), Self::Error>;

    async fn leave_party(&self, party_id: &str) -> Result<(), Self::Error>;

    async fn list_party_join_requests(
        &self,
        party_id: &str,
    ) -> Result<PartyJoinRequest, Self::Error>;

    async fn promote_party_member(
        &self,
        party_id: &str,
        party_member: UserPresence,
    ) -> Result<(), Self::Error>;

    async fn remove_chat_message(
        &self,
        channel_id: &str,
        message_id: &str,
    ) -> Result<ChannelMessageAck, Self::Error>;

    async fn remove_matchmaker(&self, ticket: &str) -> Result<(), Self::Error>;

    async fn remove_matchmaker_party(
        &self,
        party_id: &str,
        ticket: &str,
    ) -> Result<(), Self::Error>;

    async fn remove_party_member(
        &self,
        party_id: &str,
        presence: UserPresence,
    ) -> Result<(), Self::Error>;

    async fn rpc(&self, func_id: &str, payload: &str) -> Result<ApiRpc, Self::Error>;

    async fn rpc_bytes(&self, func_id: &str, payload: &[u8]) -> Result<ApiRpc, Self::Error>;

    async fn send_match_state(
        &self,
        match_id: &str,
        op_code: i64,
        state: &[u8],
        presences: &[UserPresence],
    ) -> Result<(), Self::Error>;

    async fn send_party_data(
        &self,
        party_id: &str,
        op_code: i64,
        data: &[u8],
    ) -> Result<(), Self::Error>;

    async fn unfollow_users(&self, user_ids: &[&str]) -> Result<(), Self::Error>;

    async fn update_chat_message(
        &self,
        channel_id: &str,
        message_id: &str,
        content: &str,
    ) -> Result<ChannelMessageAck, Self::Error>;

    async fn update_status(&self, status: &str) -> Result<(), Self::Error>;

    async fn write_chat_message(
        &self,
        channel_id: &str,
        content: &str,
    ) -> Result<ChannelMessageAck, Self::Error>;
}
