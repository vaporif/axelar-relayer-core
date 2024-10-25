//! Types for Axelar Amplifier API
//! Contsructed form the following API spec [link](https://github.com/axelarnetwork/axelar-eds-mirror/blob/main/oapi/gmp/schema.yaml)

pub use big_int::BigInt;
use bnum::types::U256;
use chrono::{DateTime, Utc};
pub use id::*;
use serde::{Deserialize, Deserializer, Serialize};

/// Represents an address as a non-empty string.
pub type Address = String;

/// Newtypes for different types of IDs so we don't mix them up in the future
mod id {

    use super::*;

    /// `NewType` for tracking transaction ids
    ///
    /// for in-depth docs reference [this document](https://bright-ambert-2bd.notion.site/Amplifier-GMP-API-EXTERNAL-911e740b570b4017826c854338b906c8#e8a7398607bd496eb0b8e95e887d6574)
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct TxId(pub String);

    /// `NewType` for tracking token ids
    ///
    /// for in-depth docs reference [this document](https://bright-ambert-2bd.notion.site/Amplifier-GMP-API-EXTERNAL-911e740b570b4017826c854338b906c8#e8a7398607bd496eb0b8e95e887d6574)
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct TokenId(pub String);

    /// Indicates a type in format of `TxHash-LogIndex`
    ///
    /// TxHash-LogIndex. for in-depth docs reference [this document](https://bright-ambert-2bd.notion.site/Amplifier-GMP-API-EXTERNAL-911e740b570b4017826c854338b906c8#e8a7398607bd496eb0b8e95e887d6574)
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct TxEvent(pub String);
    impl TxEvent {
        /// construct a new event identifier from the tx hash and the log index
        #[must_use]
        pub fn new(tx_hash: &str, log_index: usize) -> Self {
            Self(format!("{tx_hash}-{log_index}"))
        }
    }

    /// `NewType` for tracking message ids
    pub type MessageId = TxEvent;

    /// Id of chain `NativeGasPaidForContractCall`/ `NativeGasAdded` event in format
    pub type EventId = TxEvent;

    /// `NewType` for tracking task ids
    ///
    /// for in-depth docs reference [this document](https://bright-ambert-2bd.notion.site/Amplifier-GMP-API-EXTERNAL-911e740b570b4017826c854338b906c8#e8a7398607bd496eb0b8e95e887d6574)
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct TaskItemId(pub uuid::Uuid);

    /// `NewType` for tracking command ids.
    ///
    /// for in-depth docs reference [this document](https://bright-ambert-2bd.notion.site/Amplifier-GMP-API-EXTERNAL-911e740b570b4017826c854338b906c8#e8a7398607bd496eb0b8e95e887d6574)
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct CommandId(pub String);

    /// `NewType` for tracking request ids.
    ///
    /// for in-depth docs reference [this document](https://bright-ambert-2bd.notion.site/Amplifier-GMP-API-EXTERNAL-911e740b570b4017826c854338b906c8#e8a7398607bd496eb0b8e95e887d6574)
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct RequestId(pub String);
    #[expect(
        clippy::min_ident_chars,
        reason = "don't rename variable names from the trait"
    )]
    impl core::fmt::Display for RequestId {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.write_str(self.0.as_str())
        }
    }
}

mod serde_utils {
    use base64::prelude::*;
    use serde::{Deserialize as _, Deserializer, Serializer};

    pub(crate) fn base64_decode<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw_string = String::deserialize(deserializer)?;
        let bytes = BASE64_STANDARD
            .decode(raw_string)
            .inspect_err(|err| {
                tracing::error!(?err, "cannot parse base64 data");
            })
            .map_err(serde::de::Error::custom)?;
        Ok(bytes)
    }

    pub(crate) fn base64_encode<S>(data: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let encoded = BASE64_STANDARD.encode(data);
        serializer.serialize_str(&encoded)
    }
}

