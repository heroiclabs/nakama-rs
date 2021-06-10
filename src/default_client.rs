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

use crate::api;
use crate::api::{
    ApiAccount, ApiAccountApple, ApiAccountCustom, ApiAccountDevice, ApiAccountEmail,
    ApiAccountFacebook, ApiAccountGameCenter, ApiAccountGoogle, ApiAccountSteam,
    ApiChannelMessageList, ApiCreateGroupRequest, ApiDeleteStorageObjectId,
    ApiDeleteStorageObjectsRequest, ApiEvent, ApiFriendList, ApiGroup, ApiGroupList,
    ApiGroupUserList, ApiLeaderboardRecord, ApiLeaderboardRecordList, ApiLinkSteamRequest,
    ApiMatchList, ApiNotificationList, ApiOverrideOperator, ApiReadStorageObjectId,
    ApiReadStorageObjectsRequest, ApiRpc, ApiSessionLogoutRequest, ApiSessionRefreshRequest,
    ApiStorageObjectAcks, ApiStorageObjectList, ApiStorageObjects, ApiTournamentList,
    ApiTournamentRecordList, ApiUpdateAccountRequest, ApiUpdateGroupRequest, ApiUserGroupList,
    ApiUsers, ApiValidatePurchaseAppleRequest, ApiValidatePurchaseGoogleRequest,
    ApiValidatePurchaseHuaweiRequest, ApiValidatePurchaseResponse, ApiWriteStorageObject,
    RestRequest, WriteLeaderboardRecordRequestLeaderboardRecordWrite,
    WriteTournamentRecordRequestTournamentRecordWrite,
};
use crate::api_gen::{ApiSession, ApiWriteStorageObjectsRequest};
use crate::client::Client;
use crate::client_adapter::ClientAdapter;
use crate::http_adapter::RestHttpAdapter;
use crate::session::Session;
use async_trait::async_trait;
use nanoserde::DeJson;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};

pub struct DefaultClient<A: ClientAdapter> {
    adapter: A,
    server_key: String,
}

#[derive(DeJson)]
pub struct ClientError {
    pub error: String,
    pub code: i32,
    pub message: String,
}

impl DefaultClient<RestHttpAdapter> {
    pub fn new_with_adapter() -> DefaultClient<RestHttpAdapter> {
        let adapter = RestHttpAdapter::new("http://127.0.0.1", 7350);
        DefaultClient::new(adapter)
    }
}

impl<A: ClientAdapter + Send + Sync> DefaultClient<A> {
    pub fn new(adapter: A) -> DefaultClient<A> {
        DefaultClient {
            adapter,
            server_key: "defaultkey".to_owned(),
        }
    }

    #[inline]
    async fn send<T: DeJson + Send>(
        &self,
        request: RestRequest<T>,
    ) -> Result<T, DefaultClientError<A>> {
        self.adapter
            .send(request)
            .await
            .map_err(|err| DefaultClientError::HttpAdapterError(err))
    }

    fn map_session(api_session: ApiSession) -> Session {
        println!("{:?}", api_session);
        Session {
            auth_token: api_session.token,
            refresh_token: if api_session.refresh_token.len() == 0 {
                None
            } else {
                Some(api_session.refresh_token)
            },
        }
    }

    async fn _refresh_session(
        &self,
        session: &mut Session,
    ) -> Result<(), <DefaultClient<A> as Client>::Error> {
        // TODO: check expiration
        if let Some(refresh_token) = session.refresh_token.take() {
            let request = api::session_refresh(
                &self.server_key,
                "",
                ApiSessionRefreshRequest {
                    vars: HashMap::new(),
                    token: refresh_token,
                },
            );

            let sess = self.send(request).await;
            let result = sess.map(|s| {
                session.auth_token = s.token;
                session.refresh_token = if s.refresh_token.len() == 0 {
                    None
                } else {
                    Some(s.refresh_token)
                };
            });
            return result;
        }

        Ok(())
    }
}

pub fn str_slice_to_owned(slice: &[&str]) -> Vec<String> {
    slice.iter().map(|id| (*id).to_owned()).collect()
}

pub enum DefaultClientError<A: ClientAdapter> {
    HttpAdapterError(A::Error),
    ClientError(String),
}

impl<A: ClientAdapter> Debug for DefaultClientError<A> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DefaultClientError::HttpAdapterError(err) => std::fmt::Debug::fmt(err, f),
            DefaultClientError::ClientError(err) => std::fmt::Debug::fmt(err, f),
        }
    }
}

