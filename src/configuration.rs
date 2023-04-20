use dotenv::dotenv;
use serde::Deserialize;
use serde_aux::field_attributes::deserialize_number_from_string;

#[derive(Deserialize, Clone, Debug)]
pub struct Configuration {
    pub host: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
}

pub fn get_configuration() -> Result<Configuration, config::ConfigError> {
    dotenv().ok();

    let configuration = config::Config::builder()
        .add_source(config::Environment::default())
        .build()?;

    configuration.try_deserialize()
}
