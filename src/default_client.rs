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

//! The default implementation of the Nakama [`Client`] trait.
//!
//! # General concepts
//! ## Limit and cursor
//! Many functions that list data allow to pass optional `limit` and `cursor` parameters. The first call
//! will return up to `limit` entries. The returned struct contains a `cursor` member that can be passed to the next function call to
//! retrieve more data.
//!
//! If no `limit` is specified, the default `limit` applies.
//!
//! ## Authentication
//! The authentication endpoints allow you to specify using the `create` flag that the account should be created if it does
//! not exist. For that case, you can provide the `username` of the user.
//!
//! You can pass session data that will be bundled in the session token using the `vars` parameter.
//!
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
    CreateLeaderboard, Leaderboard, RestRequest,
    WriteLeaderboardRecordRequestLeaderboardRecordWrite,
    WriteTournamentRecordRequestTournamentRecordWrite,
};
use crate::api_gen::{ApiSession, ApiWriteStorageObjectsRequest};
use crate::client::Client;
use crate::client_adapter::ClientAdapter;
use crate::config::{DEFAULT_HOST, DEFAULT_PORT, DEFAULT_SERVER_KEY, DEFAULT_SERVER_PASSWORD};
use crate::http_adapter::RestHttpAdapter;
use crate::session::Session;
use crate::types::SortOrder;
use async_trait::async_trait;
use nanoserde::DeJson;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};

pub struct DefaultClient<A: ClientAdapter> {
    adapter: A,
    server_key: String,
    server_password: String,
}

impl<A: ClientAdapter + Clone> Clone for DefaultClient<A> {
    fn clone(&self) -> Self {
        DefaultClient {
            adapter: self.adapter.clone(),
            server_key: self.server_key.clone(),
            server_password: self.server_password.clone(),
        }
    }
}

#[derive(DeJson)]
pub struct ClientError {
    pub error: String,
    pub code: i32,
    pub message: String,
}

impl DefaultClient<RestHttpAdapter> {
    pub fn new_with_adapter(
        host: &str,
        port: u32,
        server_key: &str,
        server_password: &str,
    ) -> DefaultClient<RestHttpAdapter> {
        let adapter = RestHttpAdapter::new(host, port);
        DefaultClient::new(adapter, server_key, server_password)
    }

    pub fn new_with_adapter_and_defaults() -> DefaultClient<RestHttpAdapter> {
        let adapter = RestHttpAdapter::new(DEFAULT_HOST, DEFAULT_PORT);
        DefaultClient::new(adapter, DEFAULT_SERVER_KEY, DEFAULT_SERVER_PASSWORD)
    }
}