/// Enumeration of reasons why a message cannot be executed.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CannotExecuteMessageReason {
    /// Not enough gas to execute the message
    InsufficientGas,
    /// Other generic error
    Error,
}

/// Enumeration of message execution statuses.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MessageExecutionStatus {
    /// Message executed successfully
    Successful,
    /// Message reverted
    Reverted,
}

/// Represents metadata associated with an event.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventMetadata {
    /// tx id of the underlying event
    #[serde(rename = "txID", skip_serializing_if = "Option::is_none")]
    pub tx_id: Option<TxId>,
    /// timestamp of the underlying event
    #[serde(rename = "timestamp", skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<DateTime<Utc>>,
    /// sender address
    #[serde(rename = "fromAddress", skip_serializing_if = "Option::is_none")]
    pub from_address: Option<Address>,
    /// weather the event is finalized or not
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finalized: Option<bool>,
}

/// Specialized metadata for `CallEvent`.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallEventMetadata {
    /// common event data
    #[serde(flatten)]
    pub base: EventMetadata,
    /// the message id that's responsible for the event
    #[serde(rename = "parentMessageID", skip_serializing_if = "Option::is_none")]
    pub parent_message_id: Option<MessageId>,
}

/// Specialized metadata for `MessageApprovedEvent`.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageApprovedEventMetadata {
    /// common event data
    #[serde(flatten)]
    pub base: EventMetadata,
    /// The command id that corresponds to the approved message
    #[serde(rename = "commandID", skip_serializing_if = "Option::is_none")]
    pub command_id: Option<CommandId>,
}

/// Specialized metadata for `MessageExecutedEvent`.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageExecutedEventMetadata {
    /// common event data
    #[serde(flatten)]
    pub base: EventMetadata,
    /// The command id that corresponds to the executed message
    #[serde(rename = "commandID", skip_serializing_if = "Option::is_none")]
    pub command_id: Option<CommandId>,
    /// The message
    #[serde(rename = "childMessageIDs", skip_serializing_if = "Option::is_none")]
    pub child_message_ids: Option<Vec<MessageId>>,
}

/// Specialized metadata for `CannotExecuteMessageEvent`.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CannotExecuteMessageEventMetadata {
    /// The initiator of the message
    #[serde(rename = "fromAddress", skip_serializing_if = "Option::is_none")]
    pub from_address: Option<Address>,
    /// timestamp of the event
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<DateTime<Utc>>,
}

/// Represents a token amount, possibly with a token ID.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Token {
    /// indicates that amount is in native token if left blank
    #[serde(rename = "tokenID", skip_serializing_if = "Option::is_none")]
    pub token_id: Option<TokenId>,
    /// the amount in token’s denominator
    pub amount: BigInt,
}

/// Represents a cross-chain message.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GatewayV2Message {
    /// the message id of a GMP call
    #[serde(rename = "messageID")]
    pub message_id: MessageId,
    /// source chain
    #[serde(rename = "sourceChain")]
    pub source_chain: String,
    /// source address
    #[serde(rename = "sourceAddress")]
    pub source_address: Address,
    /// destination address
    #[serde(rename = "destinationAddress")]
    pub destination_address: Address,
    /// string representation of the payload hash
    #[serde(
        rename = "payloadHash",
        deserialize_with = "serde_utils::base64_decode",
        serialize_with = "serde_utils::base64_encode"
    )]
    pub payload_hash: Vec<u8>,
}

/// Base struct for events.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventBase {
    /// The event id
    #[serde(rename = "eventID")]
    pub event_id: EventId,
    /// Metadata of the event
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<EventMetadata>,
}

/// Represents a Gas Credit Event.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GasCreditEvent {
    /// Event base
    #[serde(flatten)]
    pub base: EventBase,
    /// Message ID
    #[serde(rename = "messageID")]
    pub message_id: MessageId,
    /// the refunder address of the `NativeGasPaidForContractCall`/ `NativeGasAdded` event
    #[serde(rename = "refundAddress")]
    pub refund_address: Address,
    /// payment for the Contract Call
    pub payment: Token,
}

