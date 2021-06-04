use crate::api::RestRequest;
use async_trait::async_trait;
use nanoserde::DeJson;
use std::error::Error;

#[async_trait]
pub trait ClientAdapter {
    type Error: Error;
    async fn send<T: DeJson + Send>(&self, request: RestRequest<T>) -> Result<T, Self::Error>;
}