impl<A: ClientAdapter> Display for DefaultClientError<A> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

impl<A: ClientAdapter> Error for DefaultClientError<A> {}

#[async_trait]
impl<A: ClientAdapter + Sync + Send> Client for DefaultClient<A> {
    type Error = DefaultClientError<A>;

    async fn add_friends(
        &self,
        session: &mut Session,
        ids: &[&str],
        usernames: &[&str],
    ) -> Result<(), Self::Error> {
        let ids = str_slice_to_owned(ids);
        let usernames = str_slice_to_owned(usernames);
        let request = api::add_friends(&session.auth_token, &ids, &usernames);
        self.send(request).await
    }

    async fn add_group_users(
        &self,
        session: &mut Session,
        group_id: &str,
        ids: &[&str],
    ) -> Result<(), Self::Error> {
        let ids = str_slice_to_owned(ids);
        let request = api::add_group_users(&session.auth_token, group_id, &ids);
        self.send(request).await
    }

    /// Authenticate a user with an Apple ID against the server.
    ///
    /// Authenticate user with the ID `token` received from Apple.
    /// If the user does not exist and `create` is passed, the user is created with the optional `username`.
    /// `vars` can contain extra information that will be bundled in the session token.
    async fn authenticate_apple(
        &self,
        token: &str,
        username: Option<&str>,
        create: bool,
        vars: HashMap<String, String>,
    ) -> Result<Session, Self::Error> {
        let request = api::authenticate_apple(
            &self.server_key,
            "",
            ApiAccountApple {
                token: token.to_owned(),
                vars,
            },
            Some(create),
            username,
        );

        self.send(request)
            .await
            .map(DefaultClient::<A>::map_session)
    }

    /// Authenticate a user with a custom id.
    ///
    /// Authenticate user with a custom identifier usually obtained from an external authentication service.
    /// If the user does not exist and `create` is passed, the user is created with the optional `username`.
    /// `vars` can contain extra information that will be bundled in the session token.
    async fn authenticate_custom(
        &self,
        id: &str,
        username: Option<&str>,
        create: bool,
        vars: HashMap<String, String>,
    ) -> Result<Session, Self::Error> {
        let request = api::authenticate_custom(
            &self.server_key,
            "",
            ApiAccountCustom {
                id: id.to_owned(),
                vars,
            },
            Some(create),
            username,
        );

        self.send(request)
            .await
            .map(DefaultClient::<A>::map_session)
    }

    /// Authenticate a user with a device id.
    ///
    /// TODO: Mention minimum length requirements;
    /// Authenticate user with a device identifier usually obtained from a platform API.
    /// If the user does not exist and `create` is passed, the user is created with the optional `username`.
    /// `vars` can contain extra information that will be bundled in the session token.
    async fn authenticate_device(
        &self,
        id: &str,
        username: Option<&str>,
        create: bool,
        vars: HashMap<String, String>,
    ) -> Result<Session, Self::Error> {
        let request = api::authenticate_device(
            &self.server_key.clone(),
            "",
            ApiAccountDevice {
                id: id.to_owned(),
                vars,
            },
            Some(create),
            username,
        );

        self.send(request)
            .await
            .map(DefaultClient::<A>::map_session)
    }

    async fn authenticate_email(
        &self,
        email: &str,
        password: &str,
        username: Option<&str>,
        create: bool,
        vars: HashMap<String, String>,
    ) -> Result<Session, Self::Error> {
        let request = api::authenticate_email(
            &self.server_key.clone(),
            "",
            ApiAccountEmail {
                email: email.to_owned(),
                password: password.to_owned(),
                vars,
            },
            Some(create),
            username,
        );

        self.send(request)
            .await
            .map(DefaultClient::<A>::map_session)
    }

    async fn authenticate_facebook(
        &self,
        token: &str,
        username: Option<&str>,
        create: bool,
        vars: HashMap<String, String>,
        import: bool,
    ) -> Result<Session, Self::Error> {
        let request = api::authenticate_facebook(
            &self.server_key.clone(),
            "",
            ApiAccountFacebook {
                token: token.to_owned(),
                vars,
            },
            Some(create),
            username,
            Some(import),
        );

        self.send(request)
            .await
            .map(DefaultClient::<A>::map_session)
    }