/// Represents a Gas Refunded Event.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GasRefundedEvent {
    /// Event base
    #[serde(flatten)]
    pub base: EventBase,
    /// Message ID
    #[serde(rename = "messageID")]
    pub message_id: MessageId,
    /// address of the refund recipient
    #[serde(rename = "recipientAddress")]
    pub recipient_address: Address,
    /// the amount to refund
    #[serde(rename = "refundedAmount")]
    pub refunded_amount: Token,
    /// the cost of the refund transaction
    pub cost: Token,
}

/// Represents a Call Event.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallEvent {
    /// Event base
    #[serde(flatten)]
    pub base: EventBase,
    /// The cross chain message
    pub message: GatewayV2Message,
    /// Name of the destination chain
    #[serde(rename = "destinationChain")]
    pub destination_chain: String,
    /// string representation of the payload bytes
    #[serde(
        deserialize_with = "serde_utils::base64_decode",
        serialize_with = "serde_utils::base64_encode"
    )]
    pub payload: Vec<u8>,
}

/// Represents a Message Approved Event.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageApprovedEvent {
    /// Event base
    #[serde(flatten)]
    pub base: EventBase,
    /// The cross chain message
    pub message: GatewayV2Message,
    /// the cost of the approval. (#of approvals in transaction / transaction cost)
    pub cost: Token,
    /// Event metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<MessageApprovedEventMetadata>,
}

/// Represents a Message Executed Event.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageExecutedEvent {
    /// Event base
    #[serde(flatten)]
    pub base: EventBase,
    /// message id
    #[serde(rename = "messageID")]
    pub message_id: MessageId,
    #[serde(rename = "sourceChain")]
    /// source chian
    pub source_chain: String,
    /// message execution status
    pub status: MessageExecutionStatus,
    /// the cost of the transaction containing the execution
    pub cost: Token,
    /// event metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<MessageExecutedEventMetadata>,
}

/// Represents a Cannot Execute Message Event.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CannotExecuteMessageEvent {
    /// tevent base
    #[serde(flatten)]
    pub base: EventBase,
    /// task id
    #[serde(rename = "taskItemID")]
    pub task_item_id: TaskItemId,
    /// failed executioin reason
    pub reason: CannotExecuteMessageReason,
    /// details of the error
    pub details: String,
    /// event metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<CannotExecuteMessageEventMetadata>,
}

/// Represents a generic Event, which can be any of the specific event types.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Event {
    /// gas credit event
    GasCredit(GasCreditEvent),
    /// gas refunded event
    GasRefunded(GasRefundedEvent),
    /// call contract event
    Call(CallEvent),
    /// message approved event
    MessageApproved(MessageApprovedEvent),
    /// message executed event
    MessageExecuted(MessageExecutedEvent),
    /// cannot execute message event
    CannotExecuteMessage(CannotExecuteMessageEvent),
}

/// Represents the request payload for posting events.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublishEventsRequest {
    /// list of events to publish
    pub events: Vec<Event>,
}

/// Base struct for publish event result items.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublishEventResultItemBase {
    /// index of the event
    pub index: usize,
}

/// Represents an accepted publish event result.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublishEventAcceptedResult {
    /// event base
    #[serde(flatten)]
    pub base: PublishEventResultItemBase,
}

/// Represents an error in publishing an event.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublishEventErrorResult {
    /// event base
    #[serde(flatten)]
    pub base: PublishEventResultItemBase,
    /// error message
    pub error: String,
    /// weather we can retry publishing the event
    pub retriable: bool,
}

/// Represents the result of processing an event.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PublishEventResultItem {
    /// the event was accepted
    Accepted(PublishEventAcceptedResult),
    /// could not accept the event
    Error(PublishEventErrorResult),
}

