use mockito::ServerGuard;

// Import directly from the crate
use tsight_agent::agent::{initialize_agents, Agent};
use tsight_agent::config::{Config, ServerConfig};
use tsight_agent::models::{DataSource, DataSourceType};

// Setup test server
async fn setup_test_server() -> ServerGuard {
    mockito::Server::new_async().await
}

// Create a test configuration
fn create_test_config(server_url: &str) -> Config {
    Config {
        server: ServerConfig {
            api_key: "test_api_key".to_string(),
            server_url: server_url.to_string(),
        },
        datasources: vec![DataSource {
            name: "test_source".to_string(),
            source_type: DataSourceType::Clickhouse,
            hosts: vec!["http://localhost:8123".to_string()],
            username: "default".to_string(),
            password: "".to_string(),
            filters: None,
            timeout: 60,
        }],
        global_filters: None,
    }
}

#[tokio::test]
async fn test_initialize_agents() {
    // Setup mock server
    let server = setup_test_server().await;

    let server_url = server.url();

    // Create test config
    let config = create_test_config(&server_url);

    // Initialize agents
    let (hp_agent, job_agent, main_agent) = initialize_agents(&config);

    // Verify agents were created with correct types
    match hp_agent {
        Agent::Observation(agent) => {
            assert!(agent.is_high_priority_queue);
        }
        _ => panic!("Expected Observation agent for high priority"),
    }

    match job_agent {
        Agent::Job(_) => {
            // Job agent created correctly
        }
        _ => panic!("Expected Job agent"),
    }

    match main_agent {
        Agent::Observation(agent) => {
            assert!(!agent.is_high_priority_queue);
        }
        _ => panic!("Expected Observation agent for main agent"),
    }
}
