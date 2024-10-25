use core::future::Future;
use core::pin::Pin;
use core::task::Poll;
use std::collections::VecDeque;
use std::sync::Arc;

use amplifier_api::types::TaskItem;
use axelar_rkyv_encoding::types::{HasheableMessageVec, VerifierSet};
use effective_tx_sender::ComputeBudgetError;
use futures::stream::{FusedStream as _, FuturesUnordered};
use futures::StreamExt as _;
use gmp_gateway::commands::OwnedCommand;
use gmp_gateway::state::GatewayApprovedCommand;
use gmp_gateway::{hasher_impl, instructions};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_response::RpcSimulateTransactionResult;
use solana_sdk::instruction::{Instruction, InstructionError};
use solana_sdk::program_pack::Pack as _;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signature};
use solana_sdk::signer::Signer as _;
use solana_sdk::transaction::TransactionError;
use tokio::task::JoinSet;
use tracing::{instrument, Instrument as _};

use crate::config;

/// A component that pushes transactions over to the Solana blockchain.
/// The transactions to push are dependant on the events that the Amplifier API will provide
pub struct SolanaTxPusher {
    config: config::Config,
    rpc_client: Arc<RpcClient>,
    task_receiver: relayer_amplifier_api_integration::AmplifierTaskReceiver,
}

impl relayer_engine::RelayerComponent for SolanaTxPusher {
    fn process(self: Box<Self>) -> Pin<Box<dyn Future<Output = eyre::Result<()>> + Send>> {
        use futures::FutureExt as _;

        self.process_internal().boxed()
    }
}

impl SolanaTxPusher {
    /// Create a new [`SolanaTxPusher`] component
    #[must_use]
    pub const fn new(
        config: config::Config,
        rpc_client: Arc<RpcClient>,
        task_receiver: relayer_amplifier_api_integration::AmplifierTaskReceiver,
    ) -> Self {
        Self {
            config,
            rpc_client,
            task_receiver,
        }
    }

    async fn process_internal(self) -> eyre::Result<()> {
        let config_metadata = self.get_config_metadata().await.map(Arc::new)?;

        let keypair = Arc::new(self.config.signing_keypair.insecure_clone());
        let mut join_set = JoinSet::<eyre::Result<()>>::new();
        let mut rx = self.task_receiver.receiver.fuse();
        let mut task_stream = futures::stream::poll_fn(move |cx| {
            // check if we have new requests to add to the join set
            match rx.poll_next_unpin(cx) {
                Poll::Ready(Some(command)) => {
                    // spawn the command on the joinset, returning the error
                    tracing::info!(?command, "received command from amplifier API");
                    join_set.spawn({
                        let solana_rpc_client = Arc::clone(&self.rpc_client);
                        let keypair = Arc::clone(&keypair);
                        let config_metadata = Arc::clone(&config_metadata);
                        async move {
                            process_task(&keypair, &solana_rpc_client, command, &config_metadata)
                                .await
                        }
                    });
                }
                Poll::Pending => (),
                Poll::Ready(None) => {
                    tracing::error!("receiver channel closed");
                    join_set.abort_all();
                }
            }
            // check if any background tasks are done
            match join_set.poll_join_next(cx) {
                Poll::Ready(Some(res)) => Poll::Ready(Some(res)),
                // join set returns `Poll::Ready(None)` when it's empty
                Poll::Ready(None) => {
                    if rx.is_terminated() {
                        return Poll::Ready(None)
                    }
                    Poll::Pending
                }
                Poll::Pending => Poll::Pending,
            }
        });

        while let Some(task_result) = task_stream.next().await {
            let Ok(res) = task_result else {
                tracing::error!(?task_result, "background task panicked");
                continue;
            };
            let Err(err) = res else {
                continue;
            };

            tracing::error!(?err, "background task returned an error");
        }

        eyre::bail!("fatal error")
    }

    async fn get_config_metadata(&self) -> Result<ConfigMetadata, eyre::Error> {
        let gateway_root_pda = gmp_gateway::get_gateway_root_config_pda().0;
        let data = self.rpc_client.get_account_data(&gateway_root_pda).await?;
        let root_config = gmp_gateway::state::GatewayConfig::unpack_from_slice(&data)?;
        let config_metadata = ConfigMetadata {
            gateway_root_pda,
            domain_separator: root_config.domain_separator,
        };
        Ok(config_metadata)
    }
}

struct ConfigMetadata {
    gateway_root_pda: Pubkey,
    domain_separator: [u8; 32],
}