/// Represents the response from posting events.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublishEventsResult {
    /// The result array
    pub results: Vec<PublishEventResultItem>,
}

/// Represents a Verify Task.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifyTask {
    /// the cross chain message
    pub message: GatewayV2Message,
    /// the raw payload of the task
    #[serde(
        deserialize_with = "serde_utils::base64_decode",
        serialize_with = "serde_utils::base64_encode"
    )]
    pub payload: Vec<u8>,
}

/// Represents a Gateway Transaction Task.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GatewayTransactionTask {
    /// the execute data for the gateway
    #[serde(
        deserialize_with = "serde_utils::base64_decode",
        serialize_with = "serde_utils::base64_encode"
    )]
    #[serde(rename = "executeData")]
    pub execute_data: Vec<u8>,
}

/// Represents an Execute Task.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecuteTask {
    /// the cross-chain message
    pub message: GatewayV2Message,
    #[serde(
        deserialize_with = "serde_utils::base64_decode",
        serialize_with = "serde_utils::base64_encode"
    )]
    /// the raw payload
    pub payload: Vec<u8>,
    /// The gas balance we currently have left
    #[serde(rename = "availableGasBalance")]
    pub available_gas_balance: Token,
}

/// Represents a Refund Task.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefundTask {
    /// the cross-chain message
    pub message: GatewayV2Message,
    #[serde(rename = "refundRecipientAddress")]
    /// the address that will receive the balance
    pub refund_recipient_address: Address,
    #[serde(rename = "remainingGasBalance")]
    /// how much balance is left post-refund
    pub remaining_gas_balance: Token,
}

/// Represents a generic Task, which can be any of the specific task types.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "task", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Task {
    /// Verify task
    Verify(VerifyTask),
    /// Gateway TX task
    GatewayTx(GatewayTransactionTask),
    /// Execute task
    Execute(ExecuteTask),
    /// Refund task
    Refund(RefundTask),
}

/// Represents an individual Task Item.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskItem {
    /// UUID of current task
    pub id: TaskItemId,
    /// timestamp of task’s creation
    pub timestamp: DateTime<Utc>,
    /// the inner task
    #[serde(flatten)]
    pub task: Task,
}

/// Represents the response from fetching tasks.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetTasksResult {
    /// Array of tasks matching the filters
    pub tasks: Vec<TaskItem>,
}

/// Represents an error response.
#[derive(Debug, thiserror::Error, PartialEq, Eq, Serialize, Deserialize)]
#[error("Amplifier API Error: {error}")]
pub struct ErrorResponse {
    /// error message
    pub error: String,
    /// the request id
    #[serde(rename = "requestID")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<RequestId>,
}

mod big_int {
    use super::*;

    /// Represents a big integer as a string matching the pattern `^(0|[1-9]\d*)$`.
    #[derive(Debug, PartialEq, Eq)]
    pub struct BigInt(pub bnum::types::U256);
    impl BigInt {
        /// Creates a new [`BigInt`].
        #[must_use]
        pub const fn new(num: U256) -> Self {
            Self(num)
        }

        /// Helper utility to transform u64 into a `BigInt`
        #[must_use]
        pub fn from_u64(num: u64) -> Self {
            Self(num.into())
        }
    }

    impl Serialize for BigInt {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            let string = self.0.to_string();
            serializer.serialize_str(&string)
        }
    }

    impl<'de> Deserialize<'de> for BigInt {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            let string = String::deserialize(deserializer)?;
            let number = bnum::types::U256::parse_str_radix(string.as_str(), 10);
            Ok(Self(number))
        }
    }
}
/// reference types are copied from the following documentation [link](https://bright-ambert-2bd.notion.site/Amplifier-GMP-API-EXTERNAL-911e740b570b4017826c854338b906c8#e8a7398607bd496eb0b8e95e887d6574)
#[cfg(test)]
mod tests {
    use base64::prelude::*;
    use pretty_assertions::assert_eq;
    use serde::de::DeserializeOwned;
    use simd_json::{from_slice, json, to_owned_value, to_string};

