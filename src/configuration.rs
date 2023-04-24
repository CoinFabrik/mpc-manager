//! # Configuration and settings
//!
//! This module is used to retrieve the configuration from the environment variables
//! and parse them into a struct.

use dotenv::dotenv;
use serde::Deserialize;
use serde_aux::field_attributes::deserialize_number_from_string;

/// Configuration settings for the server.
#[derive(Deserialize, Clone, Debug)]
pub struct Configuration {
    /// Host used by the server.
    pub host: String,
    /// Port used to expose the server.
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
}

/// Returns a configuration object from the environment variables.
pub fn get_configuration() -> Result<Configuration, config::ConfigError> {
    dotenv().ok();

    let configuration = config::Config::builder()
        .add_source(config::Environment::default())
        .build()?;

    configuration.try_deserialize()
}