    async fn authenticate_game_center(
        &self,
        bundle_id: &str,
        player_id: &str,
        public_key_url: &str,
        salt: &str,
        signature: &str,
        timestamp: &str,
        username: Option<&str>,
        create: bool,
        vars: HashMap<String, String>,
    ) -> Result<Session, Self::Error> {
        let request = api::authenticate_game_center(
            &self.server_key.clone(),
            "",
            ApiAccountGameCenter {
                bundle_id: bundle_id.to_owned(),
                player_id: player_id.to_owned(),
                public_key_url: public_key_url.to_owned(),
                salt: salt.to_owned(),
                signature: signature.to_owned(),
                timestamp_seconds: timestamp.to_owned(),
                vars,
            },
            Some(create),
            username,
        );

        self.send(request)
            .await
            .map(DefaultClient::<A>::map_session)
    }

    async fn authenticate_google(
        &self,
        token: &str,
        username: Option<&str>,
        create: bool,
        vars: HashMap<String, String>,
    ) -> Result<Session, Self::Error> {
        let request = api::authenticate_google(
            &self.server_key.clone(),
            "",
            ApiAccountGoogle {
                token: token.to_owned(),
                vars,
            },
            Some(create),
            username,
        );

        self.send(request)
            .await
            .map(DefaultClient::<A>::map_session)
    }

    async fn authenticate_steam(
        &self,
        token: &str,
        username: Option<&str>,
        create: bool,
        vars: HashMap<String, String>,
    ) -> Result<Session, Self::Error> {
        let request = api::authenticate_google(
            &self.server_key.clone(),
            "",
            ApiAccountGoogle {
                token: token.to_owned(),
                vars,
            },
            Some(create),
            username,
        );

        self.send(request)
            .await
            .map(DefaultClient::<A>::map_session)
    }

    async fn ban_group_users(
        &self,
        session: &mut Session,
        group_id: &str,
        user_ids: &[&str],
    ) -> Result<(), Self::Error> {
        let user_ids = str_slice_to_owned(user_ids);
        let request = api::ban_group_users(&session.auth_token, group_id, &user_ids);

        self.send(request).await
    }

    async fn block_friends(
        &self,
        session: &mut Session,
        ids: &[&str],
        usernames: &[&str],
    ) -> Result<(), Self::Error> {
        let ids = str_slice_to_owned(ids);
        let usernames = str_slice_to_owned(usernames);
        let request = api::block_friends(&session.auth_token, &ids, &usernames);

        self.send(request).await
    }

    async fn create_group(
        &self,
        session: &mut Session,
        name: &str,
        description: Option<&str>,
        avatar_url: Option<&str>,
        lang_tag: Option<&str>,
        open: Option<bool>,
        max_count: Option<i32>,
    ) -> Result<ApiGroup, Self::Error> {
        let request = api::create_group(
            &session.auth_token,
            ApiCreateGroupRequest {
                avatar_url: avatar_url.map_or("".to_owned(), |url| url.to_owned()),
                description: description
                    .map_or("".to_owned(), |description| description.to_owned()),
                lang_tag: lang_tag.map_or("".to_owned(), |lang_tag| lang_tag.to_owned()),
                max_count: max_count.unwrap_or(100),
                name: name.to_owned(),
                open: open.unwrap_or(true),
            },
        );

        self.send(request).await
    }

    async fn delete_friends(
        &self,
        session: &mut Session,
        ids: &[&str],
        usernames: &[&str],
    ) -> Result<(), Self::Error> {
        let ids = str_slice_to_owned(ids);
        let usernames = str_slice_to_owned(usernames);
        let request = api::delete_friends(&session.auth_token, &ids, &usernames);

        self.send(request).await
    }

    async fn delete_group(&self, session: &mut Session, group_id: &str) -> Result<(), Self::Error> {
        let request = api::delete_group(&session.auth_token, group_id);
        self.send(request).await
    }

    async fn delete_leaderboard_record(
        &self,
        session: &mut Session,
        leaderboard_id: &str,
    ) -> Result<(), Self::Error> {
        let request = api::delete_leaderboard_record(&session.auth_token, leaderboard_id);
        self.send(request).await
    }

    async fn delete_notifications(
        &self,
        session: &mut Session,
        ids: &[&str],
    ) -> Result<(), Self::Error> {
        let ids = str_slice_to_owned(ids);
        let request = api::delete_notifications(&session.auth_token, &ids);
        self.send(request).await
    }