    use super::*;

    const BASE64_PAYLOAD: &str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAEAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAgAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAASaGVsbG8gdGVzdC1zZXBvbGlhAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA==";
    const BASE64_PAYLOAD_HASH: &str = "Y2YO3UuCRRackxbPYX9dWmNTYcnAMOommp9g4ydb3i4=";

    fn payload_bytes() -> Vec<u8> {
        BASE64_STANDARD.decode(BASE64_PAYLOAD).unwrap()
    }
    fn payload_hash_bytes() -> Vec<u8> {
        BASE64_STANDARD.decode(BASE64_PAYLOAD_HASH).unwrap()
    }

    fn test_serialization<T>(type_in_rust: &T, mut reference_json: Vec<u8>)
    where
        T: Serialize + DeserializeOwned + PartialEq + core::fmt::Debug,
    {
        // Action
        let mut type_in_rust_as_json = to_string(&type_in_rust).unwrap().into_bytes();

        // Assert
        assert_eq!(
            to_owned_value(type_in_rust_as_json.as_mut_slice()).unwrap(),
            to_owned_value(reference_json.as_mut_slice()).unwrap(),
            "During serialization process, the Rust type diverged from the reference"
        );

        let deserialized: T = from_slice(type_in_rust_as_json.as_mut_slice()).unwrap();

        assert_eq!(
            type_in_rust, &deserialized,
            "Deserialization caused the type to malform"
        );
    }
    #[test]
    fn test_gas_credit_event_serialization1() {
        // Setup
        let reference_json = simd_json::to_string(&json!({
            "eventID": "0x1fbe533b3bed6e6be3a74734042b3ae0836cabfdc2b646358080db75e9fe9ba8-1",
            "meta": {
                "txID": "0x1fbe533b3bed6e6be3a74734042b3ae0836cabfdc2b646358080db75e9fe9ba8",
                "timestamp": "2024-09-11T13:32:48Z",
                "fromAddress": "0xEA12282BaC49497793622d67e2CD43bf1065a819",
                "finalized": true
            },
            "messageID": "0x1fbe533b3bed6e6be3a74734042b3ae0836cabfdc2b646358080db75e9fe9ba8-2",
            "refundAddress": "0xEA12282BaC49497793622d67e2CD43bf1065a819",
            "payment": {
                "amount": "410727029715539"
            }
        }))
        .unwrap()
        .into_bytes();
        let type_in_rust = GasCreditEvent {
            base: EventBase {
                event_id: EventId::new(
                    "0x1fbe533b3bed6e6be3a74734042b3ae0836cabfdc2b646358080db75e9fe9ba8",
                    1,
                ),
                meta: Some(EventMetadata {
                    tx_id: Some(TxId(
                        "0x1fbe533b3bed6e6be3a74734042b3ae0836cabfdc2b646358080db75e9fe9ba8"
                            .to_owned(),
                    )),
                    timestamp: DateTime::parse_from_rfc3339("2024-09-11T13:32:48Z")
                        .map(|x| x.to_utc())
                        .ok(),
                    from_address: Some("0xEA12282BaC49497793622d67e2CD43bf1065a819".to_owned()),
                    finalized: Some(true),
                }),
            },
            message_id: MessageId::new(
                "0x1fbe533b3bed6e6be3a74734042b3ae0836cabfdc2b646358080db75e9fe9ba8",
                2,
            ),
            refund_address: "0xEA12282BaC49497793622d67e2CD43bf1065a819".to_owned(),
            payment: Token {
                token_id: None,
                amount: BigInt::from_u64(410_727_029_715_539),
            },
        };

        // Action
        test_serialization(&type_in_rust, reference_json);
    }

