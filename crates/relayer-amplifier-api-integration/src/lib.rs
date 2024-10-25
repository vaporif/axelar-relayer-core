//! The component that is responsible for communicating with the Axelar Amplifier API

mod component;
mod config;
mod healthcheck;
mod listener;
mod subscriber;

pub use amplifier_api;
pub use component::{Amplifier, AmplifierCommand, AmplifierCommandClient, AmplifierTaskReceiver};
pub use config::Config;
