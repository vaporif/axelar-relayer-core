//! Bindings for the Amplifier API REST [paths](https://github.com/axelarnetwork/axelar-eds-mirror/blob/3dcef3bc08ecb51af79c6223605d4fbc01660847/oapi/gmp/schema.yaml#L6-L77)

use std::borrow::Cow;

use crate::error::AmplifierApiError;
use crate::types::{
    ErrorResponse, GetTasksResult, PublishEventsRequest, PublishEventsResult, TaskItemId,
};

/// Represents a singular REST request that can be done on the Amplifier API
pub trait AmplifierApiRequest<'a> {
    /// The successufl result type to be returned
    type Res: serde::de::DeserializeOwned + core::fmt::Debug;
    /// The error type to be returned on invalid data
    type Error: serde::de::DeserializeOwned;
    /// The payload that we will send as json during in the request body
    type Payload: serde::Serialize;

    /// The HTTP method to use
    const METHOD: reqwest::Method;

    /// The full qualified path to send the request to.
    /// The `base_url` points to the Amplifier API
    ///
    /// # Errors
    ///
    /// This function will return an error if any of the requests error, or the serialization does
    /// not work
    fn path(&self, base_url: &url::Url) -> Result<url::Url, AmplifierApiError>;
    /// The payload to send in the request body
    fn payload(&self) -> Self::Payload;
}

/// Translation of the `/health` [endpoint](https://github.com/axelarnetwork/axelar-eds-mirror/blob/3dcef3bc08ecb51af79c6223605d4fbc01660847/oapi/gmp/schema.yaml#L7-L13)
pub struct HealthCheck;
impl AmplifierApiRequest<'static> for HealthCheck {
    type Res = ();
    type Error = ();
    type Payload = ();

    const METHOD: reqwest::Method = reqwest::Method::GET;

    fn path(&self, base_url: &url::Url) -> Result<url::Url, AmplifierApiError> {
        base_url.join("health").map_err(AmplifierApiError::from)
    }

    fn payload(&self) -> Self::Payload {}
}

/// Translation of GET `/chains/{chain}/tasks` [endpoint](https://github.com/axelarnetwork/axelar-eds-mirror/blob/3dcef3bc08ecb51af79c6223605d4fbc01660847/oapi/gmp/schema.yaml#L7-L13)
#[derive(Debug, Clone, typed_builder::TypedBuilder)]
pub struct GetChains<'a> {
    /// The name of the cain that we want to query and get the tasks for
    pub chain: Cow<'a, str>,
    #[builder(setter(strip_option), default)]
    /// The earliers task id
    pub after: Option<TaskItemId>,
    #[builder(setter(strip_option), default)]
    /// The latest task id
    pub before: Option<TaskItemId>,
    /// the amount of results to return
    #[builder(setter(strip_option), default)]
    pub limit: Option<u8>,
}

impl<'a> AmplifierApiRequest<'a> for GetChains<'a> {
    type Res = GetTasksResult;
    type Error = ErrorResponse;
    type Payload = ();

    const METHOD: reqwest::Method = reqwest::Method::GET;

    fn path(&self, base_url: &url::Url) -> Result<url::Url, AmplifierApiError> {
        let mut url = base_url
            .join("chains")?
            .join(self.chain.as_ref())?
            .join("tasks")?;

        {
            let mut query_pairs = url.query_pairs_mut();
            if let Some(ref after) = self.after {
                query_pairs.append_pair("after", &after.0.as_hyphenated().to_string());
            }
            if let Some(ref before) = self.before {
                query_pairs.append_pair("before", &before.0.as_hyphenated().to_string());
            }
            if let Some(limit) = self.limit {
                query_pairs.append_pair("limit", &limit.to_string());
            }
        }

        Ok(url)
    }

    fn payload(&self) -> Self::Payload {}
}

/// Translation of POST `/chains/{chain}/tasks` [endpoint](https://github.com/axelarnetwork/axelar-eds-mirror/blob/3dcef3bc08ecb51af79c6223605d4fbc01660847/oapi/gmp/schema.yaml#L14-L50)
#[derive(Debug, Clone, typed_builder::TypedBuilder)]
pub struct PostEvents<'a, 'b> {
    /// The chain that we want to publish events for
    pub chain: Cow<'a, str>,
    /// The payload body to send
    pub payload: &'b PublishEventsRequest,
}

impl<'a, 'b> AmplifierApiRequest<'a> for PostEvents<'a, 'b> {
    type Res = PublishEventsResult;
    type Payload = &'b PublishEventsRequest;
    type Error = ErrorResponse;

    const METHOD: reqwest::Method = reqwest::Method::GET;

    fn path(&self, base_url: &url::Url) -> Result<url::Url, AmplifierApiError> {
        let url = base_url
            .join("chains")?
            .join(self.chain.as_ref())?
            .join("events")?;

        Ok(url)
    }

    fn payload(&self) -> Self::Payload {
        self.payload
    }
}