    #[test]
    fn test_gas_credit_event_serialization() {
        // Setup
        let reference_json = to_string(&json!({
            "eventID": "0x1fbe533b3bed6e6be3a74734042b3ae0836cabfdc2b646358080db75e9fe9ba8-1",
            "meta": {
                "txID": "0x1fbe533b3bed6e6be3a74734042b3ae0836cabfdc2b646358080db75e9fe9ba8",
                "timestamp": "2024-09-11T13:32:48Z",
                "fromAddress": "0xEA12282BaC49497793622d67e2CD43bf1065a819",
                "finalized": true
            },
            "messageID": "0x1fbe533b3bed6e6be3a74734042b3ae0836cabfdc2b646358080db75e9fe9ba8-2",
            "refundAddress": "0xEA12282BaC49497793622d67e2CD43bf1065a819",
            "payment": {
                "amount": "410727029715539"
            }
        }))
        .unwrap()
        .into_bytes();

        let type_in_rust = GasCreditEvent {
            base: EventBase {
                event_id: MessageId::new(
                    "0x1fbe533b3bed6e6be3a74734042b3ae0836cabfdc2b646358080db75e9fe9ba8",
                    1,
                ),
                meta: Some(EventMetadata {
                    tx_id: Some(TxId(
                        "0x1fbe533b3bed6e6be3a74734042b3ae0836cabfdc2b646358080db75e9fe9ba8"
                            .to_owned(),
                    )),
                    timestamp: DateTime::parse_from_rfc3339("2024-09-11T13:32:48Z")
                        .map(|x| x.to_utc())
                        .ok(),
                    from_address: Some("0xEA12282BaC49497793622d67e2CD43bf1065a819".to_owned()),
                    finalized: Some(true),
                }),
            },
            message_id: MessageId::new(
                "0x1fbe533b3bed6e6be3a74734042b3ae0836cabfdc2b646358080db75e9fe9ba8",
                2,
            ),
            refund_address: "0xEA12282BaC49497793622d67e2CD43bf1065a819".to_owned(),
            payment: Token {
                token_id: None,
                amount: BigInt::from_u64(410_727_029_715_539),
            },
        };

        test_serialization(&type_in_rust, reference_json);
    }

    #[test]
    fn test_call_event_serialization() {
        // Setup
        let reference_json = to_string(&json!({
            "eventID": "0xe432150cce91c13a887f7D836923d5597adD8E31-2",
            "message": {
                "messageID": "0xe432150cce91c13a887f7D836923d5597adD8E31-2",
                "sourceChain": "ethereum",
                "sourceAddress": "0xEA12282BaC49497793622d67e2CD43bf1065a819",
                "destinationAddress": "0xf4ff91f79E35E7dF460A6B259fD971Ec85E933CF",
                "payloadHash": BASE64_PAYLOAD_HASH
            },
            "destinationChain": "Avalanche",
            "payload": BASE64_PAYLOAD
        }))
        .unwrap()
        .into_bytes();

        let type_in_rust = CallEvent {
            base: EventBase {
                event_id: EventId::new("0xe432150cce91c13a887f7D836923d5597adD8E31", 2),
                meta: None,
            },
            message: GatewayV2Message {
                message_id: MessageId::new("0xe432150cce91c13a887f7D836923d5597adD8E31", 2),
                source_chain: "ethereum".to_owned(),
                source_address: "0xEA12282BaC49497793622d67e2CD43bf1065a819".to_owned(),
                destination_address: "0xf4ff91f79E35E7dF460A6B259fD971Ec85E933CF".to_owned(),
                payload_hash: payload_hash_bytes(),
            },
            destination_chain: "Avalanche".to_owned(),
            payload: payload_bytes(),
        };

        test_serialization(&type_in_rust, reference_json);
    }

