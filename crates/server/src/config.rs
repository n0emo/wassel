use config::ConfigError;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub host: String,
    pub port: String,
}

impl Config {
    pub fn load() -> Result<Self, ConfigError> {
        let config = config::Config::builder()
            .add_source(config::File::with_name("wassel").required(false))
            .set_default("host", "127.0.0.1")
            .unwrap()
            .set_default("port", "9000")
            .unwrap()
            .build()?;

        config.try_deserialize()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_owned(),
            port: "9000".to_owned(),
        }
    }
}
