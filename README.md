# Axelar Relayer core libraries
This repo provides building blocks for Axelar<>Blockchain Relayer

## Relayer Architecture Overview
```mermaid
graph TD
    subgraph "Relayer System"
        subgraph "Amplifier to Blockchain Flow"
            amp_sub[Amplifier Subscriber]
            tasks_queue[Amplifier Tasks Queue]
            bcx_ing[Blockchain Ingester]
        end

        subgraph "Blockchain to Amplifier Flow"
            bcx_sub[Blockchain Subscriber]
            events_queue[Amplifier Events Queue]
            amp_ing[Amplifier Ingester]
        end
    end
 
    %% External systems
    amp_api[/"Amplifier REST API"\]
    bcx["Blockchain"]

    %% Amplifier to Blockchain flow
    amp_api -->|amplifier tasks| amp_sub
    amp_sub -->|pushes amplifier tasks| tasks_queue
    tasks_queue -->|consumes amplifier tasks| bcx_ing
    bcx_ing -->|transforms & includes amplifier tasks| bcx
 
    %% Blockchain to Amplifier flow
    bcx -->|blockchain events| bcx_sub
    bcx_sub -->|transforms to amplifier events| events_queue
    events_queue -->|consumes amplifier events| amp_ing
    amp_ing -->|sends amplifier events| amp_api

    %% Styling
    classDef component fill:#d9d2e9,stroke:#674ea7,stroke-width:2px;
    classDef queue fill:#ffe6cc,stroke:#d79b00,stroke-width:2px;
    classDef api fill:#d5e8d4,stroke:#82b366,stroke-width:3px,color:#000,font-weight:bold,font-size:18px;
    classDef blockchain fill:#dae8fc,stroke:#6c8ebf,stroke-width:3px,color:#000,font-weight:bold,font-size:18px;

    class amp_sub,bcx_ing,bcx_sub,amp_ing component;
    class tasks_queue,events_queue queue;
    class amp_api api;
    class bcx blockchain;

    %% Styling
    classDef component stroke:#333,stroke-width:2px;
    classDef queue fill:#ff9,stroke:#333,stroke-width:2px;
    classDef external fill:#bbf,stroke:#333,stroke-width:1px;
 
    class amp_sub,bcx_ing,bcx_sub,amp_ing component;
    class tasks_queue,events_queue queue;
    class amp_api,bcx external;
```

## Bidirectional Flow Architecture

The relayer establishes bidirectional communication between an Amplifier API and a blockchain. It consists of these main flows:

### Amplifier to Blockchain Flow
- **Amplifier Subscriber**: Subscribes to the Amplifier REST API and receives amplifier tasks
- **Amplifier Tasks Queue**: Stores tasks before processing
- **Blockchain Ingester**: Consumes tasks from the queue, transforms them to a compatible format, and includes them in the blockchain

### Blockchain to Amplifier Flow
- **Blockchain Subscriber**: Subscribes to blockchain events 
- **Amplifier Events Queue**: Stores transformed events
- **Amplifier Ingester**: Consumes events from the queue and sends them to the Amplifier API

## Internal Relayer Architecture

The relayer uses threading and supervision model:

1. **Supervisor**:
   - Runs on its own dedicated thread with a Tokio runtime
   - Spawns and monitors worker threads only for the components selected via CLI
   - Detects crashes and automatically restarts failed components
   - Provides a graceful shutdown period when termination is requested

2. **Termination Handling**:
   - A dedicated thread listens for Ctrl+C signals
   - Uses an AtomicBool as a shared termination flag
   - When Ctrl+C is received, the AtomicBool is set to true
   - All components, including the supervisor, watch this AtomicBool
   - Upon termination, components have graceful period to finish current work

3. **Worker Components**:
   - Each component (Amplifier Subscriber, Blockchain Ingester, Blockchain Subscriber, Amplifier Ingester) runs on its own thread
   - Each thread has an isolated Tokio runtime
   - Components check the termination flag regularly and shut down when needed
   - Isolation ensures a failure in one component doesn't affect others
   - It's up to the implementation to run all components at once or as separate binaries but isolation is mandatory

4. **Queue Abstraction**:
   - All access to queues is abstracted via Rust traits
   - Currently implemented using [NATS](https://nats.io/) open-source messaging system
   - Abstraction allows for easy replacement with different queue technologies
   - Components interact with queues only through trait interfaces, maintaining loose coupling
   - Supports horizontal scaling by allowing multiple instances to consume from the same queue

## WorkerFn Array
Since `WorkerFn` is pointer to async function pushing data to hashmap is queite noisy.

check [Example](crates/amplifier-components/examples/components_array.rs)
