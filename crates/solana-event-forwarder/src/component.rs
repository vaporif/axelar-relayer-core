use core::future::Future;
use core::pin::Pin;

use axelar_message_primitives::U256;
use axelar_rkyv_encoding::rkyv::{self, Deserialize as _};
use futures::{SinkExt as _, StreamExt as _};
use gmp_gateway::events::{EventContainer, GatewayEvent};
use relayer_amplifier_api_integration::amplifier_api::types::{
    BigInt, CallEvent, CallEventMetadata, CommandId, Event, EventBase, EventId, EventMetadata,
    GatewayV2Message, MessageApprovedEvent, MessageApprovedEventMetadata, MessageId,
    PublishEventsRequest, SignersRotatedEvent, SignersRotatedMetadata, Token, TxEvent, TxId,
};
use relayer_amplifier_api_integration::AmplifierCommand;
use solana_sdk::pubkey::Pubkey;

/// The core component that is responsible for ingesting raw Solana events.
///
/// As a result, the logs get parsed, filtererd and mapped to Amplifier API events.
#[derive(Debug)]
pub struct SolanaEventForwarder {
    config: crate::Config,
    solana_listener_client: solana_listener::SolanaListenerClient,
    amplifier_client: relayer_amplifier_api_integration::AmplifierCommandClient,
}

impl relayer_engine::RelayerComponent for SolanaEventForwarder {
    fn process(self: Box<Self>) -> Pin<Box<dyn Future<Output = eyre::Result<()>> + Send>> {
        use futures::FutureExt as _;

        self.process_internal().boxed()
    }
}

impl SolanaEventForwarder {
    /// Instantiate a new `SolanaEventForwarder` using the pre-configured configuration.
    #[must_use]
    pub const fn new(
        config: crate::Config,
        solana_listener_client: solana_listener::SolanaListenerClient,
        amplifier_client: relayer_amplifier_api_integration::AmplifierCommandClient,
    ) -> Self {
        Self {
            config,
            solana_listener_client,
            amplifier_client,
        }
    }

    #[tracing::instrument(skip_all, name = "Solana log forwarder")]
    pub(crate) async fn process_internal(mut self) -> eyre::Result<()> {
        let match_context = MatchContext::new(self.config.gateway_program_id.to_string().as_str());

        while let Some(message) = self.solana_listener_client.log_receiver.next().await {
            let gateway_program_stack = build_program_event_stack(&match_context, &message.logs);
            let total_cost = message.cost_in_lamports;

            // Collect all successful events into a vector
            let events_vec = gateway_program_stack
                .into_iter()
                .filter_map(|x| {
                    if let ProgramInvocationState::Succeeded(events) = x {
                        Some(events)
                    } else {
                        None
                    }
                })
                .flatten()
                .collect::<Vec<_>>();

            // Calculate the number of events
            let num_events = events_vec.len();

            // Compute the price per event, handling the case where num_events is zero
            let price_for_event = total_cost.checked_div(num_events.try_into()?).unwrap_or(0);

            // Map the events to amplifier events with the calculated price
            let events_to_send = events_vec
                .into_iter()
                .filter_map(|(log_index, event)| {
                    map_gateway_event_to_amplifier_event(
                        self.config.source_chain_name.as_str(),
                        &event,
                        &message,
                        log_index,
                        price_for_event,
                    )
                })
                .collect::<Vec<_>>();

            // Only send events if there are any from successful invocations
            if events_to_send.is_empty() {
                continue;
            }

            tracing::info!(count = ?events_to_send.len(), "sending solana events to amplifier component");
            let command = AmplifierCommand::PublishEvents(PublishEventsRequest {
                events: events_to_send,
            });
            self.amplifier_client.sender.send(command).await?;
        }
        eyre::bail!("Listener has stopped unexpectedly");
    }
}

#[derive(Debug, PartialEq, Eq)]
enum ProgramInvocationState {
    InProgress(Vec<(usize, EventContainer)>),
    Succeeded(Vec<(usize, EventContainer)>),
    Failed,
}

