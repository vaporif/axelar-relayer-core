# 3. Solana Event Parsing

Date: 2024-10-17

## Status

Accepted

## Context

We need to fetch logs from the Solana blockchain, examine them, compose them into an Amplifier API event, and forward it to the Axelar blockchain. On Solana, logs are attached to `signatures`, which act as unique transaction identifiers. We are interested in signatures that involve the Gateway program ID.

Solana's RPC offers methods for interacting with the blockchain:

- The [`getSignaturesForAddress`](https://solana.com/docs/rpc/http/getsignaturesforaddress) method allows fetching signatures in a range. It can be used to poll the node and query historical signatures in batches. However, this method does not return the logs associated with the signatures.
- The [`getTransaction`](https://solana.com/docs/rpc/http/gettransaction) method can be used to fetch the transaction details, including logs, for a given signature. This means that we need at least two RPC calls for every signature: one to fetch the signatures and another to fetch the transaction logs.

Additionally, Solana offers a WebSocket API:

- The [`logsSubscribe`](https://solana.com/docs/rpc/websocket/logssubscribe) method allows subscribing to live log updates for specified accounts or programs. This method provides real-time logs but does not provide historical data.

In order to efficiently process both historical and real-time logs, we need a strategy that combines both the HTTP RPC and WebSocket APIs, minimizing the number of RPC calls while ensuring no data is missed.

## Decision

We have designed the Solana Listener component to process logs in a multi-step execution flow, incorporating strategies for catching up missed signatures:

1. **Fetching Historical Signatures**

   - Upon component startup, using the provided configuration, we determine the catch-up strategy for missed signatures. The strategies include:
     - **None:** Start from the latest available signature without fetching historical data.
     - **UntilSignatureReached:** Fetch signatures until a specified target signature is reached.
     - **UntilBeginning:** Fetch all missed signatures starting from the beginning.
   - We fetch signatures using the HTTP RPC endpoint `getSignaturesForAddress` in batches.
   - For each signature, we fetch the corresponding transaction logs using `getTransaction`.
   - We keep track of the latest signature we have processed to avoid duplicate processing.

2. **Establishing a WebSocket Connection**

   - We establish a new WebSocket connection to the Solana node using `logsSubscribe`, subscribing to logs involving the Gateway program ID.
   - We await the first message from the WebSocket stream.

3. **Fetching Signatures Missed During WebSocket Connection Establishment**

   - Establishing the WebSocket connection can take a few seconds, during which we might miss some signatures.
   - To handle this, we use the HTTP RPC methods to fetch any signatures that might have been missed between the last processed signature and the first signature received via WebSocket.
   - This ensures continuity in the event stream.

4. **Ingesting New Signatures Using WebSocket Data**

   - After handling historical and potentially missed signatures, we process new incoming logs from the WebSocket connection in real-time.
   - We monitor the WebSocket connection and handle reconnections if necessary to maintain real-time data ingestion.

## Consequences

Pros:

- **Improved Reliability:** By implementing configurable catch-up strategies and combining HTTP RPC and WebSocket APIs, we ensure that no events are missed, even during startup or reconnection periods.
- **Flexibility:** The ability to configure the catch-up strategy allows us to balance between startup time and data completeness based on specific needs.

Cons:

- **Complexity:** The implementation involves coordinating between HTTP RPC calls and WebSocket subscriptions, handling potential overlaps. This can be hard to grasp and may seem as unnecessary complexity.
- **Resource Utilization:** Fetching historical data may increase the number of RPC calls, to evade rate limitations, the RPC uses a rate-limiting semaphore and internal sleeps.