    async fn delete_storage_objects(
        &self,
        session: &mut Session,
        ids: &[ApiDeleteStorageObjectId],
    ) -> Result<(), Self::Error> {
        let request = api::delete_storage_objects(
            &session.auth_token,
            ApiDeleteStorageObjectsRequest {
                object_ids: ids.to_vec(),
            },
        );
        self.send(request).await
    }

    async fn demote_group_users(
        &self,
        session: &mut Session,
        group_id: &str,
        user_ids: &[&str],
    ) -> Result<(), Self::Error> {
        let user_ids = str_slice_to_owned(user_ids);
        let request = api::demote_group_users(&session.auth_token, group_id, &user_ids);
        self.send(request).await
    }

    async fn event(
        &self,
        session: &mut Session,
        name: &str,
        properties: HashMap<String, String>,
    ) -> Result<(), Self::Error> {
        let request = api::event(
            &session.auth_token,
            ApiEvent {
                name: name.to_owned(),
                timestamp: "".to_owned(),
                external: true,
                properties,
            },
        );
        self.send(request).await
    }

    async fn get_account(&self, session: &mut Session) -> Result<ApiAccount, Self::Error> {
        let request = api::get_account(&session.auth_token);
        self.send(request).await
    }

    async fn get_users(
        &self,
        session: &mut Session,
        ids: &[&str],
        usernames: &[&str],
        facebook_ids: &[&str],
    ) -> Result<ApiUsers, Self::Error> {
        let ids = str_slice_to_owned(ids);
        let usernames = str_slice_to_owned(usernames);
        let facebook_ids = str_slice_to_owned(facebook_ids);
        let request = api::get_users(&session.auth_token, &ids, &usernames, &facebook_ids);
        self.send(request).await
    }

    async fn import_facebook_friends(
        &self,
        session: &mut Session,
        token: &str,
        reset: Option<bool>,
    ) -> Result<(), Self::Error> {
        let request = api::import_facebook_friends(
            &session.auth_token,
            ApiAccountFacebook {
                vars: HashMap::new(),
                token: token.to_owned(),
            },
            reset,
        );
        self.send(request).await
    }

    async fn import_steam_friends(
        &self,
        session: &mut Session,
        token: &str,
        reset: Option<bool>,
    ) -> Result<(), Self::Error> {
        let request = api::import_steam_friends(
            &session.auth_token,
            ApiAccountSteam {
                vars: HashMap::new(),
                token: token.to_owned(),
            },
            reset,
        );
        self.send(request).await
    }

    async fn join_group(&self, session: &mut Session, group_id: &str) -> Result<(), Self::Error> {
        let request = api::join_group(&session.auth_token, group_id);
        self.send(request).await
    }

    async fn join_tournament(
        &self,
        session: &mut Session,
        tournament_id: &str,
    ) -> Result<(), Self::Error> {
        let request = api::join_tournament(&session.auth_token, tournament_id);
        self.send(request).await
    }

    async fn kick_group_users(
        &self,
        session: &mut Session,
        group_id: &str,
        ids: &[&str],
    ) -> Result<(), Self::Error> {
        let ids = str_slice_to_owned(ids);
        let request = api::kick_group_users(&session.auth_token, group_id, &ids);
        self.send(request).await
    }

    async fn leave_group(&self, session: &mut Session, group_id: &str) -> Result<(), Self::Error> {
        let request = api::leave_group(&session.auth_token, group_id);
        self.send(request).await
    }

    async fn link_apple(&self, session: &mut Session, token: &str) -> Result<(), Self::Error> {
        let request = api::link_apple(
            &session.auth_token,
            ApiAccountApple {
                vars: HashMap::new(),
                token: token.to_owned(),
            },
        );
        self.send(request).await
    }

    async fn link_custom(&self, session: &mut Session, id: &str) -> Result<(), Self::Error> {
        let request = api::link_custom(
            &session.auth_token,
            ApiAccountCustom {
                vars: HashMap::new(),
                id: id.to_owned(),
            },
        );
        self.send(request).await
    }

    async fn link_device(&self, session: &mut Session, id: &str) -> Result<(), Self::Error> {
        let request = api::link_device(
            &session.auth_token,
            ApiAccountDevice {
                vars: HashMap::new(),
                id: id.to_owned(),
            },
        );
        self.send(request).await
    }