#[expect(clippy::struct_field_names, reason = "improves readability")]
struct MatchContext {
    /// the log prefix that indicates that we've entered the target program
    expected_start: String,
    /// the log prefix that indicates that the target program succeeded
    expected_success: String,
    /// the log prefix that indicates that the target program failed
    expected_failure: String,
}

impl MatchContext {
    pub(crate) fn new(gateway_program_id: &str) -> Self {
        Self {
            expected_start: format!("Program {gateway_program_id} invoke"),
            expected_success: format!("Program {gateway_program_id} success"),
            expected_failure: format!("Program {gateway_program_id} failed"),
        }
    }
}

fn build_program_event_stack<T>(ctx: &MatchContext, logs: &[T]) -> Vec<ProgramInvocationState>
where
    T: AsRef<str>,
{
    let logs = logs.iter().enumerate();
    let mut program_stack: Vec<ProgramInvocationState> = Vec::new();

    for (idx, log) in logs {
        tracing::trace!(log =?log.as_ref(), "incoming log from Solana");
        if log.as_ref().starts_with(ctx.expected_start.as_str()) {
            // Start a new program invocation
            program_stack.push(ProgramInvocationState::InProgress(Vec::new()));
        } else if log.as_ref().starts_with(ctx.expected_success.as_str()) {
            handle_success_log(&mut program_stack);
        } else if log.as_ref().starts_with(ctx.expected_failure.as_str()) {
            handle_failure_log(&mut program_stack);
        } else {
            // Process logs if inside a program invocation
            parse_execution_log(&mut program_stack, log, idx);
        }
    }
    program_stack
}

fn parse_execution_log<T>(program_stack: &mut [ProgramInvocationState], log: &T, idx: usize)
where
    T: AsRef<str>,
{
    let Some(&mut ProgramInvocationState::InProgress(ref mut events)) = program_stack.last_mut()
    else {
        return;
    };
    let Some(gateway_event) = GatewayEvent::parse_log(log.as_ref()) else {
        return;
    };
    events.push((idx, gateway_event));
}

fn handle_failure_log(program_stack: &mut Vec<ProgramInvocationState>) {
    // Mark the current program invocation as failed
    if program_stack.pop().is_some() {
        program_stack.push(ProgramInvocationState::Failed);
    } else {
        tracing::warn!("Program failure without matching invocation");
    }
}

fn handle_success_log(program_stack: &mut Vec<ProgramInvocationState>) {
    // Mark the current program invocation as succeeded
    let Some(state) = program_stack.pop() else {
        tracing::warn!("Program success without matching invocation");
        return;
    };
    match state {
        ProgramInvocationState::InProgress(events) => {
            program_stack.push(ProgramInvocationState::Succeeded(events));
        }
        ProgramInvocationState::Succeeded(_) | ProgramInvocationState::Failed => {
            // This should not happen
            tracing::warn!("Unexpected state when marking program success");
        }
    }
}

