use anyhow::Result;
use async_trait::async_trait;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum QueryError {
    #[error("Connection error: {0}")]
    ConnectionError(String),
    #[error("Query execution error: {0}")]
    ExecutionError(String),
}

#[async_trait]
pub trait QueryExecutor: Send + Sync {
    async fn execute_ts(&self, query: &str) -> Result<Vec<crate::models::Record>, QueryError>;
    async fn execute_job(&self, query: &str) -> Result<Vec<crate::models::JobType>, QueryError>;
    async fn connect(&mut self) -> Result<(), QueryError>;
    async fn discover_schemas(
        &self,
    ) -> Result<Vec<crate::executors::clickhouse_source::TableSchema>, QueryError>;
    fn filter_job_results(&self, rows: Vec<crate::models::JobType>) -> Vec<crate::models::JobType>;
}
