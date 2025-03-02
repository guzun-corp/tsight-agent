use mockito::{Mock, Server};
use serde_json::json;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::timeout;
use tsight_agent::models::{DataSource, DataSourceType};

// Test constants
const TEST_API_KEY: &str = "test-api-key";
const TEST_BEARER_HEADER: &str = "Bearer test-api-key";
const TEST_TASK_ID: &str = "123";
const TEST_DATASOURCE_NAME: &str = "test_clickhouse";
const TEST_QUERY: &str = "SELECT 1";

// Helper functions
async fn setup_test_server() -> mockito::ServerGuard {
    Server::new_async().await
}

fn create_test_datasource(hosts: Vec<String>) -> DataSource {
    DataSource {
        name: TEST_DATASOURCE_NAME.to_string(),
        source_type: DataSourceType::Clickhouse,
        hosts,
        username: "test_user".to_string(),
        password: "test_password".to_string(),
        timeout: 60,
        filters: None,
    }
}

fn mock_acquire_success(
    server: &mut mockito::ServerGuard,
    datasource_name: &str,
    query: &str,
) -> Mock {
    server
        .mock("POST", "/tasks/acquire")
        .match_header("Authorization", TEST_BEARER_HEADER)
        .match_body(mockito::Matcher::Json(
            json!({"is_high_priority_queue":false}),
        ))
        .with_status(200)
        .with_body(
            json!({
                "id": TEST_TASK_ID,
                "datasource_name": datasource_name,
                "query": query,
            })
            .to_string(),
        )
        .create()
}

fn mock_submit_error_failure(server: &mut mockito::ServerGuard) -> Mock {
    server
        .mock("POST", format!("/tasks/{}/submit", TEST_TASK_ID).as_str())
        .match_header("Authorization", TEST_BEARER_HEADER)
        .with_status(500)
        .with_body(json!({"error": "Internal server error"}).to_string())
        .create()
}

fn mock_job_acquire_success(
    server: &mut mockito::ServerGuard,
    datasource_name: &str,
    query: &str,
) -> Mock {
    server
        .mock("POST", "/jobs/acquire")
        .match_header("Authorization", TEST_BEARER_HEADER)
        .with_status(200)
        .with_body(
            json!({
                "id": TEST_TASK_ID,
                "datasource_name": datasource_name,
                "query": query,
            })
            .to_string(),
        )
        .create()
}

fn mock_job_submit_error_failure(server: &mut mockito::ServerGuard) -> Mock {
    server
        .mock("POST", format!("/jobs/{}/submit", TEST_TASK_ID).as_str())
        .match_header("Authorization", TEST_BEARER_HEADER)
        .with_status(500)
        .with_body(json!({"error": "Internal server error"}).to_string())
        .create()
}

fn mock_acquire_no_tasks(server: &mut mockito::ServerGuard) -> Mock {
    server
        .mock("POST", "/tasks/acquire")
        .match_header("Authorization", TEST_BEARER_HEADER)
        .with_status(404)
        .with_body(json!({"error": "No tasks available"}).to_string())
        .expect(3) // Expect 3 calls instead of 1
        .create()
}

fn mock_job_acquire_no_jobs(server: &mut mockito::ServerGuard) -> Mock {
    server
        .mock("POST", "/jobs/acquire")
        .match_header("Authorization", TEST_BEARER_HEADER)
        .with_status(404)
        .with_body(json!({"error": "No jobs available"}).to_string())
        .expect(3) // Expect 3 calls instead of 1
        .create()
}

#[tokio::test]
async fn test_observation_agent_submit_error_failure() {
    let mut server = setup_test_server().await;

    // Create mock responses
    let acquire_mock = mock_acquire_success(&mut server, TEST_DATASOURCE_NAME, "INVALID QUERY");
    let submit_error_mock = mock_submit_error_failure(&mut server);

    // Create test datasource and agent
    let datasources = vec![create_test_datasource(vec![
        "http://invalid-host:8123".to_string()
    ])];
    let agent = tsight_agent::agent::factory::create_observation_agent(
        TEST_API_KEY.to_string(),
        server.url(),
        datasources,
        false,
        None,
    );

    // Execute test
    let result = agent.process_next().await;

    // Verify results
    assert!(result.is_err(), "Expected an error but got success");
    acquire_mock.assert();
    submit_error_mock.assert();
}

#[tokio::test]
async fn test_job_agent_submit_error_failure() {
    let mut server = setup_test_server().await;

    // Create mock responses
    let acquire_mock = mock_job_acquire_success(&mut server, TEST_DATASOURCE_NAME, "INVALID QUERY");
    let submit_error_mock = mock_job_submit_error_failure(&mut server);

    // Create test datasource and agent
    let datasources = vec![create_test_datasource(vec![
        "http://invalid-host:8123".to_string()
    ])];
    let agent = tsight_agent::agent::factory::create_job_agent(
        TEST_API_KEY.to_string(),
        server.url(),
        datasources,
        None,
    );

    // Execute test
    let result = agent.process_next().await;

    // Verify results
    assert!(result.is_err(), "Expected an error but got success");
    acquire_mock.assert();
    submit_error_mock.assert();
}

