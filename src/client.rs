//! Client module for interacting with the server API
//!
//! This module provides a client for communicating with the server API,
//! handling tasks, jobs, schema discovery, and datasource management.

use crate::models::JobType;
use anyhow::{anyhow, Context, Result};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::time::Duration;

// Request/Response types
mod types {
    use super::*;
    use crate::executors::clickhouse_source::TableSchema;
    use crate::models::{JobType, Record};

    /// Request to acquire a task from the queue
    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct AcquireRequest {
        pub is_high_priority_queue: bool,
    }

    /// Response when acquiring a task or job
    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct AcquireResultBody {
        pub id: String,
        pub datasource_name: String,
        pub query: String,
    }

    /// Request to submit task results
    #[derive(Debug, Serialize, Deserialize)]
    pub struct SubmitTaskRequest {
        pub records: Vec<Record>,
        pub is_high_priority_queue: bool,
    }

    /// Request to submit job results
    #[derive(Debug, Serialize, Deserialize)]
    pub struct SubmitJobRequest {
        pub records: Vec<JobType>,
    }

    /// Request to submit an error
    #[derive(Debug, Serialize)]
    pub struct ErrorSubmissionRequest {
        pub error: String,
        pub is_high_priority_queue: bool,
    }

    /// Request to submit schema information
    #[derive(Debug, Serialize)]
    pub struct SchemaSubmissionRequest {
        pub schemas: Vec<TableSchema>,
    }

    /// Request to create or update a datasource
    #[derive(Debug, Serialize)]
    pub struct DatasourceUpsertRequest {
        pub datasource_type: String,
    }
}

use types::*;

/// Client for interacting with the server API
#[derive(Clone)]
pub struct ServerClient {
    api_key: String,
    server_url: String,
    client: Client,
}

// Re-export types that are used by other modules
pub use types::AcquireResultBody;

impl ServerClient {
    /// Create a new server client
    pub fn new(api_key: String, server_url: String) -> Self {
        Self {
            api_key,
            server_url,
            client: Client::new(),
        }
    }

    /// Get authorization header for API requests
    fn auth_header(&self) -> String {
        format!("Bearer {}", self.api_key)
    }

    /// Handle common response error cases
    async fn handle_response_errors<T>(
        &self,
        response: reqwest::Response,
        not_found_msg: String,
        error_context: String,
    ) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        if response.status() == StatusCode::NOT_FOUND {
            return Err(anyhow!(not_found_msg));
        } else if !response.status().is_success() {
            return Err(anyhow!("{}: {}", error_context, response.status()));
        }

        response.json::<T>().await.context(error_context)
    }

    // Task-related methods

    /// Acquire the next task from the queue
    pub async fn acquire_next_query(
        &self,
        is_high_priority_queue: bool,
    ) -> Result<AcquireResultBody> {
        let response = self
            .client
            .post(format!("{}/tasks/acquire", self.server_url))
            .header("Authorization", self.auth_header())
            .json(&AcquireRequest {
                is_high_priority_queue,
            })
            .timeout(Duration::from_secs(60))
            .send()
            .await
            .context("Failed to send acquire task request")?;

        self.handle_response_errors(
            response,
            "No tasks available".to_string(),
            "Failed to acquire task".to_string(),
        )
        .await
    }

    /// Submit task results to the server
    pub async fn submit_results(
        &self,
        task_id: &str,
        data: Vec<crate::models::Record>,
        is_high_priority_queue: bool,
    ) -> Result<()> {
        let response = self
            .client
            .post(format!("{}/tasks/{}/submit", self.server_url, task_id))
            .header("Authorization", self.auth_header())
            .json(&SubmitTaskRequest {
                records: data,
                is_high_priority_queue,
            })
            .send()
            .await
            .context("Failed to send submit results request")?;

        if !response.status().is_success() {
            return Err(anyhow!("Failed to submit results: {}", response.status()));
        }

        Ok(())
    }

    /// Submit an error for a task
    pub async fn submit_error(
        &self,
        task_id: &str,
        error: &str,
        is_high_priority_queue: bool,
    ) -> Result<()> {
        let response = self
            .client
            .post(format!("{}/tasks/{}/submit", self.server_url, task_id))
            .header("Authorization", self.auth_header())
            .json(&ErrorSubmissionRequest {
                error: error.to_string(),
                is_high_priority_queue,
            })
            .send()
            .await
            .context("Failed to send submit error request")?;

        if !response.status().is_success() {
            return Err(anyhow!("Failed to submit error: {}", response.status()));
        }

        Ok(())
    }

    // Job-related methods

    /// Acquire the next job from the queue
    pub async fn acquire_next_job(&self) -> Result<AcquireResultBody> {
        let response = self
            .client
            .post(format!("{}/jobs/acquire", self.server_url))
            .header("Authorization", self.auth_header())
            .timeout(Duration::from_secs(60))
            .send()
            .await
            .context("Failed to send acquire job request")?;

        self.handle_response_errors(
            response,
            "No jobs available".to_string(),
            "Failed to acquire job".to_string(),
        )
        .await
    }

    /// Submit job results to the server
    pub async fn submit_job_results(&self, job_id: &str, data: Vec<JobType>) -> Result<()> {
        let response = self
            .client
            .post(format!("{}/jobs/{}/submit", self.server_url, job_id))
            .header("Authorization", self.auth_header())
            .json(&SubmitJobRequest { records: data })
            .send()
            .await
            .context("Failed to send submit job results request")?;

        log::debug!("submit_job_results, response: {:?}", &response);

        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to submit job results: {}",
                response.status()
            ));
        }

        Ok(())
    }

    /// Submit an error for a job
    pub async fn submit_job_error(&self, job_id: &str, error: &str) -> Result<()> {
        let response = self
            .client
            .post(format!("{}/jobs/{}/submit", self.server_url, job_id))
            .header("Authorization", self.auth_header())
            .json(&ErrorSubmissionRequest {
                error: error.to_string(),
                is_high_priority_queue: false,
            })
            .send()
            .await
            .context("Failed to send submit job error request")?;

        if !response.status().is_success() {
            return Err(anyhow!("Failed to submit error: {}", response.status()));
        }

        Ok(())
    }

    // Schema and datasource management methods

    /// Submit schema information for a datasource
    pub async fn submit_schemas(
        &self,
        datasource_name: &str,
        schemas: Vec<crate::executors::clickhouse_source::TableSchema>,
    ) -> Result<()> {
        log::debug!("Submitting schemas: {:?}", &schemas);
        let response = self
            .client
            .post(format!(
                "{}/datasource/{}/discovery",
                self.server_url, datasource_name
            ))
            .header("Authorization", self.auth_header())
            .json(&SchemaSubmissionRequest { schemas })
            .send()
            .await
            .context("Failed to send submit schemas request")?;

        if !response.status().is_success() {
            return Err(anyhow!("Failed to submit schemas: {}", response.status()));
        }

        Ok(())
    }

    /// Add or update a datasource
    pub async fn add_datasource(&self, datasource_name: &str, datasource_type: &str) -> Result<()> {
        log::info!("Add datasource: {:?}", &datasource_name);
        let response = self
            .client
            .post(format!(
                "{}/datasource/{}/add",
                self.server_url, datasource_name
            ))
            .header("Authorization", self.auth_header())
            .json(&DatasourceUpsertRequest {
                datasource_type: datasource_type.to_string(),
            })
            .send()
            .await
            .context("Failed to send add datasource request")?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to update existed or create a new datasource: {}",
                response.status()
            ));
        }

        Ok(())
    }
}
