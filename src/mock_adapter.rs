use crate::client_adapter::{ClientAdapter, ClientAdapterError};
use crate::api::RestRequest;
use nanoserde::DeJson;
use std::error::Error;
use std::fmt::{Display, Formatter};
use async_trait::async_trait;

#[derive(Debug)]
pub struct MockAdapterError {
}

impl Display for MockAdapterError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

impl Error for MockAdapterError {
}

impl ClientAdapterError for MockAdapterError {
    fn is_server_error(&self) -> bool {
        return true
    }

    fn is_client_error(&self) -> bool {
        return false
    }
}

pub struct MockAdapter {
}

#[async_trait]
impl ClientAdapter for MockAdapter {
    type Error = MockAdapterError;

    async fn send<T: DeJson + Send>(&self, _request: RestRequest<T>) -> Result<T, Self::Error> {
        return Err(MockAdapterError {})
    }
}