#[tokio::test]
async fn test_agent_factory_methods() {
    // Test create_agent with Observation type
    let datasources = vec![create_test_datasource(vec![
        "http://localhost:8123".to_string()
    ])];
    let agent = tsight_agent::agent::factory::create_observation_agent(
        TEST_API_KEY.to_string(),
        "http://localhost:8080".to_string(),
        datasources.clone(),
        false,
        None,
    );

    // Verify agent type by checking datasources
    assert_eq!(agent.datasources()[0].name, TEST_DATASOURCE_NAME);

    // Test create_agent with Job type
    let job_agent = tsight_agent::agent::factory::create_job_agent(
        TEST_API_KEY.to_string(),
        "http://localhost:8080".to_string(),
        datasources.clone(),
        None,
    );

    // Verify agent type by checking datasources
    assert_eq!(job_agent.datasources()[0].name, TEST_DATASOURCE_NAME);
}

#[tokio::test]
async fn test_agent_run_with_no_tasks() {
    let mut server = setup_test_server().await;

    // Create mock response for no tasks
    let acquire_mock = mock_acquire_no_tasks(&mut server);

    // Create test datasource and agent
    let datasources = vec![create_test_datasource(vec![
        "http://localhost:8123".to_string()
    ])];
    let agent = tsight_agent::agent::factory::create_observation_agent(
        TEST_API_KEY.to_string(),
        server.url(),
        datasources,
        false,
        None,
    );

    // Create a counter to track how many times the loop runs
    let counter = Arc::new(Mutex::new(0));
    let counter_clone = counter.clone();

    // Create a modified run function that counts iterations and exits after a few
    let run_task = async move {
        for _ in 0..3 {
            match agent.process_next().await {
                Ok(_) => (),
                Err(e) => {
                    let mut count = counter_clone.lock().unwrap();
                    *count += 1;
                    if e.to_string().contains("No tasks available") {
                        // Expected error
                    } else {
                        panic!("Unexpected error: {}", e);
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    };

    // Run with timeout to ensure it completes
    let _ = timeout(Duration::from_secs(1), run_task)
        .await
        .expect("Test timed out");

    // Verify the loop ran and encountered the expected errors
    let count = *counter.lock().unwrap();
    assert!(count > 0, "Loop should have run and encountered errors");

    // Verify mock was called
    acquire_mock.assert();
}

#[tokio::test]
async fn test_job_agent_run_with_no_jobs() {
    let mut server = setup_test_server().await;

    // Create mock response for no jobs
    let acquire_mock = mock_job_acquire_no_jobs(&mut server);

    // Create test datasource and agent
    let datasources = vec![create_test_datasource(vec![
        "http://localhost:8123".to_string()
    ])];
    let agent = tsight_agent::agent::factory::create_job_agent(
        TEST_API_KEY.to_string(),
        server.url(),
        datasources,
        None,
    );

    // Create a counter to track how many times the loop runs
    let counter = Arc::new(Mutex::new(0));
    let counter_clone = counter.clone();

    // Create a modified run function that counts iterations and exits after a few
    let run_task = async move {
        for _ in 0..3 {
            match agent.process_next().await {
                Ok(_) => (),
                Err(e) => {
                    let mut count = counter_clone.lock().unwrap();
                    *count += 1;
                    if e.to_string().contains("No jobs available") {
                        // Expected error
                    } else {
                        panic!("Unexpected error: {}", e);
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    };

    // Run with timeout to ensure it completes
    let _ = timeout(Duration::from_secs(1), run_task)
        .await
        .expect("Test timed out");

    // Verify the loop ran and encountered the expected errors
    let count = *counter.lock().unwrap();
    assert!(count > 0, "Loop should have run and encountered errors");

    // Verify mock was called
    acquire_mock.assert();
}

#[tokio::test]
async fn test_agent_run_with_unexpected_error() {
    let mut server = setup_test_server().await;

    // Create mock responses that will cause an unexpected error
    let acquire_mock =
        mock_acquire_success(&mut server, "invalid_datasource", TEST_QUERY).expect(3); // Expect 3 calls instead of 1

    // Also mock the error submission
    let _ = server
        .mock("POST", format!("/tasks/{}/submit", TEST_TASK_ID).as_str())
        .match_header("Authorization", TEST_BEARER_HEADER)
        .match_body(mockito::Matcher::Json(
            json!({"error": "No matching datasource found for query invalid_datasource", "is_high_priority_queue": false})
        ))
        .with_status(200)
        .expect(3)  // Expect 3 calls
        .create();

    // Create test datasource and agent
    let datasources = vec![create_test_datasource(vec![
        "http://localhost:8123".to_string()
    ])];
    let agent = tsight_agent::agent::factory::create_observation_agent(
        TEST_API_KEY.to_string(),
        server.url(),
        datasources,
        false,
        None,
    );

    // Create a counter to track how many times the loop runs
    let counter = Arc::new(Mutex::new(0));
    let counter_clone = counter.clone();

    // Create a modified run function that counts iterations and exits after a few
    let run_task = async move {
        for _ in 0..3 {
            match agent.process_next().await {
                Ok(_) => (),
                Err(e) => {
                    let mut count = counter_clone.lock().unwrap();
                    *count += 1;
                    if e.to_string().contains("No matching datasource found") {
                        // Expected error
                    } else {
                        panic!("Unexpected error: {}", e);
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    };

    // Run with timeout to ensure it completes
    let _ = timeout(Duration::from_secs(1), run_task)
        .await
        .expect("Test timed out");

    // Verify the loop ran and encountered the expected errors
    let count = *counter.lock().unwrap();
    assert!(count > 0, "Loop should have run and encountered errors");

    // Verify mock was called
    acquire_mock.assert();
}