    #[test]
    fn test_event_enum_serialization() {
        // Setup
        let reference_json = to_string(&json!({
            "type": "CALL",
            "eventID": "0xe432150cce91c13a887f7D836923d5597adD8E31-2",
            "message": {
                "messageID": "0xe432150cce91c13a887f7D836923d5597adD8E31-2",
                "sourceChain": "ethereum",
                "sourceAddress": "0xEA12282BaC49497793622d67e2CD43bf1065a819",
                "destinationAddress": "0xf4ff91f79E35E7dF460A6B259fD971Ec85E933CF",
                "payloadHash": BASE64_PAYLOAD_HASH
            },
            "destinationChain": "Avalanche",
            "payload": BASE64_PAYLOAD
        }))
        .unwrap()
        .into_bytes();

        let type_in_rust = Event::Call(CallEvent {
            base: EventBase {
                event_id: EventId::new("0xe432150cce91c13a887f7D836923d5597adD8E31", 2),
                meta: None,
            },
            message: GatewayV2Message {
                message_id: MessageId::new("0xe432150cce91c13a887f7D836923d5597adD8E31", 2),
                source_chain: "ethereum".to_owned(),
                source_address: "0xEA12282BaC49497793622d67e2CD43bf1065a819".to_owned(),
                destination_address: "0xf4ff91f79E35E7dF460A6B259fD971Ec85E933CF".to_owned(),
                payload_hash: payload_hash_bytes(),
            },
            destination_chain: "Avalanche".to_owned(),
            payload: payload_bytes(),
        });

        test_serialization(&type_in_rust, reference_json);
    }

    #[test]
    fn test_tasks_response_deserialization() {
        // Setup
        let execute_data = vec![111];
        let execute_data_encoded = BASE64_STANDARD.encode(&execute_data);

        let reference_json = to_string(&json!({
              "tasks": [{
                "id": "550e8400-e29b-41d4-a716-446655440000",
                "timestamp": "2023-04-01T12:00:00Z",
                "type": "GATEWAY_TX",
                "task": {
                  "executeData": execute_data_encoded
                }
              }]
        }))
        .unwrap()
        .into_bytes();

        let type_in_rust = GetTasksResult {
            tasks: vec![TaskItem {
                id: TaskItemId("550e8400-e29b-41d4-a716-446655440000".parse().unwrap()),
                timestamp: DateTime::parse_from_rfc3339("2023-04-01T12:00:00Z")
                    .map(|x| x.to_utc())
                    .unwrap(),
                task: Task::GatewayTx(GatewayTransactionTask { execute_data }),
            }],
        };
        test_serialization(&type_in_rust, reference_json);
    }