    async fn link_email(
        &self,
        session: &mut Session,
        email: &str,
        password: &str,
    ) -> Result<(), Self::Error> {
        let request = api::link_email(
            &session.auth_token,
            ApiAccountEmail {
                vars: HashMap::new(),
                email: email.to_owned(),
                password: password.to_owned(),
            },
        );
        self.send(request).await
    }

    async fn link_facebook(
        &self,
        session: &mut Session,
        token: &str,
        import: Option<bool>,
    ) -> Result<(), Self::Error> {
        let request = api::link_facebook(
            &session.auth_token,
            ApiAccountFacebook {
                vars: HashMap::new(),
                token: token.to_owned(),
            },
            import,
        );
        self.send(request).await
    }

    async fn link_game_center(
        &self,
        session: &mut Session,
        bundle_id: &str,
        player_id: &str,
        public_key_url: &str,
        salt: &str,
        signature: &str,
        timestamp: &str,
    ) -> Result<(), Self::Error> {
        let request = api::link_game_center(
            &session.auth_token,
            ApiAccountGameCenter {
                vars: HashMap::new(),
                bundle_id: bundle_id.to_owned(),
                player_id: player_id.to_owned(),
                public_key_url: public_key_url.to_owned(),
                salt: salt.to_owned(),
                signature: signature.to_owned(),
                timestamp_seconds: timestamp.to_owned(),
            },
        );
        self.send(request).await
    }

    async fn link_google(&self, session: &mut Session, token: &str) -> Result<(), Self::Error> {
        let request = api::link_google(
            &session.auth_token,
            ApiAccountGoogle {
                vars: HashMap::new(),
                token: token.to_owned(),
            },
        );
        self.send(request).await
    }

    async fn link_steam(
        &self,
        session: &mut Session,
        token: &str,
        import: bool,
    ) -> Result<(), Self::Error> {
        let request = api::link_steam(
            &session.auth_token,
            ApiLinkSteamRequest {
                account: ApiAccountSteam {
                    vars: HashMap::new(),
                    token: token.to_owned(),
                },
                sync: import,
            },
        );
        self.send(request).await
    }

    async fn list_channel_messages(
        &self,
        session: &mut Session,
        channel_id: &str,
        limit: Option<i32>,
        forward: Option<bool>,
        cursor: Option<&str>,
    ) -> Result<ApiChannelMessageList, Self::Error> {
        let request =
            api::list_channel_messages(&session.auth_token, channel_id, limit, forward, cursor);

        self.send(request).await
    }

    async fn list_friends(
        &self,
        session: &mut Session,
        state: Option<i32>,
        limit: Option<i32>,
        cursor: Option<&str>,
    ) -> Result<ApiFriendList, Self::Error> {
        let request = api::list_friends(&session.auth_token, limit, state, cursor);

        self.send(request).await
    }

    async fn list_group_users(
        &self,
        session: &mut Session,
        group_id: &str,
        state: Option<i32>,
        limit: Option<i32>,
        cursor: Option<&str>,
    ) -> Result<ApiGroupUserList, Self::Error> {
        let request = api::list_group_users(&session.auth_token, group_id, limit, state, cursor);

        self.send(request).await
    }

    async fn list_groups(
        &self,
        session: &mut Session,
        name: Option<&str>,
        limit: Option<i32>,
        cursor: Option<&str>,
    ) -> Result<ApiGroupList, Self::Error> {
        let request = api::list_groups(&session.auth_token, name, cursor, limit);

        self.send(request).await
    }

    async fn list_leaderboard_records(
        &self,
        session: &mut Session,
        leaderboard_id: &str,
        owner_ids: &[&str],
        expiry: Option<&str>,
        limit: Option<i32>,
        cursor: Option<&str>,
    ) -> Result<ApiLeaderboardRecordList, Self::Error> {
        let owner_ids = str_slice_to_owned(owner_ids);
        let request = api::list_leaderboard_records(
            &session.auth_token,
            leaderboard_id,
            &owner_ids,
            limit,
            cursor,
            expiry,
        );

        self.send(request).await
    }

    async fn list_leaderboard_records_around_owner(
        &self,
        session: &mut Session,
        leaderboard_id: &str,
        owner_id: &str,
        expiry: Option<&str>,
        limit: Option<i32>,
    ) -> Result<ApiLeaderboardRecordList, Self::Error> {
        let request = api::list_leaderboard_records_around_owner(
            &session.auth_token,
            leaderboard_id,
            owner_id,
            limit,
            expiry,
        );

        self.send(request).await
    }

