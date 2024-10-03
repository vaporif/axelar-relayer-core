//! The component that is responsible for communicating with the Axelar Amplifier API

mod component;
mod config;
mod healthcheck;
mod listener;
mod subscriber;

pub use component::{Amplifier, AmplifierClient};
pub use config::Config;
