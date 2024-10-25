# 4. solana tx sending

Date: 2024-10-25

## Status

Accepted

## Context

Our relayer system needs an efficient and reliable way to send transactions to the Solana blockchain based on tasks received from the Amplifier API. These tasks include various operations such as verifying data, executing gateway transactions, processing messages, rotating verifier sets, and handling refunds.

Challenges influencing this decision:

- Compute Budget Constraints: Solana imposes a compute budget limit per transaction, which can vary based on network conditions.
- Dynamic Compute Unit Pricing: The cost of compute units can fluctuate, affecting transaction prioritization and inclusion.
- Asynchronous Task Processing: The system must handle multiple tasks concurrently without blocking, ensuring scalability and responsiveness.
- Error Handling and Recovery: Transactions may fail due to simulation errors or network issues, requiring robust error handling mechanisms.

## Decision

We implemented a new `SolanaTxPusher` component within the relayer system to handle the submission of transactions to the Solana blockchain. Key decisions and features include:

- Asynchronous Processing: The `SolanaTxPusher` uses a JoinSet to process incoming tasks concurrently, allowing the system to handle multiple tasks without blocking.
- Task Handling: It supports different task types from the Amplifier API, specifically focusing on GatewayTx tasks, which involve processing messages and updating verifier sets.
- Dynamic Compute Budget Adjustment:
  - Introduced an `EffectiveTxSender` utility that simulates transactions to estimate the required compute units.
  - Adjusts the compute budget and compute unit price based on simulation results and recent network fees to maximize the chances of transaction inclusion.
- Error Handling:
  - Implements detailed error inspection after simulation to handle specific errors like insufficient compute units or invalid account data.
  - Uses retries or alternative flows when acceptable simulation errors occur (e.g., already initialized PDAs).

## Consequences

Benefits:

- Increased Transaction Success Rate: By simulating and adjusting compute budgets and unit prices, the system increases the likelihood of transactions being accepted by the Solana network.
- Scalability: Asynchronous task processing allows the system to scale and handle higher loads without significant performance degradation.
- Optimized Resource Usage: Dynamic adjustment of compute budgets helps in efficient utilization of resources, avoiding over-allocation or under-utilization.

Drawbacks:

- Increased Complexity: The introduction of transaction simulation and dynamic adjustments adds complexity to the system, which may lead to longer development and debugging times.
- Potential Latency: Additional steps like simulation and fee analysis may introduce latency in transaction processing. More HTTP round-trips.
- Error Handling Complexity: Robust error handling necessitates comprehensive testing to ensure all edge cases are covered, which can be resource-intensive.

Risks and Mitigation:

- Simulation Failures: If simulations fail or provide inaccurate estimates, transactions may fail. To mitigate this, the system includes error inspection and alternative handling paths.
- Security Concerns: Handling keypairs and signing transactions introduces security risks. Proper key management and secure coding practices are essential to mitigate potential vulnerabilities.