#[instrument(skip_all)]
async fn process_task(
    keypair: &Keypair,
    solana_rpc_client: &RpcClient,
    task: TaskItem,
    metadata: &ConfigMetadata,
) -> eyre::Result<()> {
    use amplifier_api::types::Task::{Execute, GatewayTx, Refund, Verify};
    use axelar_rkyv_encoding::types::Payload::{Messages, VerifierSet};
    let signer = keypair.pubkey();
    let gateway_root_pda = gmp_gateway::get_gateway_root_config_pda().0;

    #[expect(
        clippy::todo,
        reason = "fine for the time being, will be refactored later"
    )]
    match task.task {
        Verify(_verify_task) => todo!(),
        GatewayTx(gateway_transaction_task) => {
            let execute_data_bytes = gateway_transaction_task.execute_data.as_ref();

            let decoded_execute_data =
                axelar_rkyv_encoding::types::ExecuteData::from_bytes(execute_data_bytes)
                    .map_err(|_err| eyre::eyre!("cannot decode execute data"))?;
            let signing_verifier_set = decoded_execute_data.proof.verifier_set();
            let (signing_verifier_set_pda, _) = gmp_gateway::get_verifier_set_tracker_pda(
                &gmp_gateway::id(),
                signing_verifier_set.hash(hasher_impl()),
            );

            match decoded_execute_data.payload {
                Messages(messages) => {
                    ProcessMessages::builder()
                        .messages(messages)
                        .signer(signer)
                        .gateway_root_pda(gateway_root_pda)
                        .metadata(metadata)
                        .execute_data_bytes(execute_data_bytes)
                        .signing_verifier_set_pda(signing_verifier_set_pda)
                        .solana_rpc_client(solana_rpc_client)
                        .keypair(keypair)
                        .build()
                        .execute()
                        .await?;
                }
                VerifierSet(new_verifier_set) => {
                    ProcessVerifierSet::builder()
                        .new_verifier_set(new_verifier_set)
                        .signer(signer)
                        .gateway_root_pda(gateway_root_pda)
                        .metadata(metadata)
                        .execute_data_bytes(execute_data_bytes)
                        .signing_verifier_set_pda(signing_verifier_set_pda)
                        .solana_rpc_client(solana_rpc_client)
                        .keypair(keypair)
                        .build()
                        .execute()
                        .await?;
                }
            }
        }
        Execute(_execute_task) => todo!(),
        Refund(_refund_task) => todo!(),
    };

    Ok(())
}

#[derive(typed_builder::TypedBuilder)]
struct ProcessMessages<'a> {
    messages: HasheableMessageVec,
    signer: Pubkey,
    gateway_root_pda: Pubkey,
    metadata: &'a ConfigMetadata,
    execute_data_bytes: &'a [u8],
    signing_verifier_set_pda: Pubkey,
    solana_rpc_client: &'a RpcClient,
    keypair: &'a Keypair,
}

impl<'a> ProcessMessages<'a> {
    #[tracing::instrument(skip_all)]
    async fn execute(&self) -> eyre::Result<()> {
        let execute_data_pda = InitializeApproveMessagesExecuteData::builder()
            .signer(self.signer)
            .gateway_root_pda(self.gateway_root_pda)
            .domain_separator(&self.metadata.domain_separator)
            .execute_data_bytes(self.execute_data_bytes)
            .solana_rpc_client(self.solana_rpc_client)
            .keypair(self.keypair)
            .build()
            .execute()
            .await?;

        // Compose messages
        let mut future_set = self
            .messages
            .iter()
            .filter_map(|message| {
                let command = OwnedCommand::ApproveMessage(message.clone());
                let (approved_message_pda, _bump, _seed) =
                    GatewayApprovedCommand::pda(&self.metadata.gateway_root_pda, &command);
                let ix = instructions::initialize_pending_command(
                    &self.metadata.gateway_root_pda,
                    &self.signer,
                    command,
                )
                .ok()?;

                let output = async move {
                    let send_transaction_result =
                        send_transaction(self.solana_rpc_client, self.keypair, ix).await;

                    let Err(err) = send_transaction_result else {
                        // tx was successfully executed
                        return Ok(execute_data_pda)
                    };

                    // tx was not executed -- inspect root cause
                    let ComputeBudgetError::SimulationError(ref simulation) = err else {
                        // some kid of irrecoverable error
                        return Err(eyre::Error::from(err))
                    };

                    if matches!(
                        simulation.err,
                        Some(TransactionError::InstructionError(
                            1, // <-- 0th idx is the ComputeBudget prefix
                            InstructionError::InvalidAccountData
                        ))
                    ) {
                        return eyre::Result::Ok(approved_message_pda);
                    }

                    // Return the simulation error
                    Err(eyre::Error::from(err))
                }
                .instrument(tracing::info_span!(
                    "registering command PDA",
                    ?approved_message_pda
                ));
                Some(output)
            })
            .collect::<FuturesUnordered<_>>();

        let mut command_accounts = Vec::new();
        while let Some(result) = future_set.next().await {
            let pubkey = result?;
            command_accounts.push(pubkey);
        }

        ApproveMessages::builder()
            .execute_data_pda(execute_data_pda)
            .gateway_root_pda(&self.metadata.gateway_root_pda)
            .command_accounts(&command_accounts)
            .signing_verifier_set_pda(self.signing_verifier_set_pda)
            .solana_rpc_client(self.solana_rpc_client)
            .keypair(self.keypair)
            .build()
            .execute()
            .await?;

        Ok(())
    }
}

