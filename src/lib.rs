//! # MPC Manager
//!
//! This library is part of the [mpc-framework](https://github.com/mpc-framework).
//! It has been created to work as a coordinator between different parties in a
//! multi-party-computation.
//!
//! This library provides a binary which can be downloaded from [here](https://github.com/mpc-framework/mpc-manager/releases) or can be built from the latest available code from the github repository.
//!
//! # Implementation notes
//!
//! * Mpc-manager is built around using websockets with [json-rpc](https://www.jsonrpc.org/specification) as a messaging protocol.
//! * Although it includes logging by default, it can be easily disabled.
//! * It was built with security in mind: no data is stored long-term and as soon as it's not needed anymore it's deleted.

#[cfg(feature = "server")]
pub mod configuration;

#[cfg(feature = "server")]
pub mod server;
#[cfg(feature = "server")]
pub mod telemetry;

pub mod service;
pub mod state;
