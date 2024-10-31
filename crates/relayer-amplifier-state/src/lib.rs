//! Abstract state interfacte for the Amplifier component

use amplifier_api::types::TaskItemId;

/// State interfacte to be used by the Amplifier relayer component
pub trait State: Clone + Send + Sync + 'static {
    /// Geneirc error
    type Err: core::error::Error + Send + Sync + 'static;

    /// Get the latest stored task item id
    fn latest_processed_task_id(&self) -> Option<TaskItemId>;

    /// Get the latest stored task item id
    fn latest_queried_task_id(&self) -> Option<TaskItemId>;

    /// Store the latest processed task item id
    ///
    /// # Errors
    /// Depends on the implementation details
    fn set_latest_processed_task_id(&self, task_item_id: TaskItemId) -> Result<(), Self::Err>;

    /// Store the latest queried task item id
    ///
    /// # Errors
    /// Depends on the implementation details
    fn set_latest_queried_task_id(&self, task_item_id: TaskItemId) -> Result<(), Self::Err>;
}