    async fn list_matches(
        &self,
        session: &mut Session,
        min: Option<i32>,
        max: Option<i32>,
        limit: Option<i32>,
        authoritative: Option<bool>,
        label: &str,
        query: &str,
    ) -> Result<ApiMatchList, Self::Error> {
        let request = api::list_matches(
            &session.auth_token,
            limit,
            authoritative,
            Some(label),
            min,
            max,
            Some(query),
        );

        self.send(request).await
    }

    async fn list_notifications(
        &self,
        session: &mut Session,
        limit: Option<i32>,
        cacheable_cursor: Option<&str>,
    ) -> Result<ApiNotificationList, Self::Error> {
        let request = api::list_notifications(&session.auth_token, limit, cacheable_cursor);

        self.send(request).await
    }

    async fn list_storage_objects(
        &self,
        session: &mut Session,
        collection: &str,
        limit: Option<i32>,
        cursor: Option<&str>,
    ) -> Result<ApiStorageObjectList, Self::Error> {
        let request =
            api::list_storage_objects(&session.auth_token, collection, None, limit, cursor);

        self.send(request).await
    }

    async fn list_tournament_records_around_owner(
        &self,
        session: &mut Session,
        tournament_id: &str,
        owner_id: &str,
        expiry: Option<&str>,
        limit: Option<i32>,
    ) -> Result<ApiTournamentRecordList, Self::Error> {
        let request = api::list_tournament_records_around_owner(
            &session.auth_token,
            tournament_id,
            owner_id,
            limit,
            expiry,
        );

        self.send(request).await
    }

    async fn list_tournament_records(
        &self,
        session: &mut Session,
        tournament_id: &str,
        owner_ids: &[&str],
        expiry: Option<&str>,
        limit: Option<i32>,
        cursor: Option<&str>,
    ) -> Result<ApiTournamentRecordList, Self::Error> {
        let owner_ids = str_slice_to_owned(owner_ids);
        let request = api::list_tournament_records(
            &session.auth_token,
            tournament_id,
            &owner_ids,
            limit,
            cursor,
            expiry,
        );

        self.send(request).await
    }

    async fn list_tournaments(
        &self,
        session: &mut Session,
        category_start: Option<i32>,
        category_end: Option<i32>,
        start_time: Option<i32>,
        end_time: Option<i32>,
        limit: Option<i32>,
        cursor: Option<&str>,
    ) -> Result<ApiTournamentList, Self::Error> {
        let request = api::list_tournaments(
            &session.auth_token,
            category_start,
            category_end,
            start_time,
            end_time,
            limit,
            cursor,
        );

        self.send(request).await
    }

    async fn list_current_user_groups(
        &self,
        _session: &mut Session,
        _state: Option<i32>,
        _limit: Option<i32>,
        _cursor: Option<&str>,
    ) -> Result<ApiUserGroupList, Self::Error> {
        todo!()
    }

    async fn list_user_groups(
        &self,
        session: &mut Session,
        user_id: &str,
        state: Option<i32>,
        limit: Option<i32>,
        cursor: Option<&str>,
    ) -> Result<ApiUserGroupList, Self::Error> {
        let request = api::list_user_groups(&session.auth_token, user_id, limit, state, cursor);

        self.send(request).await
    }

    async fn list_users_storage_objects(
        &self,
        session: &mut Session,
        collection: &str,
        user_id: &str,
        limit: Option<i32>,
        cursor: Option<&str>,
    ) -> Result<ApiStorageObjectList, Self::Error> {
        let request = api::list_storage_objects(
            &session.auth_token,
            collection,
            Some(user_id),
            limit,
            cursor,
        );

        self.send(request).await
    }

    async fn promote_group_user(
        &self,
        session: &mut Session,
        group_id: &str,
        ids: &[&str],
    ) -> Result<(), Self::Error> {
        let ids = str_slice_to_owned(ids);
        let request = api::promote_group_users(&session.auth_token, group_id, &ids);

        self.send(request).await
    }

    async fn read_storage_objects(
        &self,
        session: &mut Session,
        ids: &[ApiReadStorageObjectId],
    ) -> Result<ApiStorageObjects, Self::Error> {
        let ids = ids.to_vec();
        let request = api::read_storage_objects(
            &session.auth_token,
            ApiReadStorageObjectsRequest { object_ids: ids },
        );

        self.send(request).await
    }

