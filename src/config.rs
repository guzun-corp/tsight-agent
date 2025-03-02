use crate::models::DataSource;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct ServerConfig {
    pub api_key: String,
    pub server_url: String,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct SqlFilterRules {
    pub database_regexes: Option<Vec<String>>,
    pub table_regexes: Option<Vec<String>>,
    pub column_name_regexes: Option<Vec<String>>,
    pub column_value_regexes: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct GlobalFilters {
    pub sql_filters_exclude: Option<Vec<SqlFilterRules>>,
    pub sql_filters_allow: Option<Vec<SqlFilterRules>>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub datasources: Vec<DataSource>,
    pub global_filters: Option<GlobalFilters>,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self, config::ConfigError> {
        let settings = config::Config::builder()
            .add_source(config::File::from(path))
            .build()
            .map_err(|e| {
                config::ConfigError::NotFound(format!(
                    "Failed to load config file at '{}': {}",
                    path.display(),
                    e
                ))
            })?;

        settings.try_deserialize().map_err(|e| {
            config::ConfigError::Message(format!(
                "Failed to parse config file at '{}': {}",
                path.display(),
                e
            ))
        })
    }
}
