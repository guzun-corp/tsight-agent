pub mod base;
pub mod clickhouse_source;
use crate::config::GlobalFilters;
use crate::executors::{base::QueryExecutor, clickhouse_source::ClickhouseExecutor};
use crate::models::{DataSource, DataSourceType};
use anyhow::{anyhow, Result};

/// Create an appropriate executor based on the datasource type
pub async fn create_executor(
    datasource: &DataSource,
    global_filters: Option<GlobalFilters>,
) -> Result<Box<dyn QueryExecutor>> {
    let host: &String = datasource
        .hosts
        .first()
        .ok_or_else(|| anyhow!("No host specified for Clickhouse datasource"))?;

    match datasource.source_type {
        DataSourceType::Clickhouse => Ok(Box::new(ClickhouseExecutor::with_global_filters(
            host,
            &datasource.username,
            &datasource.password,
            global_filters,
        )?)),
        DataSourceType::PostgreSQL => Err(anyhow!("PostgreSQL executor not implemented")),
        DataSourceType::MySQL => Err(anyhow!("MySQL executor not implemented")),
        DataSourceType::Prometheus => Err(anyhow!("Prometheus executor not implemented")),
    }
}
