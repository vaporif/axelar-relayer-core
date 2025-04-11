use core::marker::PhantomData;

use async_nats::jetstream::{self, kv};
use borsh::{BorshDeserialize, BorshSerialize};
use url::Url;

use super::error::Error;
use crate::interfaces;

pub struct NatsKvStore<T> {
    bucket: String,
    store: kv::Store,
    _phantom: PhantomData<T>,
}

pub async fn connect<T>(
    urls: Vec<Url>,
    bucket: String,
    description: String,
) -> Result<NatsKvStore<T>, Error> {
    let connect_options = async_nats::ConnectOptions::default().retry_on_initial_connect();
    let client = async_nats::connect_with_options(urls.clone(), connect_options).await?;
    let context = jetstream::new(client);
    let store = context
        .create_key_value(async_nats::jetstream::kv::Config {
            bucket: bucket.clone(),
            description,
            ..Default::default()
        })
        .await?;

    let store = NatsKvStore {
        bucket,
        store,
        _phantom: PhantomData,
    };
    Ok(store)
}

impl<T> interfaces::kv_store::KvStore<T> for NatsKvStore<T>
where
    T: BorshSerialize + BorshDeserialize,
{
    #[expect(refining_impl_trait)]
    async fn update(
        &self,
        data: interfaces::kv_store::WithRevision<T>,
    ) -> Result<interfaces::kv_store::WithRevision<T>, Error> {
        let value_bytes = borsh::to_vec(&data.value).map_err(Error::Serialize)?;
        let revision = data.revision;

        let revision = self
            .store
            .update(&self.bucket, value_bytes.clone().into(), revision)
            .await
            .map_err(Error::Update)?;

        Ok(interfaces::kv_store::WithRevision {
            value: data.value,
            revision,
        })
    }

    #[expect(refining_impl_trait)]
    async fn put(&self, value: T) -> Result<interfaces::kv_store::WithRevision<T>, Error> {
        let value_bytes = borsh::to_vec(&value).map_err(Error::Serialize)?;
        let revision = self
            .store
            .put(&self.bucket, value_bytes.clone().into())
            .await
            .map_err(Error::Put)?;

        Ok(interfaces::kv_store::WithRevision { value, revision })
    }

    #[expect(refining_impl_trait)]
    async fn get(&self) -> Result<Option<interfaces::kv_store::WithRevision<T>>, Error> {
        let entry = self.store.entry(&self.bucket).await.map_err(Error::Entry)?;

        entry
            .map(|entry| {
                let value =
                    T::deserialize(&mut entry.value.as_ref()).map_err(Error::Deserialize)?;

                Ok(interfaces::kv_store::WithRevision {
                    value,
                    revision: entry.revision,
                })
            })
            .transpose()
    }
}