#[expect(
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    reason = "easier to read when all the transformations in one place rather than scattered around"
)]
fn map_gateway_event_to_amplifier_event(
    source_chain: &str,
    event: &EventContainer,
    message: &solana_listener::SolanaTransaction,
    log_index: usize,
    price_per_event_in_lamports: u64,
) -> Option<Event> {
    use gmp_gateway::events::ArchivedGatewayEvent::{
        CallContract, MessageApproved, MessageExecuted, OperatorshipTransferred, SignersRotated,
    };
    let signature = message.signature.to_string();
    let event_id = EventId::new(&signature, log_index);
    let tx_id = TxId(signature.clone());

    #[expect(
        clippy::little_endian_bytes,
        reason = "we are guaranteed correct conversion"
    )]
    match *event.parse() {
        CallContract(ref call_contract) => {
            let message_id = MessageId::new(&signature, log_index);
            let source_address = Pubkey::new_from_array(call_contract.sender).to_string();
            let amplifier_event = Event::Call(
                CallEvent::builder()
                    .base(
                        EventBase::builder()
                            .event_id(event_id)
                            .meta(Some(
                                EventMetadata::builder()
                                    .tx_id(Some(tx_id))
                                    .timestamp(message.timestamp)
                                    .from_address(Some(source_address.clone()))
                                    .finalized(Some(true))
                                    .extra(CallEventMetadata::builder().build())
                                    .build(),
                            ))
                            .build(),
                    )
                    .message(
                        GatewayV2Message::builder()
                            .message_id(message_id)
                            .source_chain(source_chain.to_owned())
                            .source_address(source_address)
                            .destination_address(call_contract.destination_address.to_string())
                            .payload_hash(call_contract.payload_hash.to_vec())
                            .build(),
                    )
                    .destination_chain(call_contract.destination_chain.to_string())
                    .payload(call_contract.payload.to_vec())
                    .build(),
            );
            Some(amplifier_event)
        }
        SignersRotated(ref signers) => {
            tracing::info!(?signers, "Signers rotated");

            let decoded_u256: U256 = signers
                .new_epoch
                .deserialize(&mut rkyv::Infallible)
                .unwrap();
            let le_bytes = decoded_u256.to_le_bytes();
            let (le_u64, _) = le_bytes.split_first_chunk::<8>()?;
            let epoch = u64::from_le_bytes(*le_u64);

            let amplifier_event = Event::SignersRotated(
                SignersRotatedEvent::builder()
                    .base(
                        EventBase::builder()
                            .event_id(event_id)
                            .meta(Some(
                                EventMetadata::builder()
                                    .tx_id(Some(tx_id))
                                    .timestamp(message.timestamp)
                                    .finalized(Some(true))
                                    .extra(
                                        SignersRotatedMetadata::builder()
                                            .signer_hash(signers.new_signers_hash.to_vec())
                                            .epoch(epoch)
                                            .build(),
                                    )
                                    .build(),
                            ))
                            .build(),
                    )
                    .cost(
                        Token::builder()
                            .token_id(None)
                            .amount(BigInt::from_u64(price_per_event_in_lamports))
                            .build(),
                    )
                    .build(),
            );
            Some(amplifier_event)
        }
        MessageApproved(ref approved_message) => {
            let command_id = approved_message.command_id;
            let message_id = TxEvent(
                String::from_utf8(approved_message.message_id.to_vec())
                    .expect("message id is not a valid String"),
            );
            let amplifier_event = Event::MessageApproved(
                MessageApprovedEvent::builder()
                    .base(
                        EventBase::builder()
                            .event_id(event_id)
                            .meta(Some(
                                EventMetadata::builder()
                                    .tx_id(Some(tx_id))
                                    .timestamp(message.timestamp)
                                    .from_address(Some(
                                        String::from_utf8(approved_message.source_address.to_vec())
                                            .expect("source address is not a valid string"),
                                    ))
                                    .finalized(Some(true))
                                    .extra(
                                        MessageApprovedEventMetadata::builder()
                                            .command_id(Some(CommandId(
                                                bs58::encode(command_id).into_string(),
                                            )))
                                            .build(),
                                    )
                                    .build(),
                            ))
                            .build(),
                    )
                    .message(
                        GatewayV2Message::builder()
                            .message_id(message_id)
                            .source_chain(
                                String::from_utf8(approved_message.source_chain.to_vec())
                                    .expect("invalid source chain"),
                            )
                            .source_address(
                                String::from_utf8(approved_message.source_address.to_vec())
                                    .expect("invalid source address"),
                            )
                            .destination_address(
                                Pubkey::new_from_array(approved_message.destination_address)
                                    .to_string(),
                            )
                            .payload_hash(approved_message.payload_hash.to_vec())
                            .build(),
                    )
                    .cost(
                        Token::builder()
                            .amount(BigInt::from_u64(price_per_event_in_lamports))
                            .build(),
                    )
                    .build(),
            );
            tracing::info!(?approved_message, "Message approved");
            Some(amplifier_event)
        }
        MessageExecuted(ref _executed_message) => {
            tracing::warn!(
                "current gateway event does not produce enough artifacts to relay this message"
            );
            None
        }
        OperatorshipTransferred(ref new_operatorship) => {
            tracing::info!(?new_operatorship, "Operatorship transferred");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use gmp_gateway::events::CallContract;
    use pretty_assertions::assert_eq;
    use test_log::test;

    use super::*;

    static GATEWAY_EXAMPLE_ID: &str = "gtwEpzTprUX7TJLx1hFXNeqCXJMsoxYQhQaEbnuDcj1";

    // Include the test_call_data function
    fn fixture_call_data() -> (String, EventContainer) {
        use base64::prelude::*;
        // Simple `CallContract` fixture
        let event = gmp_gateway::events::GatewayEvent::CallContract(CallContract {
            sender: Pubkey::new_unique().to_bytes(),
            destination_chain: "ethereum".to_owned(),
            destination_address: "0x9e3e785dD9EA3826C9cBaFb1114868bc0e79539a".to_owned(),
            payload: vec![42, 42],
            payload_hash: Pubkey::new_unique().to_bytes(),
        });
        let event = event.encode();
        let event_container = EventContainer::new(event.to_vec()).unwrap();
        let base64_data = BASE64_STANDARD.encode(&event);
        (base64_data, event_container)
    }

    fn fixture_match_context() -> MatchContext {
        MatchContext::new(GATEWAY_EXAMPLE_ID)
    }

    #[test]
    fn test_simple_event() {
        // Use the test_call_data fixture
        let (base64_data, event) = fixture_call_data();

        // Sample logs with multiple gateway calls, some succeed and some fail
        let logs = vec![
            format!("Program {GATEWAY_EXAMPLE_ID} invoke [1]"), // Invocation 1 starts
            "Program log: Instruction: Call Contract".to_owned(),
            format!("Program data: {}", base64_data),
            format!("Program {GATEWAY_EXAMPLE_ID} success"), // Invocation 1 succeeds
        ];

        let result = build_program_event_stack(&fixture_match_context(), &logs);

        // Expected result: two successful invocations with their events, one failed invocation
        let expected = vec![ProgramInvocationState::Succeeded(vec![(2, event)])];

        assert_eq!(result, expected);
    }

    #[test]
    fn test_multiple_gateway_calls_some_succeed_some_fail() {
        // Use the test_call_data fixture
        let (base64_data, event) = fixture_call_data();

        // Sample logs with multiple gateway calls, some succeed and some fail
        let logs = vec![
            format!("Program {GATEWAY_EXAMPLE_ID} invoke [1]"), // Invocation 1 starts
            "Program log: Instruction: Call Contract".to_owned(),
            format!("Program data: {}", base64_data),
            format!("Program {GATEWAY_EXAMPLE_ID} success"), // Invocation 1 succeeds
            format!("Program {GATEWAY_EXAMPLE_ID} invoke [2]"), // Invocation 2 starts
            "Program log: Instruction: Call Contract".to_owned(),
            format!("Program data: {}", base64_data),
            format!("Program {GATEWAY_EXAMPLE_ID} failed"), // Invocation 2 fails
            format!("Program {GATEWAY_EXAMPLE_ID} invoke [3]"), // Invocation 3 starts
            "Program log: Instruction: Call Contract".to_owned(),
            format!("Program data: {}", base64_data),
            format!("Program {GATEWAY_EXAMPLE_ID} success"), // Invocation 3 succeeds
        ];

        let result = build_program_event_stack(&fixture_match_context(), &logs);

        // Expected result: two successful invocations with their events, one failed invocation
        let expected = vec![
            ProgramInvocationState::Succeeded(vec![(2, event.clone())]),
            ProgramInvocationState::Failed,
            ProgramInvocationState::Succeeded(vec![(10, event)]),
        ];

        assert_eq!(result, expected);
    }

    #[test]
    fn test_no_gateway_calls() {
        // Logs with no gateway calls
        let logs = vec![
            "Program some_other_program invoke [1]".to_owned(),
            "Program log: Instruction: Do something".to_owned(),
            "Program some_other_program success".to_owned(),
        ];

        let result = build_program_event_stack(&fixture_match_context(), &logs);

        // Expected result: empty stack
        let expected: Vec<ProgramInvocationState> = Vec::new();

        assert_eq!(result, expected);
    }

    #[test]
    fn test_gateway_call_with_no_events() {
        // Gateway call that succeeds but has no events
        let logs = vec![
            format!("Program {GATEWAY_EXAMPLE_ID} invoke [1]"),
            "Program log: Instruction: Do something".to_owned(),
            format!("Program {GATEWAY_EXAMPLE_ID} success"),
        ];

        let result = build_program_event_stack(&fixture_match_context(), &logs);

        // Expected result: one successful invocation with no events
        let expected = vec![ProgramInvocationState::Succeeded(vec![])];

        assert_eq!(result, expected);
    }

    #[test]
    fn test_gateway_call_failure_with_events() {
        // Use the test_call_data fixture
        let (base64_data, _event) = fixture_call_data();

        // Gateway call that fails but has events (events should be discarded)
        let logs = vec![
            format!("Program {GATEWAY_EXAMPLE_ID} invoke [1]"),
            format!("Program data: {}", base64_data),
            format!("Program {GATEWAY_EXAMPLE_ID} failed"),
        ];

        let result = build_program_event_stack(&fixture_match_context(), &logs);

        // Expected result: one failed invocation
        let expected = vec![ProgramInvocationState::Failed];

        assert_eq!(result, expected);
    }

    #[test]
    fn test_real_life_data_set() {
        let logs = vec![
            "Program meme9s3tVXUYLLmPomrk36sEDodjKu4zjUuKn12tSic invoke [1] ",
            "Program log: Instruction: Native ",
            "Program log: Instruction: SendToGateway ",
            "Program gtwEpzTprUX7TJLx1hFXNeqCXJMsoxYQhQaEbnuDcj1 invoke [2] ",
            "Program log: Instruction: Call Contract ",
            "Program data: YXZhbGFuY2hlLWZ1amkweDQzNjZhMDQxYkE0MjM3RjliNzU1M0I4YTczZThBRjFmMmVlMUY0ZDFoZWxsbwAAAAAAAAABGmAGtWFiOdXkE8Fe9cROo6h2dsYneWtnOVWA1xsoSByK/5UGhcLtS8MXTzRyKHtW2VF7nJSBJzGaCaejberIDgAAAHz///8qAAAAgv///6T///8FAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
            "Program gtwEpzTprUX7TJLx1hFXNeqCXJMsoxYQhQaEbnuDcj1 consumed 4737 of 196074 compute units ",
            "Program gtwEpzTprUX7TJLx1hFXNeqCXJMsoxYQhQaEbnuDcj1 success ",
            "Program meme9s3tVXUYLLmPomrk36sEDodjKu4zjUuKn12tSic consumed 8781 of 200000 compute units ",
            "Program meme9s3tVXUYLLmPomrk36sEDodjKu4zjUuKn12tSic success          ",
        ];
        let ctx = MatchContext::new("gtwEpzTprUX7TJLx1hFXNeqCXJMsoxYQhQaEbnuDcj1");
        let mut result = build_program_event_stack(&ctx, &logs);
        assert_eq!(result.len(), 1);
        let state = result.pop().unwrap();
        let ProgramInvocationState::Succeeded(item) = state else {
            panic!("state is not of `succeeded` version");
        };
        assert_eq!(item.len(), 1);
    }
}
