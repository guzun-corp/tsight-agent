use clickhouse;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Serialize, PartialEq, Clone)]
pub enum DataSourceType {
    Clickhouse,
    PostgreSQL,
    MySQL,
    Prometheus,
}

impl std::fmt::Display for DataSourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataSourceType::Clickhouse => write!(f, "clickhouse"),
            DataSourceType::PostgreSQL => write!(f, "postgresql"),
            DataSourceType::MySQL => write!(f, "mysql"),
            DataSourceType::Prometheus => write!(f, "prometheus"),
        }
    }
}

impl<'de> Deserialize<'de> for DataSourceType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.to_lowercase().as_str() {
            "clickhouse" => Ok(DataSourceType::Clickhouse),
            "postgresql" => Ok(DataSourceType::PostgreSQL),
            "mysql" => Ok(DataSourceType::MySQL),
            "prometheus" => Ok(DataSourceType::Prometheus),
            _ => Err(serde::de::Error::custom(format!(
                "unknown datasource type: {}",
                s
            ))),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DataSource {
    pub name: String,
    pub source_type: DataSourceType,
    pub hosts: Vec<String>,
    pub username: String,
    pub password: String,
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    pub filters: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueryRequest {
    pub id: String,
    pub datasource_name: String,
    pub query: String,
    pub timeout: u64,
}

fn default_timeout() -> u64 {
    60
}

pub struct QueryResult {
    pub request_id: String,
    pub data: serde_json::Value,
    pub error: Option<String>,
}

#[derive(clickhouse::Row, Deserialize, Debug, Serialize)]
pub struct Record {
    pub t: u32,
    pub cnt: f64,
}

// Commented out as it's currently unused
// fn deserialize_lossy_string<'de, D>(deserializer: D) -> Result<String, D::Error>
// where
//     D: serde::Deserializer<'de>,
// {
//     let bytes: Vec<u8> = Deserialize::deserialize(deserializer)?;
//     Ok(String::from_utf8_lossy(&bytes).into_owned())
// }

/// A dynamic row is just a map from column names to values.
pub type JobType = HashMap<String, Value>;

/// A dynamic row wrapper. We use `#[serde(flatten)]` to capture all columns.
#[derive(Debug, Serialize, Deserialize, clickhouse::Row)]
pub struct DynamicRow {
    #[serde(flatten)]
    pub values: JobType,
}
