//! The component that is responsible for communicating with the Axelar Amplifier API

mod component;
mod config;
mod from_amplifier;
mod healthcheck;
mod to_amplifier;

pub use amplifier_api;
pub use component::{Amplifier, AmplifierCommand, AmplifierCommandClient, AmplifierTaskReceiver};
pub use config::Config;
