use anyhow::{anyhow, Result};
use log::debug;

use crate::client::{AcquireResultBody, ServerClient};
use crate::config::GlobalFilters;
use crate::models::{DataSource, JobType, Record};

use crate::executors::create_executor;

/// Base agent implementation with common functionality
#[derive(Clone)]
pub struct BaseAgent {
    pub server_client: ServerClient,
    pub datasources: Vec<DataSource>,
    pub global_filters: Option<GlobalFilters>,
}

impl BaseAgent {
    /// Create a new base agent with global filters
    pub fn with_filters(
        server_client: ServerClient,
        datasources: Vec<DataSource>,
        global_filters: Option<GlobalFilters>,
    ) -> Self {
        Self {
            server_client,
            datasources,
            global_filters,
        }
    }

    /// Find a datasource by name
    fn find_datasource(&self, query_request: &AcquireResultBody) -> Option<&DataSource> {
        self.datasources
            .iter()
            .find(|ds: &&DataSource| ds.name == query_request.datasource_name)
    }

    /// Process a query and return the results
    pub async fn process_query(&self, query_request: &AcquireResultBody) -> Result<Vec<Record>> {
        let datasource = self.find_datasource(query_request).ok_or_else(|| {
            anyhow!(
                "No matching datasource found for query {}",
                query_request.datasource_name
            )
        })?;

        let executor = create_executor(datasource, self.global_filters.clone()).await?;

        let data = executor
            .execute_ts(&query_request.query)
            .await
            .map_err(|e| anyhow!("Query execution error for query: {}", e))?;

        Ok(data)
    }

    /// Process a job and return the results
    pub async fn process_job(&self, query_request: &AcquireResultBody) -> Result<Vec<JobType>> {
        let datasource = self.find_datasource(query_request).ok_or_else(|| {
            anyhow!(
                "No matching datasource found for query {}",
                query_request.datasource_name
            )
        })?;

        let executor = create_executor(datasource, self.global_filters.clone()).await?;

        let data = executor
            .execute_job(&query_request.query)
            .await
            .map_err(|e| anyhow!("Query execution error for query: {}", e))?;

        debug!("Job results: {:?}", &data);

        Ok(data)
    }
}
