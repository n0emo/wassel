use std::collections::HashMap;

use serde::Deserialize;
use tracing::error;

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub host: String,
    pub port: String,
    pub plugins_directory: String,

    #[allow(unused)]
    pub plugins: HashMap<String, HashMap<String, String>>,
}

impl Config {
    pub fn load() -> Self {
        let Ok(config) = config::Config::builder()
            .add_source(config::File::with_name("config").required(false))
            .set_default("host", "127.0.0.1")
            .unwrap()
            .set_default("port", "9150")
            .unwrap()
            .set_default("plugins_directory", "plugins")
            .unwrap()
            .set_default("plugins", HashMap::<String, HashMap<String, String>>::new())
            .unwrap()
            .build()
        else {
            return Self::default();
        };

        match config.try_deserialize() {
            Ok(c) => c,
            Err(e) => {
                error!("Error reading config: {e}");
                Self::default()
            }
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_owned(),
            port: "9150".to_owned(),
            plugins_directory: "plugins".to_owned(),
            plugins: HashMap::new(),
        }
    }
}
