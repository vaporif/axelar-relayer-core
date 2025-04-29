use std::fmt::Debug;
use std::marker::PhantomData;

use redis::aio::MultiplexedConnection;
use redis::{AsyncCommands, Client};
use serde::{Deserialize, Serialize};

use super::error::Error;
use crate::interfaces::kv_store::WithRevision;
use crate::interfaces::{self};

pub struct GcpRedis<T> {
    key: String,
    connection: MultiplexedConnection,
    _phantom: PhantomData<T>,
}

impl<T> GcpRedis<T>
where
    T: Serialize + for<'de> Deserialize<'de> + Send + Sync,
{
    pub async fn connect(key: String, connection: String) -> Result<Self, Error> {
        let client = Client::open(connection).map_err(Error::Connection)?;

        let connection = client
            .get_multiplexed_async_connection()
            .await
            .map_err(Error::Connection)?;

        Ok(GcpRedis {
            key,
            connection,
            _phantom: PhantomData,
        })
    }

    pub async fn upsert(&self, value: &T) -> Result<(), Error> {
        let json_string = serde_json::to_string(value).map_err(Error::RedisSerialize)?;

        let _: () = self
            .connection
            .clone()
            .set(&self.key, json_string)
            .await
            .map_err(Error::RedisSave)?;

        Ok(())
    }
}

// Revision is not used here
// TODO: remove it from interfaces?
impl<T> interfaces::kv_store::KvStore<T> for GcpRedis<T>
where
    T: Serialize + for<'de> Deserialize<'de> + Send + Sync + Debug,
{
    #[allow(refining_impl_trait)]
    #[tracing::instrument(skip(self))]
    async fn update(&self, data: &WithRevision<T>) -> Result<u64, Error> {
        tracing::debug!(?data, "updating");
        self.upsert(&data.value).await?;
        Ok(0)
    }

    #[allow(refining_impl_trait)]
    #[tracing::instrument(skip(self))]
    async fn put(&self, value: &T) -> Result<u64, Error> {
        tracing::debug!(?value, "updating");
        self.upsert(value).await?;
        Ok(0)
    }

    #[allow(refining_impl_trait)]
    #[tracing::instrument(skip(self))]
    async fn get(&self) -> Result<Option<WithRevision<T>>, Error> {
        let mut connection = self.connection.clone();
        let value: Option<String> = connection.get(&self.key).await.map_err(Error::RedisGet)?;

        value
            .map(|entry| {
                let value: T = serde_json::from_str(&entry).map_err(Error::RedisDeserialize)?;

                Ok(interfaces::kv_store::WithRevision { value, revision: 0 })
            })
            .transpose()
    }
}