impl<A: ClientAdapter + Send + Sync> DefaultClient<A> {
    pub fn new(adapter: A, server_key: &str, server_password: &str) -> DefaultClient<A> {
        DefaultClient {
            adapter,
            server_key: server_key.to_owned(),
            server_password: server_password.to_owned(),
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
        Session::new(&api_session.token, &api_session.refresh_token)
    }

    async fn refresh_session(
        &self,
        session: &Session,
    ) -> Result<(), <DefaultClient<A> as Client>::Error> {
        let refresh_token = session.get_refresh_token();
        let vars = session.vars();
        let vars = vars
            .iter()
            .map(|(key, val)| (key.as_str(), val.as_str()))
            .collect();
        if session.get_auto_refresh() && refresh_token.is_some() && session.will_expire_soon() {
            return self.session_refresh(session, vars).await;
        }

        Ok(())
    }
}

pub fn str_slice_to_owned(slice: &[&str]) -> Vec<String> {
    slice.iter().map(|id| (*id).to_owned()).collect()
}

pub fn string_map_to_owned_string_map(vars: HashMap<&str, &str>) -> HashMap<String, String> {
    vars.iter()
        .map(|(&k, &v)| (k.to_owned(), v.to_owned()))
        .collect()
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

    /// Add friends by id or username.
    ///
    /// Either accept a friend invite or send a friend invite to the specified users
    /// based on their ids or usernames.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # run_in_example(async move |client, session| {
    /// client.add_friends(&session, &["friend_id"], &["friend_user_id"]).await
    /// # });
    /// ```
    async fn add_friends(
        &self,
        session: &Session,
        ids: &[&str],
        usernames: &[&str],
    ) -> Result<(), Self::Error> {
        let ids = str_slice_to_owned(ids);
        let usernames = str_slice_to_owned(usernames);
        let request = api::add_friends(&session.get_auth_token(), &ids, &usernames);
        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// Add users to a group.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # run_in_example(async move |client, session| {
    /// let group = client.create_group(&session, "NewGroup", None, None, None, None, None).await?;
    /// client.add_group_users(&session, &group.id, &["useridtoadd"]).await
    /// # });
    /// ```
    async fn add_group_users(
        &self,
        session: &Session,
        group_id: &str,
        ids: &[&str],
    ) -> Result<(), Self::Error> {
        let ids = str_slice_to_owned(ids);
        let request = api::add_group_users(&session.get_auth_token(), group_id, &ids);
        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// Authenticate a user with an Apple ID against the server.
    ///
    /// Authenticate user with the ID `token` received from Apple.
    ///
    /// See [Authentication](index.html#authentication) for a description of the `username`, `create` and `vars` parameters.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # use std::collections::HashMap;
    /// # run_in_example(async move |client, session| {
    /// let session = client.authenticate_apple("apple_token", Some("Username"), true, HashMap::new()).await
    ///     .expect("Failed to authenticate user");
    /// # Ok(())
    /// # });
    /// ```
    async fn authenticate_apple(
        &self,
        token: &str,
        username: Option<&str>,
        create: bool,
        vars: HashMap<&str, &str>,
    ) -> Result<Session, Self::Error> {
        let request = api::authenticate_apple(
            &self.server_key,
            &self.server_password,
            ApiAccountApple {
                token: token.to_owned(),
                vars: string_map_to_owned_string_map(vars),
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
    ///
    /// See [Authentication](index.html#authentication) for a description of the `username`, `create` and `vars` parameters.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # use std::collections::HashMap;
    /// # run_in_example(async move |client, session| {
    /// let session = client.authenticate_custom("custom_token", Some("Username"), true, HashMap::new()).await
    ///     .expect("Failed to authenticate user");
    /// # Ok(())
    /// # });
    /// ```
    ///
    async fn authenticate_custom(
        &self,
        id: &str,
        username: Option<&str>,
        create: bool,
        vars: HashMap<&str, &str>,
    ) -> Result<Session, Self::Error> {
        let request = api::authenticate_custom(
            &self.server_key,
            &self.server_password,
            ApiAccountCustom {
                id: id.to_owned(),
                vars: string_map_to_owned_string_map(vars),
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
    /// Authenticate user with a device identifier usually obtained from a platform API.
    ///
    /// See [Authentication](index.html#authentication) for a description of the `username`, `create` and `vars` parameters.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # use std::collections::HashMap;
    /// # run_in_example(async move |client, session| {
    /// let session = client.authenticate_device("sufficientlylongdeviceid", Some("Username"), true, HashMap::new()).await
    ///     .expect("Failed to authenticate user");
    /// # Ok(())
    /// # });
    /// ```
    async fn authenticate_device(
        &self,
        id: &str,
        username: Option<&str>,
        create: bool,
        vars: HashMap<&str, &str>,
    ) -> Result<Session, Self::Error> {
        let request = api::authenticate_device(
            &self.server_key.clone(),
            &self.server_password,
            ApiAccountDevice {
                id: id.to_owned(),
                vars: string_map_to_owned_string_map(vars),
            },
            Some(create),
            username,
        );

        self.send(request)
            .await
            .map(DefaultClient::<A>::map_session)
    }

    /// Authenticate a user with an email and password.
    ///
    /// See [Authentication](index.html#authentication) for a description of the `username`, `create` and `vars` parameters.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # use std::collections::HashMap;
    /// # run_in_example(async move |client, session| {
    /// let session = client.authenticate_email("email@domain.com", "password", None, true, HashMap::new()).await
    ///     .expect("Failed to authenticate user");
    /// # Ok(())
    /// # });
    /// ```
    async fn authenticate_email(
        &self,
        email: &str,
        password: &str,
        username: Option<&str>,
        create: bool,
        vars: HashMap<&str, &str>,
    ) -> Result<Session, Self::Error> {
        let request = api::authenticate_email(
            &self.server_key,
            &self.server_password,
            ApiAccountEmail {
                email: email.to_owned(),
                password: password.to_owned(),
                vars: string_map_to_owned_string_map(vars),
            },
            Some(create),
            username,
        );

        self.send(request)
            .await
            .map(DefaultClient::<A>::map_session)
    }

    /// Authenticate a user with a Facebook auth token
    ///
    /// See [Authentication](index.html#authentication) for a description of the `username`, `create` and `vars` parameters.
    ///
    /// Set `import` to true to import Facebook friends to the user's social profile.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # use std::collections::HashMap;
    /// # run_in_example(async move |client, session| {
    /// let session = client.authenticate_facebook("facebooktoken", None, true, HashMap::new(), false).await
    ///     .expect("Failed to authenticate user");
    /// # Ok(())
    /// # });
    /// ```
    async fn authenticate_facebook(
        &self,
        token: &str,
        username: Option<&str>,
        create: bool,
        vars: HashMap<&str, &str>,
        import: bool,
    ) -> Result<Session, Self::Error> {
        let request = api::authenticate_facebook(
            &self.server_key,
            &self.server_password,
            ApiAccountFacebook {
                token: token.to_owned(),
                vars: string_map_to_owned_string_map(vars),
            },
            Some(create),
            username,
            Some(import),
        );

        self.send(request)
            .await
            .map(DefaultClient::<A>::map_session)
    }

    /// Authenticate a user with Apple Game Center
    ///
    /// See [Game center](https://heroiclabs.com/docs/authentication/#game-center) on how to set up authentication using the Apple Game Center.
    ///
    /// See [Authentication](index.html#authentication) for a description of the `username`, `create` and `vars` parameters.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # use std::collections::HashMap;
    /// # run_in_example(async move |client, session| {
    /// let session = client.authenticate_game_center("bundleid", "playerid", "public_key_url", "salt", "signature", "timestamp", None, true, HashMap::new()).await
    ///     .expect("Failed to authenticate user");
    /// # Ok(())
    /// # });
    /// ```
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
        vars: HashMap<&str, &str>,
    ) -> Result<Session, Self::Error> {
        let request = api::authenticate_game_center(
            &self.server_key,
            &self.server_password,
            ApiAccountGameCenter {
                bundle_id: bundle_id.to_owned(),
                player_id: player_id.to_owned(),
                public_key_url: public_key_url.to_owned(),
                salt: salt.to_owned(),
                signature: signature.to_owned(),
                timestamp_seconds: timestamp.to_owned(),
                vars: string_map_to_owned_string_map(vars),
            },
            Some(create),
            username,
        );

        self.send(request)
            .await
            .map(DefaultClient::<A>::map_session)
    }

    /// Authenticate a user with a Google auth token
    ///
    /// See [Authentication](index.html#authentication) for a description of the `username`, `create` and `vars` parameters.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # use std::collections::HashMap;
    /// # run_in_example(async move |client, session| {
    /// let session = client.authenticate_google("googletoken", None, true, HashMap::new()).await
    ///     .expect("Failed to authenticate user");
    /// # Ok(())
    /// # });
    /// ```
    async fn authenticate_google(
        &self,
        token: &str,
        username: Option<&str>,
        create: bool,
        vars: HashMap<&str, &str>,
    ) -> Result<Session, Self::Error> {
        let request = api::authenticate_google(
            &self.server_key,
            &self.server_password,
            ApiAccountGoogle {
                token: token.to_owned(),
                vars: string_map_to_owned_string_map(vars),
            },
            Some(create),
            username,
        );

        self.send(request)
            .await
            .map(DefaultClient::<A>::map_session)
    }

    /// Authenticate a user with a Steam auth token
    ///
    /// See [Authentication](index.html#authentication) for a description of the `username`, `create` and `vars` parameters.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # use std::collections::HashMap;
    /// # run_in_example(async move |client, session| {
    /// let session = client.authenticate_steam("steamtoken", None, true, HashMap::new()).await
    ///     .expect("Failed to authenticate user");
    /// # Ok(())
    /// # });
    /// ```
    async fn authenticate_steam(
        &self,
        token: &str,
        username: Option<&str>,
        create: bool,
        vars: HashMap<&str, &str>,
    ) -> Result<Session, Self::Error> {
        let request = api::authenticate_google(
            &self.server_key,
            &self.server_password,
            ApiAccountGoogle {
                token: token.to_owned(),
                vars: string_map_to_owned_string_map(vars),
            },
            Some(create),
            username,
        );

        self.send(request)
            .await
            .map(DefaultClient::<A>::map_session)
    }

    /// Ban a set of users from a group.
    ///
    /// See [`Self::list_group_users`] for an example on how to retrieve user ids.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # run_in_example(async move |client, session| {
    /// let group = client.create_group(&session, "NewGroup", None, None, None, None, None).await?;
    /// client.ban_group_users(&session, &group.id, &["userid1"]).await
    ///     .expect("Failed to ban group users");
    /// # Ok(())
    /// # })
    /// ```
    async fn ban_group_users(
        &self,
        session: &Session,
        group_id: &str,
        user_ids: &[&str],
    ) -> Result<(), Self::Error> {
        let user_ids = str_slice_to_owned(user_ids);
        let request = api::ban_group_users(&session.get_auth_token(), group_id, &user_ids);

        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// Block friends by id or username.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # run_in_example(async move |client, session| {
    /// client.block_friends(&session, &["userid1"], &["username2"]).await
    ///     .expect("Failed to block friends");
    /// # Ok(())
    /// # })
    /// ```
    async fn block_friends(
        &self,
        session: &Session,
        ids: &[&str],
        usernames: &[&str],
    ) -> Result<(), Self::Error> {
        let ids = str_slice_to_owned(ids);
        let usernames = str_slice_to_owned(usernames);
        let request = api::block_friends(&session.get_auth_token(), &ids, &usernames);

        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// Create a group.
    ///
    /// The user who creates the group becomes the owner and superadmin for it.
    ///
    /// The language tag `lang_tag` should be in BCP-47 format.
    ///
    /// If the group is open, any user can join.
    ///
    /// The default maximum member count is 100.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # run_in_example(async move |client, session| {
    /// client.create_group(&session, "GroupName", Some("Group description"), None, Some("en"), Some(true), Some(10)).await
    ///     .expect("Failed to create group");
    /// # Ok(())
    /// # })
    /// ```
    async fn create_group(
        &self,
        session: &Session,
        name: &str,
        description: Option<&str>,
        avatar_url: Option<&str>,
        lang_tag: Option<&str>,
        open: Option<bool>,
        max_count: Option<i32>,
    ) -> Result<ApiGroup, Self::Error> {
        let request = api::create_group(
            &session.get_auth_token(),
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

        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// Remove friends or friend requests.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # run_in_example(async move |client, session| {
    /// client.delete_friends(&session, &["userid1"], &["username1"]).await
    ///     .expect("Failed to remove friends");
    /// # Ok(())
    /// # })
    /// ```
    async fn delete_friends(
        &self,
        session: &Session,
        ids: &[&str],
        usernames: &[&str],
    ) -> Result<(), Self::Error> {
        let ids = str_slice_to_owned(ids);
        let usernames = str_slice_to_owned(usernames);
        let request = api::delete_friends(&session.get_auth_token(), &ids, &usernames);

        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// Delete a group.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # run_in_example(async move |client, session| {
    /// client.delete_group(&session, "groupid").await
    ///     .expect("Failed to delete group");
    /// # Ok(())
    /// # })
    /// ```
    async fn delete_group(&self, session: &Session, group_id: &str) -> Result<(), Self::Error> {
        let request = api::delete_group(&session.get_auth_token(), group_id);
        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// Delete a leaderboard record.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # run_in_example(async move |client, session| {
    /// client.delete_leaderboard_record(&session, "leaderboard_id").await
    ///     .expect("Failed to delete leaderboard record");
    /// # Ok(())
    /// # })
    /// ```
    async fn delete_leaderboard_record(
        &self,
        session: &Session,
        leaderboard_id: &str,
    ) -> Result<(), Self::Error> {
        let request = api::delete_leaderboard_record(&session.get_auth_token(), leaderboard_id);
        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// Delete notifications.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # run_in_example(async move |client, session| {
    /// client.delete_notifications(&session, &["notification_id1"]).await
    ///     .expect("Failed to delete notifications");
    /// # Ok(())
    /// # })
    /// ```
    async fn delete_notifications(
        &self,
        session: &Session,
        ids: &[&str],
    ) -> Result<(), Self::Error> {
        let ids = str_slice_to_owned(ids);
        let request = api::delete_notifications(&session.get_auth_token(), &ids);
        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// Delete storage objects.
    ///
    /// If the version in [`ApiDeleteStorageObjectId`] is specified, the deletion fails if the object was modified on the server by another client.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// use nakama_rs::api::ApiDeleteStorageObjectId;
    /// # run_in_example(async move |client, session| {
    /// // Delete 10 objects from the "collection1" collection.
    /// let mut objects = client.list_storage_objects(&session, "collection1", Some(10), None).await.expect("Failed to list object");
    /// let delete_objects: Vec<ApiDeleteStorageObjectId> = objects.objects.drain(..).map(|object|
    ///     ApiDeleteStorageObjectId {
    ///         collection: object.collection,
    ///         key: object.key,
    ///         // If we keep the version empty, `version: ""` the object would be deleted
    ///         // even if modified on the server by another client.
    ///         version: object.version,
    ///     }   
    /// ).collect();
    /// client.delete_storage_objects(&session, delete_objects.as_ref()).await
    ///     .expect("Failed to delete storage objects");
    /// # Ok(())
    /// # })
    /// ```
    async fn delete_storage_objects(
        &self,
        session: &Session,
        ids: &[ApiDeleteStorageObjectId],
    ) -> Result<(), Self::Error> {
        let request = api::delete_storage_objects(
            &session.get_auth_token(),
            ApiDeleteStorageObjectsRequest {
                object_ids: ids.to_vec(),
            },
        );
        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// Demote users in a group.
    ///
    /// The users role will change to the next role down. Members who are already at the lowest rank will be skipped.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # run_in_example(async move |client, session| {
    /// client.demote_group_users(&session, "group_id", &["userid1", "userid2"]).await
    ///     .expect("Failed to demote group users");
    /// # Ok(())
    /// # })
    /// ```
    async fn demote_group_users(
        &self,
        session: &Session,
        group_id: &str,
        user_ids: &[&str],
    ) -> Result<(), Self::Error> {
        let user_ids = str_slice_to_owned(user_ids);
        let request = api::demote_group_users(&session.get_auth_token(), group_id, &user_ids);
        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// Submit an event for processing in the server's registered runtime custom events handler.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # run_in_example(async move |client, session| {
    /// let properties = [("name1", "value1"), ("name2", "value2")].iter().cloned().collect();
    /// client.event(&session, "eventname", properties).await
    ///     .expect("Failed to submit event");
    /// # Ok(())
    /// # })
    /// ```
    async fn event(
        &self,
        session: &Session,
        name: &str,
        properties: HashMap<&str, &str>,
    ) -> Result<(), Self::Error> {
        let request = api::event(
            &session.get_auth_token(),
            ApiEvent {
                name: name.to_owned(),
                timestamp: "".to_owned(),
                external: true,
                properties: string_map_to_owned_string_map(properties),
            },
        );
        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// Fetch the users account
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # run_in_example(async move |client, session| {
    /// let account = client.get_account(&session).await
    ///     .expect("Failed to submit event");
    /// println!("{}", account.user.username);
    /// # Ok(())
    /// # })
    /// ```
    async fn get_account(&self, session: &Session) -> Result<ApiAccount, Self::Error> {
        let request = api::get_account(&session.get_auth_token());
        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// Fetch users by id, username, or facebook ids
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # run_in_example(async move |client, session| {
    /// let accounts = client.get_users(&session, &["userid1"], &["username2"], &["facebook_id3"]).await
    ///     .expect("Failed to get users");
    /// accounts.users.iter().for_each(|account| {
    ///     println!("User {}", account.username);
    /// });
    /// # Ok(())
    /// # })
    /// ```
    async fn get_users(
        &self,
        session: &Session,
        ids: &[&str],
        usernames: &[&str],
        facebook_ids: &[&str],
    ) -> Result<ApiUsers, Self::Error> {
        let ids = str_slice_to_owned(ids);
        let usernames = str_slice_to_owned(usernames);
        let facebook_ids = str_slice_to_owned(facebook_ids);
        let request = api::get_users(&session.get_auth_token(), &ids, &usernames, &facebook_ids);
        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// Import Facebook friends and add them as friends.
    ///
    /// The server will import friends when the user authenticates with Facebook.
    /// This function can be used to be explicit with the import operation.
    ///
    /// The optional `reset` parameter will reset the Facebook friend import.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # run_in_example(async move |client, session| {
    /// client.import_facebook_friends(&session, "facebook_token", Some(true)).await
    ///     .expect("Failed to import Facebook friends");
    /// # Ok(())
    /// # })
    /// ```
    async fn import_facebook_friends(
        &self,
        session: &Session,
        token: &str,
        reset: Option<bool>,
    ) -> Result<(), Self::Error> {
        let request = api::import_facebook_friends(
            &session.get_auth_token(),
            ApiAccountFacebook {
                vars: HashMap::new(),
                token: token.to_owned(),
            },
            reset,
        );
        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// Import Steam friends and add them as friends.
    ///
    /// The server will import friends when the user authenticates with Steam.
    /// This function can be used to be explicit with the import operation.
    ///
    /// The optional `reset` parameter will reset the Facebook friend import.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # run_in_example(async move |client, session| {
    /// client.import_steam_friends(&session, "steam_token", Some(true)).await
    ///     .expect("Failed to import Steam friends");
    /// # Ok(())
    /// # })
    /// ```
    async fn import_steam_friends(
        &self,
        session: &Session,
        token: &str,
        reset: Option<bool>,
    ) -> Result<(), Self::Error> {
        let request = api::import_steam_friends(
            &session.get_auth_token(),
            ApiAccountSteam {
                vars: HashMap::new(),
                token: token.to_owned(),
            },
            reset,
        );
        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// Join a group.
    ///
    /// If the group has open membership, join the group. Otherwise a join request will be sent.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # run_in_example(async move |client, session| {
    /// client.join_group(&session, "group_id").await
    ///     .expect("Failed to join group");
    /// # Ok(())
    /// # })
    /// ```
    async fn join_group(&self, session: &Session, group_id: &str) -> Result<(), Self::Error> {
        let request = api::join_group(&session.get_auth_token(), group_id);
        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// Join a tournament.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # run_in_example(async move |client, session| {
    /// client.join_tournament(&session, "tournament_id").await
    ///     .expect("Failed to join tournament");
    /// # Ok(())
    /// # })
    /// ```
    async fn join_tournament(
        &self,
        session: &Session,
        tournament_id: &str,
    ) -> Result<(), Self::Error> {
        let request = api::join_tournament(&session.get_auth_token(), tournament_id);
        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// Kick group users.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # run_in_example(async move |client, session| {
    /// client.kick_group_users(&session, "group_id", &["userid1", "userid2"]).await
    ///     .expect("Failed to kick group users");
    /// # Ok(())
    /// # })
    /// ```
    async fn kick_group_users(
        &self,
        session: &Session,
        group_id: &str,
        ids: &[&str],
    ) -> Result<(), Self::Error> {
        let ids = str_slice_to_owned(ids);
        let request = api::kick_group_users(&session.get_auth_token(), group_id, &ids);
        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// Leave a group.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # run_in_example(async move |client, session| {
    /// client.leave_group(&session, "group_id").await
    ///     .expect("Failed to leave group");
    /// # Ok(())
    /// # })
    /// ```
    async fn leave_group(&self, session: &Session, group_id: &str) -> Result<(), Self::Error> {
        let request = api::leave_group(&session.get_auth_token(), group_id);
        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// Link an Apple ID to the social profiles on the current user's account.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # use std::collections::HashMap;
    /// # run_in_example(async move |client, session| {
    /// let session = client.link_apple(&session, "appletoken").await
    ///     .expect("Failed to link account");
    /// # Ok(())
    /// # });
    /// ```
    async fn link_apple(&self, session: &Session, token: &str) -> Result<(), Self::Error> {
        let request = api::link_apple(
            &session.get_auth_token(),
            ApiAccountApple {
                vars: HashMap::new(),
                token: token.to_owned(),
            },
        );
        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// Link an custom ID to the social profiles on the current user's account.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # use std::collections::HashMap;
    /// # run_in_example(async move |client, session| {
    /// let session = client.link_custom(&session, "customtoken").await
    ///     .expect("Failed to link account");
    /// # Ok(())
    /// # });
    /// ```
    async fn link_custom(&self, session: &Session, id: &str) -> Result<(), Self::Error> {
        let request = api::link_custom(
            &session.get_auth_token(),
            ApiAccountCustom {
                vars: HashMap::new(),
                id: id.to_owned(),
            },
        );
        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// Link an device ID to the social profiles on the current user's account.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # use std::collections::HashMap;
    /// # run_in_example(async move |client, session| {
    /// let session = client.link_device(&session, "usersdeviceid").await
    ///     .expect("Failed to link account");
    /// # Ok(())
    /// # });
    /// ```
    async fn link_device(&self, session: &Session, id: &str) -> Result<(), Self::Error> {
        let request = api::link_device(
            &session.get_auth_token(),
            ApiAccountDevice {
                vars: HashMap::new(),
                id: id.to_owned(),
            },
        );
        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// Link an email and password to the social profiles on the current user's account.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # use std::collections::HashMap;
    /// # run_in_example(async move |client, session| {
    /// let session = client.link_email(&session, "email@domain.com", "password").await
    ///     .expect("Failed to link account");
    /// # Ok(())
    /// # });
    /// ```
    async fn link_email(
        &self,
        session: &Session,
        email: &str,
        password: &str,
    ) -> Result<(), Self::Error> {
        let request = api::link_email(
            &session.get_auth_token(),
            ApiAccountEmail {
                vars: HashMap::new(),
                email: email.to_owned(),
                password: password.to_owned(),
            },
        );
        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// Link a Facebook profile to the social profiles on the current user's account.
    ///
    /// If `import` is set to true, import Facebook friends to the user's social graph.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # use std::collections::HashMap;
    /// # run_in_example(async move |client, session| {
    /// let session = client.link_facebook(&session, "facebooktoken", None).await
    ///     .expect("Failed to link account");
    /// # Ok(())
    /// # });
    /// ```
    async fn link_facebook(
        &self,
        session: &Session,
        token: &str,
        import: Option<bool>,
    ) -> Result<(), Self::Error> {
        let request = api::link_facebook(
            &session.get_auth_token(),
            ApiAccountFacebook {
                vars: HashMap::new(),
                token: token.to_owned(),
            },
            import,
        );
        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// Link a Game Center profile to the social profiles on the current user's account.
    ///
    /// See [Game center](https://heroiclabs.com/docs/authentication/#game-center) on how to set up authentication using the Apple Game Center.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # use std::collections::HashMap;
    /// # run_in_example(async move |client, session| {
    /// let session = client.link_game_center(&session, "bundleid", "playerid", "public_key_url", "salt", "signature", "timestamp").await
    ///     .expect("Failed to link account");
    /// # Ok(())
    /// # });
    /// ```
    async fn link_game_center(
        &self,
        session: &Session,
        bundle_id: &str,
        player_id: &str,
        public_key_url: &str,
        salt: &str,
        signature: &str,
        timestamp: &str,
    ) -> Result<(), Self::Error> {
        let request = api::link_game_center(
            &session.get_auth_token(),
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
        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// Link a Google profile to the social profiles on the current user's account.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # use std::collections::HashMap;
    /// # run_in_example(async move |client, session| {
    /// let session = client.link_google(&session, "googletoken").await
    ///     .expect("Failed to link account");
    /// # Ok(())
    /// # });
    /// ```
    async fn link_google(&self, session: &Session, token: &str) -> Result<(), Self::Error> {
        let request = api::link_google(
            &session.get_auth_token(),
            ApiAccountGoogle {
                vars: HashMap::new(),
                token: token.to_owned(),
            },
        );
        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// Link a Steam profile to the social profiles on the current user's account.
    ///
    /// If `import` is set to true, import Steam friends to the user's social graph.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # use std::collections::HashMap;
    /// # run_in_example(async move |client, session| {
    /// let session = client.link_steam(&session, "steamtoken", false).await
    ///     .expect("Failed to link account");
    /// # Ok(())
    /// # });
    /// ```
    async fn link_steam(
        &self,
        session: &Session,
        token: &str,
        import: bool,
    ) -> Result<(), Self::Error> {
        let request = api::link_steam(
            &session.get_auth_token(),
            ApiLinkSteamRequest {
                account: ApiAccountSteam {
                    vars: HashMap::new(),
                    token: token.to_owned(),
                },
                sync: import,
            },
        );
        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// List messages from a chat channel.
    ///
    /// The chat channel id can be retrieved by using [`Socket::join_chat`].
    ///
    /// Specify `forward` to set the direction of the pagination.
    ///
    /// See [Limit and cursor](index.html#limit-and-cursor) for a description on how to use the `limit` and `cursor` parameters.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # use std::collections::HashMap;
    /// # run_in_example(async move |client, session| {
    /// let messages = client.list_channel_messages(&session, "channel_id", None, None, None).await
    ///     .expect("Failed to list channel messages");
    /// messages.messages.iter().for_each(|message| {
    ///     println!("{}: {}", message.username, message.content)
    /// });
    /// # Ok(())
    /// # });
    /// ```
    async fn list_channel_messages(
        &self,
        session: &Session,
        channel_id: &str,
        limit: Option<i32>,
        forward: Option<bool>,
        cursor: Option<&str>,
    ) -> Result<ApiChannelMessageList, Self::Error> {
        let request = api::list_channel_messages(
            &session.get_auth_token(),
            channel_id,
            limit,
            forward,
            cursor,
        );

        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// List friends
    ///
    /// It is possible to filter friends based on their state. See [Friend state](https://heroiclabs.com/docs/social-friends/#friend-state) for possible states.
    ///
    /// See [Limit and cursor](index.html#limit-and-cursor) for a description on how to use the `limit` and `cursor` parameters.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # use std::collections::HashMap;
    /// # run_in_example(async move |client, session| {
    /// let friends = client.list_friends(&session, None, None, None).await
    ///     .expect("Failed to list channel messages");
    /// # Ok(())
    /// # });
    /// ```
    async fn list_friends(
        &self,
        session: &Session,
        state: Option<i32>,
        limit: Option<i32>,
        cursor: Option<&str>,
    ) -> Result<ApiFriendList, Self::Error> {
        let request = api::list_friends(&session.get_auth_token(), limit, state, cursor);

        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// List all users part of the group.
    ///
    /// It is possible to filter users based on their state. See [Groups and Clans](https://heroiclabs.com/docs/social-groups-clans/#groups-and-clans)
    /// for possible values.
    ///
    /// See [Limit and cursor](index.html#limit-and-cursor) for a description on how to use the `limit` and `cursor` parameters.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # run_in_example(async move |client, session| {
    /// let group = client.create_group(&session, "NewGroup", None, None, None, None, None).await?;
    /// client.list_group_users(&session, &group.id, None, None, None).await
    ///     .expect("Failed to list group users");
    /// # Ok(())
    /// # })
    /// ```    
    async fn list_group_users(
        &self,
        session: &Session,
        group_id: &str,
        state: Option<i32>,
        limit: Option<i32>,
        cursor: Option<&str>,
    ) -> Result<ApiGroupUserList, Self::Error> {
        let request =
            api::list_group_users(&session.get_auth_token(), group_id, limit, state, cursor);

        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// List groups on the server.
    ///
    /// It is possible to filter groups by name. The percentage char '%' can be used as placeholder
    ///
    /// See [Limit and cursor](index.html#limit-and-cursor) for a description on how to use the `limit` and `cursor` parameters.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # run_in_example(async move |client, session| {
    /// // List all groups
    /// client.list_groups(&session, None, None, None).await
    ///     .expect("Failed to list groups");
    /// // Search for groups starting with "Instance1"
    /// client.list_groups(&session, Some("Instance1%"), None, None).await
    ///     .expect("Failed to list groups");
    /// # Ok(())
    /// # })
    /// ```    
    async fn list_groups(
        &self,
        session: &Session,
        name: Option<&str>,
        limit: Option<i32>,
        cursor: Option<&str>,
    ) -> Result<ApiGroupList, Self::Error> {
        let request = api::list_groups(&session.get_auth_token(), name, cursor, limit);

        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// List records from a leaderboard
    ///
    /// If you specify `owner_ids` only records of those users will be returned.
    ///
    /// Leaderboards can be scheduled to recur. So if you want to grab the records for specific instances of that leaderboard
    /// in the past, then you can specify the `expiry`.
    ///
    /// See [Limit and cursor](index.html#limit-and-cursor) for a description on how to use the `limit` and `cursor` parameters.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # run_in_example(async move |client, session| {
    /// client.list_leaderboard_records(&session, "leaderboard_id", &[], None, None, None).await
    ///     .expect("Failed to list leaderboard records");
    /// # Ok(())
    /// # })
    /// ```    
    async fn list_leaderboard_records(
        &self,
        session: &Session,
        leaderboard_id: &str,
        owner_ids: &[&str],
        expiry: Option<&str>,
        limit: Option<i32>,
        cursor: Option<&str>,
    ) -> Result<ApiLeaderboardRecordList, Self::Error> {
        let owner_ids = str_slice_to_owned(owner_ids);
        let request = api::list_leaderboard_records(
            &session.get_auth_token(),
            leaderboard_id,
            &owner_ids,
            limit,
            cursor,
            expiry,
        );

        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// List leaderboard records around owner
    ///
    /// If you specify `owner_ids` only records of those users will be returned.
    ///
    /// Leaderboards can be scheduled to recur. So if you want to grab the records for specific instances of that leaderboard
    /// in the past, then you can specify the `expiry`.
    ///
    /// See [Limit and cursor](index.html#limit-and-cursor) for a description on how to use the `limit`.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # run_in_example(async move |client, session| {
    /// let owner_id = client.get_account(&session).await.expect("Failed to get account").user.id;
    /// client.list_leaderboard_records_around_owner(&session, "leaderboard_id", &owner_id, None, None).await
    ///     .expect("Failed to list leaderboard records around owner");
    /// # Ok(())
    /// # })
    /// ```    
    async fn list_leaderboard_records_around_owner(
        &self,
        session: &Session,
        leaderboard_id: &str,
        owner_id: &str,
        expiry: Option<&str>,
        limit: Option<i32>,
    ) -> Result<ApiLeaderboardRecordList, Self::Error> {
        let request = api::list_leaderboard_records_around_owner(
            &session.get_auth_token(),
            leaderboard_id,
            owner_id,
            limit,
            expiry,
        );

        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// Fetch matches active on the server
    ///
    /// You can specify the minimum and maximum number of match participants.
    ///
    /// See [Limit and cursor](index.html#limit-and-cursor) for a description on how to use the `limit` and `cursor`.
    ///
    /// Use the label or a query to filter the match list.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # run_in_example(async move |client, session| {
    /// client.list_matches(&session, Some(2), Some(4), None, None, "", "").await
    ///     .expect("Failed to list matches");
    /// # Ok(())
    /// # })
    /// ```    
    async fn list_matches(
        &self,
        session: &Session,
        min: Option<i32>,
        max: Option<i32>,
        limit: Option<i32>,
        authoritative: Option<bool>,
        label: &str,
        query: &str,
    ) -> Result<ApiMatchList, Self::Error> {
        let request = api::list_matches(
            &session.get_auth_token(),
            limit,
            authoritative,
            Some(label),
            min,
            max,
            Some(query),
        );

        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// List notifications for the user.
    ///
    /// List notifications which were received when the user was offline. These notifications are ones which were marked "persistent"
    /// when sent.
    ///
    /// See [Limit and cursor](index.html#limit-and-cursor) for a description on how to use the `limit`.
    ///
    /// It is recommended to persist the cacheable cursor so that only notifications since the last call to
    /// `list_notifications` are returned.
    ///
    /// For more information see [List notifications](https://heroiclabs.com/docs/social-in-app-notifications/#list-notifications)
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # fn store(str: String) {};
    /// # fn restore() -> String { "".to_owned() };
    /// # run_in_example(async move |client, session| {
    /// // The first call
    /// let result = client.list_notifications(&session, None, None).await
    ///     .expect("Failed to list notifications");
    /// store(result.cacheable_cursor);
    /// // ... user closes the game ...
    /// // ... user restarts the game ...
    /// let cacheable_cursor = restore();
    /// // Only fetch notifications since the user closed the game.
    /// let result = client.list_notifications(&session, None, Some(&cacheable_cursor)).await
    ///     .expect("Failed to list notifications");
    ///
    /// # Ok(())
    /// # })
    /// ```    
    async fn list_notifications(
        &self,
        session: &Session,
        limit: Option<i32>,
        cacheable_cursor: Option<&str>,
    ) -> Result<ApiNotificationList, Self::Error> {
        let request = api::list_notifications(&session.get_auth_token(), limit, cacheable_cursor);

        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// List storage objects in a collection which have public read access.
    ///
    /// See [Limit and cursor](index.html#limit-and-cursor) for a description on how to use the `limit` and `cursor`.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # run_in_example(async move |client, session| {
    /// let result = client.list_storage_objects(&session, "collection", None, None).await
    ///     .expect("Failed to list storage objects");
    /// // Print all objects
    /// result.objects.iter().for_each(|object| {
    ///     println!("Object[{}]: {}", object.key, object.value);
    /// });
    /// # Ok(())
    /// # })
    /// ```    
    async fn list_storage_objects(
        &self,
        session: &Session,
        collection: &str,
        limit: Option<i32>,
        cursor: Option<&str>,
    ) -> Result<ApiStorageObjectList, Self::Error> {
        let request =
            api::list_storage_objects(&session.get_auth_token(), collection, None, limit, cursor);

        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// List tournament records around owner
    ///
    /// If you specify `owner_ids` only records of those users will be returned.
    ///
    /// Leaderboards can be scheduled to recur. So if you want to grab the records for specific instances of that leaderboard
    /// in the past, then you can specify the `expiry`.
    ///
    /// See [Limit and cursor](index.html#limit-and-cursor) for a description on how to use the `limit`.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # run_in_example(async move |client, session| {
    /// let result = client.list_tournament_records_around_owner(&session, "tournament", "", None, None).await
    ///     .expect("Failed to list tournament records around owner");
    /// // Print all records
    /// result.records.iter().for_each(|record| {
    ///     println!("Record[{}]: {}", record.username, record.score);
    /// });
    /// # Ok(())
    /// # })
    /// ```    
    async fn list_tournament_records_around_owner(
        &self,
        session: &Session,
        tournament_id: &str,
        owner_id: &str,
        expiry: Option<&str>,
        limit: Option<i32>,
    ) -> Result<ApiTournamentRecordList, Self::Error> {
        let request = api::list_tournament_records_around_owner(
            &session.get_auth_token(),
            tournament_id,
            owner_id,
            limit,
            expiry,
        );

        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// List tournament records
    ///
    /// If you specify `owner_ids` only records of those users will be returned.
    ///
    /// Leaderboards can be scheduled to recur. So if you want to grab the records for specific instances of that leaderboard
    /// in the past, then you can specify the `expiry`.
    ///
    /// See [Limit and cursor](index.html#limit-and-cursor) for a description on how to use the `limit` and `cursor`.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # run_in_example(async move |client, session| {
    /// let result = client.list_tournament_records(&session, "tournament", &[], None, None, None).await
    ///     .expect("Failed to list tournament records");
    /// // Print all records
    /// result.records.iter().for_each(|record| {
    ///     println!("Record[{}]: {}", record.username, record.score);
    /// });
    /// # Ok(())
    /// # })
    /// ```    
    async fn list_tournament_records(
        &self,
        session: &Session,
        tournament_id: &str,
        owner_ids: &[&str],
        expiry: Option<&str>,
        limit: Option<i32>,
        cursor: Option<&str>,
    ) -> Result<ApiTournamentRecordList, Self::Error> {
        let owner_ids = str_slice_to_owned(owner_ids);
        let request = api::list_tournament_records(
            &session.get_auth_token(),
            tournament_id,
            &owner_ids,
            limit,
            cursor,
            expiry,
        );

        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// List current or upcoming tournaments
    ///
    /// Use `category_start` and `category_end` to filter based on the category that is set on the server on tournament creation. The category value is between 0 and 127.
    ///
    /// Omitting the start and end time parameters returns the ongoing and future tournaments.
    /// Setting the end time parameter to 0 only includes tournaments with no end time set in the results.
    /// Setting end time to a > 0 unix timestamp acts as an upper bound and only returns tournaments that end prior to it (excluding tournaments with no end time).
    /// Setting the start time to a > 0 unix timestamp returns any tournaments that start at a later time than it.
    ///
    /// See [Limit and cursor](index.html#limit-and-cursor) for a description on how to use the `limit` and `cursor`.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # run_in_example(async move |client, session| {
    /// let result = client.list_tournaments(&session, None, None, None, None, None, None).await
    ///     .expect("Failed to list tournaments");
    /// // Print all tournaments
    /// result.tournaments.iter().for_each(|tournament| {
    ///     println!("Tournament: {}", tournament.title);
    /// });
    /// # Ok(())
    /// # })
    /// ```    
    async fn list_tournaments(
        &self,
        session: &Session,
        category_start: Option<i32>,
        category_end: Option<i32>,
        start_time: Option<i32>,
        end_time: Option<i32>,
        limit: Option<i32>,
        cursor: Option<&str>,
    ) -> Result<ApiTournamentList, Self::Error> {
        let request = api::list_tournaments(
            &session.get_auth_token(),
            category_start,
            category_end,
            start_time,
            end_time,
            limit,
            cursor,
        );

        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// List groups an user is a member of.
    ///
    /// You can filter by the group state. For possible values, see [Groups and Clans](https://heroiclabs.com/docs/social-groups-clans/#groups-and-clans)
    ///
    /// See [Limit and cursor](index.html#limit-and-cursor) for a description on how to use the `limit` and `cursor`.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # run_in_example(async move |client, session| {
    /// let result = client.list_user_groups(&session, "user_id", None, None, None).await
    ///     .expect("Failed to list user groups");
    /// // Print all groups
    /// result.user_groups.iter().for_each(|group| {
    ///     println!("Group: {}", group.group.name);
    /// });
    /// # Ok(())
    /// # })
    /// ```    
    async fn list_user_groups(
        &self,
        session: &Session,
        user_id: &str,
        state: Option<i32>,
        limit: Option<i32>,
        cursor: Option<&str>,
    ) -> Result<ApiUserGroupList, Self::Error> {
        let request =
            api::list_user_groups(&session.get_auth_token(), user_id, limit, state, cursor);

        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// List storage objects in a collection which belong to a specific user and have public read access.
    ///
    /// See [Limit and cursor](index.html#limit-and-cursor) for a description on how to use the `limit` and `cursor`.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # run_in_example(async move |client, session| {
    /// let result = client.list_users_storage_objects(&session, "collection", "user_id", None, None).await
    ///     .expect("Failed to list users storage objects");
    /// // Print all objects
    /// result.objects.iter().for_each(|object| {
    ///     println!("Object[{}]: {}", object.key, object.value);
    /// });
    /// # Ok(())
    /// # })
    /// ```    
    async fn list_users_storage_objects(
        &self,
        session: &Session,
        collection: &str,
        user_id: &str,
        limit: Option<i32>,
        cursor: Option<&str>,
    ) -> Result<ApiStorageObjectList, Self::Error> {
        let request = api::list_storage_objects(
            &session.get_auth_token(),
            collection,
            Some(user_id),
            limit,
            cursor,
        );

        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// Promote group users.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # run_in_example(async move |client, session| {
    /// client.promote_group_user(&session, "group_id", &["userid1", "userid2"]).await
    ///     .expect("Failed to pomote group users");
    /// # Ok(())
    /// # })
    /// ```    
    async fn promote_group_user(
        &self,
        session: &Session,
        group_id: &str,
        ids: &[&str],
    ) -> Result<(), Self::Error> {
        let ids = str_slice_to_owned(ids);
        let request = api::promote_group_users(&session.get_auth_token(), group_id, &ids);

        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// Read objects from the storage engine.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # use nakama_rs::api::ApiReadStorageObjectId;
    /// # run_in_example(async move |client, session| {
    /// let result = client.read_storage_objects(&session, &[
    ///     ApiReadStorageObjectId {
    ///         collection: "collection1".to_owned(),
    ///         key: "key1".to_owned(),
    ///         user_id: "userid1".to_owned(),
    ///     }
    /// ]).await
    ///     .expect("Failed to read storage objects");
    /// result.objects.iter().for_each(|object| {
    ///     println!("{} - {}: {}", object.collection, object.key, object.value);
    /// });
    /// # Ok(())
    /// # })
    /// ```    
    async fn read_storage_objects(
        &self,
        session: &Session,
        ids: &[ApiReadStorageObjectId],
    ) -> Result<ApiStorageObjects, Self::Error> {
        let ids = ids.to_vec();
        let request = api::read_storage_objects(
            &session.get_auth_token(),
            ApiReadStorageObjectsRequest { object_ids: ids },
        );

        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// Execute a function on the server
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # run_in_example(async move |client, session| {
    /// let result = client.rpc(&session, "rpc_func_name", Some("Hello World!")).await
    ///     .expect("Failed to execute rpc function");
    /// println!("Returned: {}", result.payload);
    /// # Ok(())
    /// # })
    /// ```    
    async fn rpc(
        &self,
        session: &Session,
        id: &str,
        payload: Option<&str>,
    ) -> Result<ApiRpc, Self::Error> {
        let request = api::rpc_func2(&session.get_auth_token(), id, payload, None);

        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// Log out a session which optionally invalidates the authorization and/or refresh token.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # run_in_example(async move |client, session| {
    /// client.session_logout(&session).await
    ///     .expect("Failed to log out session");
    /// # Ok(())
    /// # })
    /// ```    
    async fn session_logout(&self, session: &Session) -> Result<(), Self::Error> {
        let request = api::session_logout(
            &session.get_auth_token(),
            ApiSessionLogoutRequest {
                token: session.get_auth_token(),
                refresh_token: session.get_refresh_token().unwrap_or("".to_owned()),
            },
        );

        self.send(request).await
    }

    /// Refresh the session.
    ///
    /// Refresh the session unless the current refresh token has expired. If `vars` are specified they will
    /// replace what is curently stored inside the session token.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # run_in_example(async move |client, session| {
    /// let new_vars = [("key", "value")].iter().cloned().collect();
    /// client.session_refresh(&session, new_vars).await
    ///     .expect("Failed to refresh session");
    /// # Ok(())
    /// # })
    /// ```    
    async fn session_refresh(
        &self,
        session: &Session,
        vars: HashMap<&str, &str>,
    ) -> Result<(), Self::Error> {
        let refresh_token = session
            .get_refresh_token()
            .expect("Session refresh can only be called when a refresh token is available");
        let request = api::session_refresh(
            &self.server_key,
            &self.server_password,
            ApiSessionRefreshRequest {
                token: refresh_token,
                vars: string_map_to_owned_string_map(vars),
            },
        );

        let data = self.send(request).await?;

        session.replace(&data.token, &data.refresh_token);

        Ok(())
    }

    /// Unlink an Apple ID from the users account.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # run_in_example(async move |client, session| {
    /// client.unlink_apple(&session, "appletoken").await
    ///     .expect("Failed to unlink account");
    /// # Ok(())
    /// # })
    /// ```   
    async fn unlink_apple(&self, session: &Session, token: &str) -> Result<(), Self::Error> {
        let request = api::unlink_apple(
            &session.get_auth_token(),
            ApiAccountApple {
                vars: HashMap::new(),
                token: token.to_owned(),
            },
        );

        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// Unlink a custom ID from the users account.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # run_in_example(async move |client, session| {
    /// client.unlink_custom(&session, "customid").await
    ///     .expect("Failed to unlink account");
    /// # Ok(())
    /// # })
    /// ```   
    async fn unlink_custom(&self, session: &Session, id: &str) -> Result<(), Self::Error> {
        let request = api::unlink_custom(
            &session.get_auth_token(),
            ApiAccountCustom {
                vars: HashMap::new(),
                id: id.to_owned(),
            },
        );

        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// Unlink a device ID from the users account.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # run_in_example(async move |client, session| {
    /// client.unlink_device(&session, "usersdeviceid").await
    ///     .expect("Failed to unlink account");
    /// # Ok(())
    /// # })
    /// ```   
    async fn unlink_device(&self, session: &Session, id: &str) -> Result<(), Self::Error> {
        let request = api::unlink_device(
            &session.get_auth_token(),
            ApiAccountDevice {
                vars: HashMap::new(),
                id: id.to_owned(),
            },
        );

        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// Unlink an email with password from the users account.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # run_in_example(async move |client, session| {
    /// client.unlink_email(&session, "email@domain.com", "password").await
    ///     .expect("Failed to unlink account");
    /// # Ok(())
    /// # })
    /// ```   
    async fn unlink_email(
        &self,
        session: &Session,
        email: &str,
        password: &str,
    ) -> Result<(), Self::Error> {
        let request = api::unlink_email(
            &session.get_auth_token(),
            ApiAccountEmail {
                vars: HashMap::new(),
                email: email.to_owned(),
                password: password.to_owned(),
            },
        );

        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// Unlink a Facebook profile from the users account.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # run_in_example(async move |client, session| {
    /// client.unlink_facebook(&session, "facebooktoken").await
    ///     .expect("Failed to unlink account");
    /// # Ok(())
    /// # })
    /// ```   
    async fn unlink_facebook(&self, session: &Session, token: &str) -> Result<(), Self::Error> {
        let request = api::unlink_facebook(
            &session.get_auth_token(),
            ApiAccountFacebook {
                vars: HashMap::new(),
                token: token.to_owned(),
            },
        );

        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// Unlink a Game Center profile from the users account.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # run_in_example(async move |client, session| {
    /// client.unlink_game_center(&session, "bundleid", "playerid" ,"public_key_url", "salt", "signature", "timestamp").await
    ///     .expect("Failed to unlink account");
    /// # Ok(())
    /// # })
    /// ```   
    async fn unlink_game_center(
        &self,
        session: &Session,
        bundle_id: &str,
        player_id: &str,
        public_key_url: &str,
        salt: &str,
        signature: &str,
        timestamp: &str,
    ) -> Result<(), Self::Error> {
        let request = api::unlink_game_center(
            &session.get_auth_token(),
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

        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// Unlink a Google profile from the users account.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # run_in_example(async move |client, session| {
    /// client.unlink_google(&session, "googletoken").await
    ///     .expect("Failed to unlink account");
    /// # Ok(())
    /// # })
    /// ```   
    async fn unlink_google(&self, session: &Session, token: &str) -> Result<(), Self::Error> {
        let request = api::unlink_google(
            &session.get_auth_token(),
            ApiAccountGoogle {
                vars: HashMap::new(),
                token: token.to_owned(),
            },
        );

        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// Unlink a Steam profile from the users account.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # run_in_example(async move |client, session| {
    /// client.unlink_steam(&session, "steamtoken").await
    ///     .expect("Failed to unlink account");
    /// # Ok(())
    /// # })
    /// ```   
    async fn unlink_steam(&self, session: &Session, token: &str) -> Result<(), Self::Error> {
        let request = api::unlink_steam(
            &session.get_auth_token(),
            ApiAccountSteam {
                vars: HashMap::new(),
                token: token.to_owned(),
            },
        );

        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// Update the user's account.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # run_in_example(async move |client, session| {
    /// client.update_account(&session, "NewUsername", None, None, None, None, None).await
    ///     .expect("Failed to update account");
    /// # Ok(())
    /// # })
    /// ```   
    async fn update_account(
        &self,
        session: &Session,
        username: &str,
        display_name: Option<&str>,
        avatar_url: Option<&str>,
        lang_tag: Option<&str>,
        location: Option<&str>,
        timezone: Option<&str>,
    ) -> Result<(), Self::Error> {
        let request = api::update_account(
            &session.get_auth_token(),
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

        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// Update a group.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # run_in_example(async move |client, session| {
    /// client.update_group(&session, "groupid", "GroupName", true, None, None, None).await
    ///     .expect("Failed to update group");
    /// # Ok(())
    /// # })
    /// ```   
    async fn update_group(
        &self,
        session: &Session,
        group_id: &str,
        name: &str,
        open: bool,
        description: Option<&str>,
        avatar_url: Option<&str>,
        lang_tag: Option<&str>,
    ) -> Result<(), Self::Error> {
        let request = api::update_group(
            &session.get_auth_token(),
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

        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// Validate a purchase receipt against the Apple App Store.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # run_in_example(async move |client, session| {
    /// client.validate_purchase_apple(&session, "receipt").await
    ///     .expect("Failed to validate purchase");
    /// # Ok(())
    /// # })
    /// ```   
    async fn validate_purchase_apple(
        &self,
        session: &Session,
        receipt: &str,
    ) -> Result<ApiValidatePurchaseResponse, Self::Error> {
        let request = api::validate_purchase_apple(
            &session.get_auth_token(),
            ApiValidatePurchaseAppleRequest {
                receipt: receipt.to_string(),
            },
        );

        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// Validate a purchase receipt against the Google Play Store.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # run_in_example(async move |client, session| {
    /// client.validate_purchase_google(&session, "receipt").await
    ///     .expect("Failed to validate purchase");
    /// # Ok(())
    /// # })
    /// ```   
    async fn validate_purchase_google(
        &self,
        session: &Session,
        receipt: &str,
    ) -> Result<ApiValidatePurchaseResponse, Self::Error> {
        let request = api::validate_purchase_google(
            &session.get_auth_token(),
            ApiValidatePurchaseGoogleRequest {
                purchase: receipt.to_string(),
            },
        );

        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// Validate a purchase receipt against the Huawei AppGallery.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # run_in_example(async move |client, session| {
    /// client.validate_purchase_huawei(&session, "receipt", "signature").await
    ///     .expect("Failed to validate purchase");
    /// # Ok(())
    /// # })
    /// ```   
    async fn validate_purchase_huawei(
        &self,
        session: &Session,
        receipt: &str,
        signature: &str,
    ) -> Result<ApiValidatePurchaseResponse, Self::Error> {
        let request = api::validate_purchase_huawei(
            &session.get_auth_token(),
            ApiValidatePurchaseHuaweiRequest {
                purchase: receipt.to_owned(),
                signature: signature.to_owned(),
            },
        );

        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// Write a leaderboard record.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # run_in_example(async move |client, session| {
    /// client.write_leaderboard_record(&session, "leaderboard_id", 100, None, None, None).await
    ///     .expect("Failed to write leaderboard record");
    /// # Ok(())
    /// # })
    /// ```   
    async fn write_leaderboard_record(
        &self,
        session: &Session,
        leaderboard_id: &str,
        score: i64,
        sub_score: Option<i64>,
        override_operator: Option<ApiOverrideOperator>,
        metadata: Option<&str>,
    ) -> Result<ApiLeaderboardRecord, Self::Error> {
        let operator = override_operator.unwrap_or(ApiOverrideOperator::NoOverride);
        let request = api::write_leaderboard_record(
            &session.get_auth_token(),
            leaderboard_id,
            WriteLeaderboardRecordRequestLeaderboardRecordWrite {
                metadata: metadata.unwrap_or("").to_owned(),
                score: score.to_string(),
                subscore: sub_score.map(|sub_score| sub_score.to_string()),
                operator,
            },
        );

        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// Write a leaderboard record.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::api::ApiOverrideOperator;
    /// use nakama_rs::test_helpers::*;
    /// # run_in_example(async move |client, session| {
    ///     client.create_leaderboard(&session, ApiOverrideOperator::SET).await
    ///     .expect("Failed to write leaderboard record");
    /// # Ok(())
    /// # })
    /// ```
    async fn create_leaderboard(
        &self,
        session: &Session,
        operator: ApiOverrideOperator,
        sort_order: SortOrder,
    ) -> Result<Leaderboard, Self::Error> {
        let request = api::create_leaderboard(
            &session.get_auth_token(),
            CreateLeaderboard {
                operator: operator.to_string(),
                sort_order: sort_order.to_string(),
            },
        );

        self.refresh_session(session).await?;
        let data = self.send(request).await?;
        let data: Leaderboard = serde_json::from_str(&data.payload).unwrap();
        Ok(data)
    }

    /// Write objects to the storage engine.
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # use nakama_rs::api::ApiWriteStorageObject;
    /// # use nanoserde::SerJson;
    /// #[derive(SerJson)]
    /// struct Card {
    ///     name: String,
    ///     cost: i32,
    /// }
    /// # run_in_example(async move |client, session| {
    /// let card = Card { cost: 10, name: "Ship".to_owned() };
    /// client.write_storage_objects(&session, &[ApiWriteStorageObject {
    ///     collection: "collection1".to_owned(),
    ///     key: "ship".to_owned(),
    ///     value: card.serialize_json(),
    ///     permission_write: 1,
    ///     permission_read: 1,
    ///     version: "*".to_owned(),
    /// }]).await
    ///     .expect("Failed to write object");
    /// # Ok(())
    /// # })
    /// ```   
    async fn write_storage_objects(
        &self,
        session: &Session,
        objects: &[ApiWriteStorageObject],
    ) -> Result<ApiStorageObjectAcks, Self::Error> {
        let request = api::write_storage_objects(
            &session.get_auth_token(),
            ApiWriteStorageObjectsRequest {
                objects: objects.to_vec(),
            },
        );

        self.refresh_session(session).await?;
        self.send(request).await
    }

    /// Write a tournament record
    ///
    /// # Example
    /// ```
    /// # #![feature(async_closure)]
    /// # use nakama_rs::test_helpers::*;
    /// # use nakama_rs::api::ApiWriteStorageObject;
    /// use nanoserde::SerJson;
    /// #[derive(SerJson)]
    /// struct Card {
    ///     name: String,
    ///     cost: i32,
    /// }
    /// # run_in_example(async move |client, session| {
    /// let card = Card { cost: 10, name: "Ship".to_owned() };
    /// client.write_storage_objects(&session, &[ApiWriteStorageObject {
    ///     collection: "collection1".to_owned(),
    ///     key: "ship".to_owned(),
    ///     value: card.serialize_json(),
    ///     permission_write: 1,
    ///     permission_read: 1,
    ///     version: "*".to_owned(),
    /// }]).await
    ///     .expect("Failed to write object");
    /// # Ok(())
    /// # })
    /// ```   
    async fn write_tournament_record(
        &self,
        session: &Session,
        tournament_id: &str,
        score: i64,
        sub_score: Option<i64>,
        override_operator: Option<ApiOverrideOperator>,
        metadata: Option<&str>,
    ) -> Result<ApiLeaderboardRecord, Self::Error> {
        let operator = override_operator.unwrap_or(ApiOverrideOperator::NoOverride);
        let request = api::write_tournament_record(
            &session.get_auth_token(),
            tournament_id,
            WriteTournamentRecordRequestTournamentRecordWrite {
                metadata: metadata.map(|str| str.to_owned()),
                score: score.to_string(),
                subscore: sub_score.map(|sub_score| sub_score.to_string()),
                operator,
            },
        );

        self.refresh_session(session).await?;
        self.send(request).await
    }
}