    #[test]
    fn test_publish_events_request_serialization() {
        // Setup
        let reference_json = to_string(&json!({
            "events": [
                {
                    "type": "CALL",
                    "eventID": "0x9b447614be654eeea0c5de0319b3f2c243ab45bebd914a1f7319f4bb599d8968-1",
                    "meta": {
                        "txID": "0x9b447614be654eeea0c5de0319b3f2c243ab45bebd914a1f7319f4bb599d8968",
                        "timestamp": "2024-09-11T13:32:48Z",
                        "fromAddress": "0xba76c6980428A0b10CFC5d8ccb61949677A61233",
                        "finalized": true
                    },
                    "message": {
                        "messageID": "0x9b447614be654eeea0c5de0319b3f2c243ab45bebd914a1f7319f4bb599d8968-1",
                        "sourceChain": "test-sepolia",
                        "sourceAddress": "0x9e3e785dD9EA3826C9cBaFb1114868bc0e79539a",
                        "destinationAddress": "0xE8E348fA7b311d6E308b1A162C3ec0172B37D1C1",
                        "payloadHash": BASE64_PAYLOAD_HASH
                    },
                    "destinationChain": "test-avalanche",
                    "payload": BASE64_PAYLOAD
                }
            ]
        }))
        .unwrap()
        .into_bytes();

        let type_in_rust = PublishEventsRequest {
            events: vec![Event::Call(CallEvent {
                base: EventBase {
                    event_id: EventId::new(
                        "0x9b447614be654eeea0c5de0319b3f2c243ab45bebd914a1f7319f4bb599d8968",
                        1,
                    ),
                    meta: Some(EventMetadata {
                        tx_id: Some(TxId(
                            "0x9b447614be654eeea0c5de0319b3f2c243ab45bebd914a1f7319f4bb599d8968"
                                .to_owned(),
                        )),
                        timestamp: DateTime::parse_from_rfc3339("2024-09-11T13:32:48Z")
                            .map(|x| x.to_utc())
                            .ok(),
                        from_address: Some("0xba76c6980428A0b10CFC5d8ccb61949677A61233".to_owned()),
                        finalized: Some(true),
                    }),
                },
                message: GatewayV2Message {
                    message_id: MessageId::new(
                        "0x9b447614be654eeea0c5de0319b3f2c243ab45bebd914a1f7319f4bb599d8968",
                        1,
                    ),
                    source_chain: "test-sepolia".to_owned(),
                    source_address: "0x9e3e785dD9EA3826C9cBaFb1114868bc0e79539a".to_owned(),
                    destination_address: "0xE8E348fA7b311d6E308b1A162C3ec0172B37D1C1".to_owned(),
                    payload_hash: payload_hash_bytes(),
                },
                destination_chain: "test-avalanche".to_owned(),
                payload: payload_bytes(),
            })],
        };

        test_serialization(&type_in_rust, reference_json);
    }

    #[test]
    fn test_payload_base64_serialization() {
        // Setup
        let type_in_rust = CallEvent {
            base: EventBase {
                event_id: TxEvent("event123".to_owned()),
                meta: None,
            },
            message: GatewayV2Message {
                message_id: TxEvent("message123".to_owned()),
                source_chain: "chainA".to_owned(),
                source_address: "0xSourceAddress".to_owned(),
                destination_address: "0xDestinationAddress".to_owned(),
                payload_hash: payload_hash_bytes(),
            },
            destination_chain: "chainB".to_owned(),
            payload: payload_bytes(),
        };

        let reference_json = to_string(&json!({
            "eventID": "event123",
            "message": {
                "messageID": "message123",
                "sourceChain": "chainA",
                "sourceAddress": "0xSourceAddress",
                "destinationAddress": "0xDestinationAddress",
                "payloadHash": BASE64_PAYLOAD_HASH
            },
            "destinationChain": "chainB",
            "payload": BASE64_PAYLOAD
        }))
        .unwrap()
        .into_bytes();

        test_serialization(&type_in_rust, reference_json);
    }

    #[test]
    fn test_cannot_execute_message_serialization() {
        // Setup
        let reference_json = to_string(&json!({
            "type": "CANNOT_EXECUTE_MESSAGE",
            "eventID": "event123",
            "taskItemID": "550e8400-e29b-41d4-a716-446655440000",
            "reason": "INSUFFICIENT_GAS",
            "details": "Not enough gas to execute the message"
        }))
        .unwrap()
        .into_bytes();

        let type_in_rust = Event::CannotExecuteMessage(CannotExecuteMessageEvent {
            base: EventBase {
                event_id: TxEvent("event123".to_owned()),
                meta: None,
            },
            task_item_id: TaskItemId("550e8400-e29b-41d4-a716-446655440000".parse().unwrap()),
            reason: CannotExecuteMessageReason::InsufficientGas,
            details: "Not enough gas to execute the message".to_owned(),
            meta: None,
        });

        test_serialization(&type_in_rust, reference_json);
    }

    #[test]
    fn can_deserialize_error() {
        let reference_json = br#"{"error":"no matching operation was found"}"#;
        let type_in_rust = ErrorResponse {
            error: "no matching operation was found".to_owned(),
            request_id: None,
        };

        test_serialization(&type_in_rust, reference_json.to_vec());
    }
}