#[derive(typed_builder::TypedBuilder)]
struct ProcessVerifierSet<'a> {
    new_verifier_set: VerifierSet,
    signer: Pubkey,
    gateway_root_pda: Pubkey,
    metadata: &'a ConfigMetadata,
    execute_data_bytes: &'a [u8],
    signing_verifier_set_pda: Pubkey,
    solana_rpc_client: &'a RpcClient,
    keypair: &'a Keypair,
}

impl<'a> ProcessVerifierSet<'a> {
    #[instrument(skip_all)]
    pub async fn execute(&self) -> eyre::Result<()> {
        let execute_data_pda = InitializeRotateSignersExecuteData::builder()
            .signer(self.signer)
            .gateway_root_pda(self.gateway_root_pda)
            .domain_separator(&self.metadata.domain_separator)
            .execute_data_bytes(self.execute_data_bytes)
            .solana_rpc_client(self.solana_rpc_client)
            .keypair(self.keypair)
            .build()
            .execute()
            .await?;

        let new_signing_verifier_set_pda =
            get_new_signing_verifier_set_pda(&self.new_verifier_set)?;

        RotateSigners::builder()
            .execute_data_pda(execute_data_pda)
            .gateway_root_pda(&self.metadata.gateway_root_pda)
            .signing_verifier_set_pda(self.signing_verifier_set_pda)
            .new_signing_verifier_set_pda(new_signing_verifier_set_pda)
            .signer(self.signer)
            .solana_rpc_client(self.solana_rpc_client)
            .keypair(self.keypair)
            .build()
            .execute()
            .await?;

        Ok(())
    }
}

#[derive(typed_builder::TypedBuilder)]
struct LogFinder<'a> {
    simulation: &'a RpcSimulateTransactionResult,
    log_to_search: &'a str,
}

impl<'a> LogFinder<'a> {
    const fn new(simulation: &'a RpcSimulateTransactionResult, log_to_search: &'a str) -> Self {
        Self {
            simulation,
            log_to_search,
        }
    }

    #[instrument(skip_all)]
    pub fn find(&self) -> bool {
        self.simulation
            .logs
            .as_ref()
            .is_some_and(|logs| logs.iter().any(|log| log.starts_with(self.log_to_search)))
    }
}

#[derive(typed_builder::TypedBuilder)]
struct InitializeApproveMessagesExecuteData<'a> {
    signer: Pubkey,
    gateway_root_pda: Pubkey,
    domain_separator: &'a [u8; 32],
    execute_data_bytes: &'a [u8],
    solana_rpc_client: &'a RpcClient,
    keypair: &'a Keypair,
}

impl<'a> InitializeApproveMessagesExecuteData<'a> {
    #[instrument(skip_all)]
    pub async fn execute(&self) -> eyre::Result<Pubkey> {
        let (ix, execute_data) = instructions::initialize_approve_messages_execute_data(
            self.signer,
            self.gateway_root_pda,
            self.domain_separator,
            self.execute_data_bytes,
        )?;
        let (execute_data_pda, ..) = gmp_gateway::get_execute_data_pda(
            &self.gateway_root_pda,
            &execute_data.hash_decoded_contents(),
        );

        let send_transaction_result =
            send_transaction(self.solana_rpc_client, self.keypair, ix).await;

        let Err(err) = send_transaction_result else {
            // tx was successfully executed
            return Ok(execute_data_pda)
        };

        // tx was not executed -- inspect root cause
        let ComputeBudgetError::SimulationError(ref simulation) = err else {
            // some kid of irrecoverable error
            return Err(eyre::Error::from(err))
        };

        // This happens if the PDA was already initialised
        if LogFinder::new(
            simulation,
            "Program log: Execute Datat PDA already initialized",
        )
        .find()
        {
            // Acceptable simulation error; proceed as successful
            return Ok(execute_data_pda)
        }

        // Return the simulation error
        Err(eyre::Error::from(err))
    }
}

#[derive(typed_builder::TypedBuilder)]
struct InitializeRotateSignersExecuteData<'a> {
    signer: Pubkey,
    gateway_root_pda: Pubkey,
    domain_separator: &'a [u8; 32],
    execute_data_bytes: &'a [u8],
    solana_rpc_client: &'a RpcClient,
    keypair: &'a Keypair,
}

