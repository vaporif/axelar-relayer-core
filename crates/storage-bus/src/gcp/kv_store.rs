use std::fmt::Debug;
use std::marker::PhantomData;

use firestore::FirestoreDb;
use serde::{Deserialize, Serialize};

use super::error::Error;
use crate::interfaces;

pub struct GcpKvStore<T> {
    collection_id: String,
    document_id: String,
    db: FirestoreDb,
    _phantom: PhantomData<T>,
}

impl<T> GcpKvStore<T>
where
    T: Serialize + for<'de> Deserialize<'de> + Send + Sync,
{
    pub async fn connect(
        collection_id: String,
        document_id: String,
        project_id: &str,
    ) -> Result<Self, Error> {
        let db = FirestoreDb::new(&project_id).await?;
        Ok(Self {
            collection_id,
            document_id,
            db,
            _phantom: PhantomData,
        })
    }

    pub(crate) async fn upsert(
        &self,
        value: T,
    ) -> Result<interfaces::kv_store::WithRevision<T>, Error> {
        let _: T = self
            .db
            .fluent()
            .update()
            .in_col(&self.collection_id)
            .document_id(&self.document_id)
            .object(&value)
            .execute()
            .await?;
        Ok(interfaces::kv_store::WithRevision { value, revision: 0 })
    }
}

// Revision is not used here
// TODO: remove it from interfaces?
impl<T> interfaces::kv_store::KvStore<T> for GcpKvStore<T>
where
    T: Serialize + for<'de> Deserialize<'de> + Send + Sync + Debug,
{
    #[allow(refining_impl_trait)]
    #[tracing::instrument(skip(self))]
    async fn update(
        &self,
        data: interfaces::kv_store::WithRevision<T>,
    ) -> Result<interfaces::kv_store::WithRevision<T>, Error> {
        self.upsert(data.value).await
    }

    #[allow(refining_impl_trait)]
    #[tracing::instrument(skip(self))]
    async fn put(&self, value: T) -> Result<interfaces::kv_store::WithRevision<T>, Error> {
        self.upsert(value).await
    }

    #[allow(refining_impl_trait)]
    #[tracing::instrument(skip(self))]
    async fn get(&self) -> Result<Option<interfaces::kv_store::WithRevision<T>>, Error> {
        let document: Option<T> = self
            .db
            .fluent()
            .select()
            .by_id_in(&self.collection_id)
            .obj()
            .one(&self.document_id)
            .await?;

        document
            .map(|value| Ok(interfaces::kv_store::WithRevision { value, revision: 0 }))
            .transpose()
    }
}