    async fn rpc(
        &self,
        session: &mut Session,
        id: &str,
        payload: Option<&str>,
    ) -> Result<ApiRpc, Self::Error> {
        let request = api::rpc_func2(&session.auth_token, id, payload, None);

        self.send(request).await
    }

    async fn session_logout(&self, session: &mut Session) -> Result<(), Self::Error> {
        let request = api::session_logout(
            &session.auth_token,
            ApiSessionLogoutRequest {
                token: session.auth_token.clone(),
                refresh_token: session.refresh_token.clone().unwrap_or("".to_owned()),
            },
        );

        self.send(request).await
    }

    async fn session_refresh(
        &self,
        session: &mut Session,
        vars: HashMap<String, String>,
    ) -> Result<Session, Self::Error> {
        let request = api::session_refresh(
            &self.server_key,
            "",
            ApiSessionRefreshRequest {
                token: session.auth_token.clone(),
                vars,
            },
        );

        self.send(request)
            .await
            .map(DefaultClient::<A>::map_session)
    }

    async fn unlink_apple(&self, session: &mut Session, token: &str) -> Result<(), Self::Error> {
        let request = api::unlink_apple(
            &session.auth_token,
            ApiAccountApple {
                vars: HashMap::new(),
                token: token.to_owned(),
            },
        );

        self.send(request).await
    }

    async fn unlink_custom(&self, session: &mut Session, id: &str) -> Result<(), Self::Error> {
        let request = api::unlink_custom(
            &session.auth_token,
            ApiAccountCustom {
                vars: HashMap::new(),
                id: id.to_owned(),
            },
        );

        self.send(request).await
    }

    async fn unlink_device(&self, session: &mut Session, id: &str) -> Result<(), Self::Error> {
        let request = api::unlink_device(
            &session.auth_token,
            ApiAccountDevice {
                vars: HashMap::new(),
                id: id.to_owned(),
            },
        );

        self.send(request).await
    }

    async fn unlink_email(
        &self,
        session: &mut Session,
        email: &str,
        password: &str,
    ) -> Result<(), Self::Error> {
        let request = api::unlink_email(
            &session.auth_token,
            ApiAccountEmail {
                vars: HashMap::new(),
                email: email.to_owned(),
                password: password.to_owned(),
            },
        );

        self.send(request).await
    }

    async fn unlink_facebook(&self, session: &mut Session, token: &str) -> Result<(), Self::Error> {
        let request = api::unlink_facebook(
            &session.auth_token,
            ApiAccountFacebook {
                vars: HashMap::new(),
                token: token.to_owned(),
            },
        );

        self.send(request).await
    }

    async fn unlink_game_center(
        &self,
        session: &mut Session,
        bundle_id: &str,
        player_id: &str,
        public_key_url: &str,
        salt: &str,
        signature: &str,
        timestamp: &str,
    ) -> Result<(), Self::Error> {
        let request = api::unlink_game_center(
            &session.auth_token,
            ApiAccountGameCenter {
                vars: HashMap::new(),
                bundle_id: bundle_id.to_owned(),
                player_id: player_id.to_owned(),
                public_key_url: public_key_url.to_owned(),
                salt: salt.to_owned(),
                signature: signature.to_owned(),
                timestamp_seconds: timestamp.to_owned(),
            },
        );

        self.send(request).await
    }

    async fn unlink_google(&self, session: &mut Session, token: &str) -> Result<(), Self::Error> {
        let request = api::unlink_google(
            &session.auth_token,
            ApiAccountGoogle {
                vars: HashMap::new(),
                token: token.to_owned(),
            },
        );

        self.send(request).await
    }

    async fn unlink_steam(&self, session: &mut Session, token: &str) -> Result<(), Self::Error> {
        let request = api::unlink_steam(
            &session.auth_token,
            ApiAccountSteam {
                vars: HashMap::new(),
                token: token.to_owned(),
            },
        );

        self.send(request).await
    }

    async fn update_account(
        &self,
        session: &mut Session,
        username: &str,
        display_name: Option<&str>,
        avatar_url: Option<&str>,
        lang_tag: Option<&str>,
        location: Option<&str>,
        timezone: Option<&str>,
    ) -> Result<(), Self::Error> {
        let request = api::update_account(
            &session.auth_token,
            ApiUpdateAccountRequest {
                avatar_url: avatar_url.map_or("".to_owned(), |url| url.to_owned()),
                lang_tag: lang_tag.map_or("".to_owned(), |lang_tag| lang_tag.to_owned()),
                username: username.to_owned(),
                display_name: display_name
                    .map_or("".to_owned(), |display_name| display_name.to_owned()),
                location: location.map_or("".to_owned(), |location| location.to_owned()),
                timezone: timezone.map_or("".to_owned(), |timezone| timezone.to_owned()),
            },
        );

        self.send(request).await
    }

