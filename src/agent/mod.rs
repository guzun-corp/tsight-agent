mod base;
mod datasource;

use anyhow::{anyhow, Result};
use log::{error, info, warn};
use std::time::Duration;

use crate::client::ServerClient;
use crate::config::Config;
use crate::config::GlobalFilters;
use crate::models::DataSource;
use base::BaseAgent;
pub use datasource::discover_and_submit_schemas;

/// Enum that holds different types of agents
#[derive(Clone)]
pub enum Agent {
    Observation(ObservationAgent),
    Job(JobAgent),
}

/// Initialize all agents based on the provided configuration
pub fn initialize_agents(config: &Config) -> (Agent, Agent, Agent) {
    // Create high priority queue agent
    let hp_agent = factory::create_observation_agent(
        config.server.api_key.clone(),
        config.server.server_url.clone(),
        config.datasources.clone(),
        true,
        config.global_filters.clone(),
    );
    info!("Initialized high priority agent");

    // Create job processing agent
    let job_agent = factory::create_job_agent(
        config.server.api_key.clone(),
        config.server.server_url.clone(),
        config.datasources.clone(),
        config.global_filters.clone(),
    );
    info!("Initialized job agent");

    // Create main agent for observations
    let main_agent = factory::create_observation_agent(
        config.server.api_key.clone(),
        config.server.server_url.clone(),
        config.datasources.clone(),
        false,
        config.global_filters.clone(),
    );
    info!("Initialized observations agent");

    (hp_agent, job_agent, main_agent)
}

/// Observation agent for processing time series queries
#[derive(Clone)]
pub struct ObservationAgent {
    pub(crate) base: BaseAgent,
    pub is_high_priority_queue: bool,
}

impl ObservationAgent {
    /// Process the next task from the server
    pub async fn process_next(&self) -> Result<()> {
        let no_task_error_message;
        if self.is_high_priority_queue {
            no_task_error_message = "Failed to acquire next high priority query from server:";
        } else {
            no_task_error_message = "Failed to acquire next query from server:";
        }

        let query_request = self
            .base
            .server_client
            .acquire_next_query(self.is_high_priority_queue)
            .await
            .map_err(|e| anyhow!("{} {}", no_task_error_message, e))?;

        let result = self.base.process_query(&query_request).await;

        match result {
            Ok(data) => {
                self.base
                    .server_client
                    .submit_results(&query_request.id, data, self.is_high_priority_queue)
                    .await?;

                info!(
                    "Successfully submitted results for query {}",
                    query_request.id
                );
            }
            Err(e) => {
                let error_msg = e.to_string();
                match self
                    .base
                    .server_client
                    .submit_error(&query_request.id, &error_msg, self.is_high_priority_queue)
                    .await
                {
                    Ok(_) => (),
                    Err(submit_err) => {
                        // Log the submission error but return the original error
                        warn!("Failed to submit error: {}", submit_err);
                    }
                }
                return Err(e);
            }
        }

        Ok(())
    }
}

/// Job agent for processing job queries
#[derive(Clone)]
pub struct JobAgent {
    pub(crate) base: BaseAgent,
}

impl JobAgent {
    /// Create a new job agent
    pub fn with_filters(
        server_client: ServerClient,
        datasources: Vec<DataSource>,
        global_filters: Option<GlobalFilters>,
    ) -> Self {
        Self {
            base: BaseAgent::with_filters(server_client, datasources, global_filters),
        }
    }

    /// Process the next job from the server
    pub async fn process_next(&self) -> Result<()> {
        let query_request = self
            .base
            .server_client
            .acquire_next_job()
            .await
            .map_err(|e| anyhow!("Failed to acquire next job from server: {}", e))?;

        let result = self.base.process_job(&query_request).await;

        match result {
            Ok(data) => {
                self.base
                    .server_client
                    .submit_job_results(&query_request.id, data)
                    .await?;

                info!(
                    "Successfully submitted results for job {}",
                    query_request.id
                );
            }
            Err(e) => {
                let error_msg = e.to_string();
                match self
                    .base
                    .server_client
                    .submit_job_error(&query_request.id, &error_msg)
                    .await
                {
                    Ok(_) => (),
                    Err(submit_err) => {
                        // Log the submission error but return the original error
                        warn!("Failed to submit error: {}", submit_err);
                    }
                }
                return Err(e);
            }
        }

        Ok(())
    }
}

impl Agent {
    /// Get a reference to the agent's server client
    pub fn server_client(&self) -> &ServerClient {
        match self {
            Agent::Observation(agent) => &agent.base.server_client,
            Agent::Job(agent) => &agent.base.server_client,
        }
    }

    /// Get a reference to the agent's datasources
    pub fn datasources(&self) -> &[DataSource] {
        match self {
            Agent::Observation(agent) => &agent.base.datasources,
            Agent::Job(agent) => &agent.base.datasources,
        }
    }

    /// Process the next task from the server
    pub async fn process_next(&self) -> Result<()> {
        match self {
            Agent::Observation(agent) => agent.process_next().await,
            Agent::Job(agent) => agent.process_next().await,
        }
    }

    /// Run the agent in a continuous loop
    pub async fn run(&self) {
        loop {
            match self.process_next().await {
                Ok(_) => (),
                Err(e) => {
                    if e.to_string().contains("No tasks available")
                        || e.to_string().contains("No jobs available")
                    {
                        warn!("{}", e);
                    } else {
                        error!("Failed to process task: {:#}", e);
                    }
                }
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
}

/// Factory functions for creating agents
pub mod factory {
    use super::*;

    /// Create a new observation agent
    pub fn create_observation_agent(
        api_key: String,
        server_url: String,
        datasources: Vec<DataSource>,
        is_high_priority_queue: bool,
        global_filters: Option<GlobalFilters>,
    ) -> Agent {
        let server_client = ServerClient::new(api_key, server_url);
        Agent::Observation(ObservationAgent {
            base: BaseAgent::with_filters(server_client, datasources, global_filters),
            is_high_priority_queue,
        })
    }

    /// Create a new job agent
    pub fn create_job_agent(
        api_key: String,
        server_url: String,
        datasources: Vec<DataSource>,
        global_filters: Option<GlobalFilters>,
    ) -> Agent {
        let server_client = ServerClient::new(api_key, server_url);
        Agent::Job(JobAgent {
            base: BaseAgent::with_filters(server_client, datasources, global_filters),
        })
    }
}

/// Types of agents that can be created
pub enum AgentType {
    Observation,
    Job,
}