impl<'a> InitializeRotateSignersExecuteData<'a> {
    #[instrument(skip_all)]
    pub async fn execute(&self) -> eyre::Result<Pubkey> {
        let (ix, execute_data) = instructions::initialize_rotate_signers_execute_data(
            self.signer,
            self.gateway_root_pda,
            self.domain_separator,
            self.execute_data_bytes,
        )?;
        let (execute_data_pda, ..) = gmp_gateway::get_execute_data_pda(
            &self.gateway_root_pda,
            &execute_data.hash_decoded_contents(),
        );
        tracing::info!(?execute_data_pda, "execute data PDA");

        let send_transaction_result =
            send_transaction(self.solana_rpc_client, self.keypair, ix).await;

        let Err(err) = send_transaction_result else {
            // tx was successfully executed
            return Ok(execute_data_pda)
        };

        // tx was not executed -- inspect root cause
        let ComputeBudgetError::SimulationError(ref simulation) = err else {
            // some kid of irrecoverable error
            return Err(eyre::Error::from(err))
        };

        // This happens if the PDA was already initialised
        if LogFinder::new(
            simulation,
            "Program log: Execute Datat PDA already initialized",
        )
        .find()
        {
            // Acceptable simulation error; proceed as successful
            return Ok(execute_data_pda)
        }

        // Return the simulation error
        Err(eyre::Error::from(err))
    }
}

#[derive(typed_builder::TypedBuilder)]
struct ApproveMessages<'a> {
    execute_data_pda: Pubkey,
    gateway_root_pda: &'a Pubkey,
    command_accounts: &'a [Pubkey],
    signing_verifier_set_pda: Pubkey,
    solana_rpc_client: &'a RpcClient,
    keypair: &'a Keypair,
}

impl<'a> ApproveMessages<'a> {
    #[instrument(skip_all)]
    pub async fn execute(&self) -> eyre::Result<()> {
        let ix = instructions::approve_messages(
            self.execute_data_pda,
            *self.gateway_root_pda,
            self.command_accounts,
            self.signing_verifier_set_pda,
        )?;

        let send_transaction_result =
            send_transaction(self.solana_rpc_client, self.keypair, ix).await;

        let Err(err) = send_transaction_result else {
            // tx was successfully executed
            return Ok(())
        };

        // tx was not executed -- inspect root cause
        let ComputeBudgetError::SimulationError(ref simulation) = err else {
            // some kid of irrecoverable error
            return Err(eyre::Error::from(err))
        };

        // This can happen if the verifier set is too old
        if LogFinder::new(simulation, "Program log: Proof validation failed").find() {
            // Acceptable simulation error; proceed as successful
            return Ok(())
        }

        // Return the simulation error
        Err(eyre::Error::from(err))
    }
}

#[derive(typed_builder::TypedBuilder)]
struct RotateSigners<'a> {
    execute_data_pda: Pubkey,
    gateway_root_pda: &'a Pubkey,
    signing_verifier_set_pda: Pubkey,
    new_signing_verifier_set_pda: Pubkey,
    signer: Pubkey,
    solana_rpc_client: &'a RpcClient,
    keypair: &'a Keypair,
}

impl<'a> RotateSigners<'a> {
    #[instrument(skip_all)]
    pub async fn execute(&self) -> eyre::Result<()> {
        let ix = instructions::rotate_signers(
            self.execute_data_pda,
            *self.gateway_root_pda,
            None,
            self.signing_verifier_set_pda,
            self.new_signing_verifier_set_pda,
            self.signer,
        )?;
        send_transaction(self.solana_rpc_client, self.keypair, ix).await?;
        Ok(())
    }
}

#[instrument(skip_all)]
fn get_new_signing_verifier_set_pda(new_verifier_set: &VerifierSet) -> eyre::Result<Pubkey> {
    let (new_signing_verifier_set_pda, _) = gmp_gateway::get_verifier_set_tracker_pda(
        &gmp_gateway::id(),
        new_verifier_set.hash(hasher_impl()),
    );
    Ok(new_signing_verifier_set_pda)
}

#[instrument(skip_all, ret)]
async fn send_transaction(
    solana_rpc_client: &RpcClient,
    keypair: &Keypair,
    ix: Instruction,
) -> Result<Signature, ComputeBudgetError> {
    effective_tx_sender::EffectiveTxSender::new(solana_rpc_client, keypair, VecDeque::from([ix]))
        .evaluate_compute_ixs()
        .await?
        .send_tx()
        .await
        .map_err(eyre::Error::from)
        .map_err(ComputeBudgetError::Generic)
}