    async fn update_group(
        &self,
        session: &mut Session,
        group_id: &str,
        name: &str,
        open: bool,
        description: Option<&str>,
        avatar_url: Option<&str>,
        lang_tag: Option<&str>,
    ) -> Result<(), Self::Error> {
        let request = api::update_group(
            &session.auth_token,
            group_id,
            ApiUpdateGroupRequest {
                avatar_url: avatar_url.map_or("".to_owned(), |url| url.to_owned()),
                description: description
                    .map_or("".to_owned(), |description| description.to_owned()),
                group_id: group_id.to_owned(),
                lang_tag: lang_tag.map_or("".to_owned(), |lang_tag| lang_tag.to_owned()),
                name: name.to_owned(),
                open,
            },
        );

        self.send(request).await
    }

    async fn validate_purchase_apple(
        &self,
        session: &mut Session,
        receipt: &str,
    ) -> Result<ApiValidatePurchaseResponse, Self::Error> {
        let request = api::validate_purchase_apple(
            &session.auth_token,
            ApiValidatePurchaseAppleRequest {
                receipt: receipt.to_string(),
            },
        );

        self.send(request).await
    }

    async fn validate_purchase_google(
        &self,
        session: &mut Session,
        receipt: &str,
    ) -> Result<ApiValidatePurchaseResponse, Self::Error> {
        let request = api::validate_purchase_google(
            &session.auth_token,
            ApiValidatePurchaseGoogleRequest {
                purchase: receipt.to_string(),
            },
        );

        self.send(request).await
    }

    async fn validate_purchase_huawei(
        &self,
        session: &mut Session,
        receipt: &str,
        signature: &str,
    ) -> Result<ApiValidatePurchaseResponse, Self::Error> {
        let request = api::validate_purchase_huawei(
            &session.auth_token,
            ApiValidatePurchaseHuaweiRequest {
                purchase: receipt.to_owned(),
                signature: signature.to_owned(),
            },
        );

        self.send(request).await
    }

    async fn write_leaderboard_record(
        &self,
        session: &mut Session,
        leaderboard_id: &str,
        score: i64,
        sub_score: Option<i64>,
        override_operator: Option<ApiOverrideOperator>,
        metadata: Option<&str>,
    ) -> Result<ApiLeaderboardRecord, Self::Error> {
        let operator = override_operator.unwrap_or(ApiOverrideOperator::NO_OVERRIDE);
        let request = api::write_leaderboard_record(
            &session.auth_token,
            leaderboard_id,
            WriteLeaderboardRecordRequestLeaderboardRecordWrite {
                metadata: metadata.unwrap_or("").to_owned(),
                score: score.to_string(),
                subscore: sub_score.map(|sub_score| sub_score.to_string()),
                operator,
            },
        );

        println!("{}", request.body);

        self.send(request).await
    }

    async fn write_storage_objects(
        &self,
        session: &mut Session,
        objects: &[ApiWriteStorageObject],
    ) -> Result<ApiStorageObjectAcks, Self::Error> {
        let request = api::write_storage_objects(
            &session.auth_token,
            ApiWriteStorageObjectsRequest {
                objects: objects.to_vec(),
            },
        );

        self.send(request).await
    }

    async fn write_tournament_record(
        &self,
        session: &mut Session,
        tournament_id: &str,
        score: i64,
        sub_score: Option<i64>,
        override_operator: Option<ApiOverrideOperator>,
        metadata: Option<&str>,
    ) -> Result<ApiLeaderboardRecord, Self::Error> {
        let operator = override_operator.unwrap_or(ApiOverrideOperator::NO_OVERRIDE);
        let request = api::write_tournament_record(
            &session.auth_token,
            tournament_id,
            WriteTournamentRecordRequestTournamentRecordWrite {
                metadata: metadata.map(|str| str.to_owned()),
                score: score.to_string(),
                subscore: sub_score.map(|sub_score| sub_score.to_string()),
                operator,
            },
        );
        println!("{:?}", request);

        self.send(request).await
    }
}
