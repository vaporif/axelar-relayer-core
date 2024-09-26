# 2. configuration

Date: 2024-09-25

## Status

Accepted

## Context

The relayer needs to be initialised with settings that allow it to:

- read data from the Solana blockchain (get signtatures and tx data)
- write data to the Solana blockchain (send txs to the Gateway and destination programs).
  This requires a private key to be present for signing purposes.
- read data from the Amplifier API
- write data to the Amplifier API

To optimize for compile times and make testing easier, we're splitting the code into:

- library (`relayer-engine`) that will be responsible for the business logic. It defines the public config interfacte.
- binary (`relayer`) that will be responsible for telemetry, tracing, parsing the config -- all the pipework for setting up the engine.

## Decision

The relayer will read the data form a config TOML file. Sensitive data, like Solana private key,
can be injected via env variables, which will become a part of the configuration.

## Consequences

Pros:

- Configuration is strongly structured, and always easy to reference.
- Dynamically overriding variables like the pirvate key via env variables is simpler (that having to upload a whole file)
  when configuring the servica via k8s or other cloud infra.

Cons:

- Having the ability to dynamically set config overrides via env variables may convolute
  the setup process. Especially when an env variable is not any more secure than a local config file.
