use core::fmt::Debug;
use core::marker::PhantomData;

use redis::aio::MultiplexedConnection;
use redis::{AsyncCommands as _, Client};
use serde::{Deserialize, Serialize};

use super::GcpError;
use crate::interfaces;
use crate::interfaces::kv_store::WithRevision;

/// Redis client
pub struct RedisClient<T> {
    key: String,
    connection: MultiplexedConnection,
    _phantom: PhantomData<T>,
}

impl<T> RedisClient<T>
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    pub(crate) async fn connect(key: String, connection: String) -> Result<Self, GcpError> {
        let client = Client::open(connection).map_err(GcpError::Connection)?;

        let connection = client
            .get_multiplexed_async_connection()
            .await
            .map_err(GcpError::Connection)?;

        Ok(Self {
            key,
            connection,
            _phantom: PhantomData,
        })
    }

    pub(crate) async fn upsert(&self, value: &T) -> Result<(), GcpError> {
        let json_string = serde_json::to_string(value).map_err(GcpError::RedisSerialize)?;

        let _: () = self
            .connection
            .clone()
            .set(&self.key, json_string)
            .await
            .map_err(GcpError::RedisSave)?;

        Ok(())
    }
}

// Revision is not used here
// TODO: remove it from interfaces?
impl<T> interfaces::kv_store::KvStore<T> for RedisClient<T>
where
    T: Serialize + for<'de> Deserialize<'de> + Debug,
{
    #[allow(refining_impl_trait, reason = "simplification")]
    #[tracing::instrument(skip(self))]
    async fn update(&self, data: &WithRevision<T>) -> Result<u64, GcpError> {
        tracing::debug!(?data, "updating");
        self.upsert(&data.value).await?;
        Ok(0)
    }

    #[allow(refining_impl_trait, reason = "simplification")]
    #[tracing::instrument(skip(self))]
    async fn put(&self, value: &T) -> Result<u64, GcpError> {
        tracing::debug!(?value, "updating");
        self.upsert(value).await?;
        Ok(0)
    }

    #[allow(refining_impl_trait, reason = "simplification")]
    #[tracing::instrument(skip(self))]
    async fn get(&self) -> Result<Option<WithRevision<T>>, GcpError> {
        let mut connection = self.connection.clone();
        let value: Option<String> = connection
            .get(&self.key)
            .await
            .map_err(GcpError::RedisGet)?;

        value
            .map(|entry| {
                let value: T = serde_json::from_str(&entry).map_err(GcpError::RedisDeserialize)?;

                Ok(interfaces::kv_store::WithRevision { value, revision: 0 })
            })
            .transpose()
    }
}
