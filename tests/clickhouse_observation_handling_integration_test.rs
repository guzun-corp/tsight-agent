use mockito::{Mock, Server};
use serde_json::json;
use tsight_agent::{
    agent::Agent,
    models::{DataSource, DataSourceType},
};

// Test constants
const TEST_API_KEY: &str = "test-api-key";
const TEST_BEARER_HEADER: &str = "Bearer test-api-key";
const TEST_TASK_ID: &str = "123";
const TEST_DATASOURCE_NAME: &str = "test_clickhouse";
const TEST_INVALID_DATASOURCE: &str = "nonexistent_datasource";

const TEST_QUERY: &str = "
SELECT
    toUnixTimestamp(toStartOfInterval(
        toTimeZone(
            created_at, 'UTC'
        ), INTERVAL 1 minute
    )) as t
    , count() / toUnixTimestamp(
        FROM_UNIXTIME(0) + INTERVAL 1 minute
    ) as cnt
FROM test_db.orders
WHERE status = 'cancelled'
AND created_at >= '2025-01-30 00:00:00'
GROUP BY
    t, status
ORDER BY
    t
";

const INVALID_QUERY: &str = "SELECT invalid_syntax";

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

fn create_agent(server_url: &str, datasources: Vec<DataSource>) -> Agent {
    tsight_agent::agent::factory::create_observation_agent(
        TEST_API_KEY.to_string(),
        server_url.to_string(),
        datasources,
        false,
        None,
    )
}

fn create_high_priority_agent(server_url: &str, datasources: Vec<DataSource>) -> Agent {
    tsight_agent::agent::factory::create_observation_agent(
        TEST_API_KEY.to_string(),
        server_url.to_string(),
        datasources,
        true,
        None,
    )
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

fn mock_acquire_high_priority_success(
    server: &mut mockito::ServerGuard,
    datasource_name: &str,
    query: &str,
) -> Mock {
    server
        .mock("POST", "/tasks/acquire")
        .match_header("Authorization", TEST_BEARER_HEADER)
        .match_body(mockito::Matcher::Json(
            json!({"is_high_priority_queue":true}),
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

fn mock_acquire_error(server: &mut mockito::ServerGuard, status: usize) -> Mock {
    server
        .mock("POST", "/tasks/acquire")
        .match_header("Authorization", TEST_BEARER_HEADER)
        .with_status(status)
        .with_body(json!({"error": "some"}).to_string())
        .create()
}

fn mock_submit_results(server: &mut mockito::ServerGuard) -> Mock {
    server
        .mock("POST", format!("/tasks/{}/submit", TEST_TASK_ID).as_str())
        .match_body(mockito::Matcher::Json(
            json!(
                {
                    "records":
                    [
                        {"t":1738280700,"cnt":0.016666666666666666},{"t":1738281060,"cnt":0.016666666666666666},
                        {"cnt":0.016666666666666666,"t":1738281120},{"cnt":0.016666666666666666,"t":1738281180},
                        {"cnt":0.05,"t":1738281240},{"cnt":0.016666666666666666,"t":1738281300}
                    ],
                    "is_high_priority_queue":false
                }
            )
        ))
        .match_header("Authorization", TEST_BEARER_HEADER)
        .with_status(200)
        .create()
}

fn mock_submit_high_priority_results(server: &mut mockito::ServerGuard) -> Mock {
    server
        .mock("POST", format!("/tasks/{}/submit", TEST_TASK_ID).as_str())
        .match_body(mockito::Matcher::Json(
            json!(
                {
                    "records":
                    [
                        {"t":1738280700,"cnt":0.016666666666666666},{"t":1738281060,"cnt":0.016666666666666666},
                        {"cnt":0.016666666666666666,"t":1738281120},{"cnt":0.016666666666666666,"t":1738281180},
                        {"cnt":0.05,"t":1738281240},{"cnt":0.016666666666666666,"t":1738281300}
                    ],
                    "is_high_priority_queue":true
                }
            )
        ))
        .match_header("Authorization", TEST_BEARER_HEADER)
        .with_status(200)
        .create()
}

fn mock_submit_error(server: &mut mockito::ServerGuard, error_message: &str) -> Mock {
    server
        .mock("POST", format!("/tasks/{}/submit", TEST_TASK_ID).as_str())
        .match_header("Authorization", TEST_BEARER_HEADER)
        .match_body(mockito::Matcher::Json(
            json!({"error": error_message, "is_high_priority_queue":false}),
        ))
        .with_status(200)
        .create()
}

fn mock_submit_high_priority_error(server: &mut mockito::ServerGuard, error_message: &str) -> Mock {
    server
        .mock("POST", format!("/tasks/{}/submit", TEST_TASK_ID).as_str())
        .match_header("Authorization", TEST_BEARER_HEADER)
        .match_body(mockito::Matcher::Json(
            json!({"error": error_message, "is_high_priority_queue": true}),
        ))
        .with_status(200)
        .create()
}

fn mock_submit_error_no_body_matching(server: &mut mockito::ServerGuard) -> Mock {
    server
        .mock("POST", format!("/tasks/{}/submit", TEST_TASK_ID).as_str())
        .match_header("Authorization", TEST_BEARER_HEADER)
        .with_status(200)
        .create()
}

#[tokio::test]
async fn test_process_next_success() {
    let mut server = setup_test_server().await;

    // Create mock responses
    let acquire_mock = mock_acquire_success(&mut server, TEST_DATASOURCE_NAME, TEST_QUERY);
    let submit_mock = mock_submit_results(&mut server);

    // Create test datasource and agent
    let datasources = vec![create_test_datasource(vec![
        "http://localhost:8123".to_string()
    ])];
    let agent = create_agent(&server.url(), datasources);

    // Execute test
    let result = agent.process_next().await;

    // Verify results
    assert!(
        result.is_ok(),
        "Failed to process query: {:?}",
        result.err()
    );
    acquire_mock.assert();
    submit_mock.assert();
}

#[tokio::test]
async fn test_process_next_acquire_failure() {
    let mut server = setup_test_server().await;

    // Create mock response
    let acquire_mock = mock_acquire_error(&mut server, 500);

    // Create test datasource and agent
    let datasources = vec![create_test_datasource(vec![
        "http://localhost:8123".to_string()
    ])];
    let agent = create_agent(&server.url(), datasources);

    // Execute test
    let result = agent.process_next().await;

    // Verify results
    assert!(result.is_err(), "Expected an error but got success");
    let error_msg = result.unwrap_err().to_string();
    assert_eq!(
        error_msg,
        "Failed to acquire next query from server: Failed to acquire task: 500 Internal Server Error",
        "Error message doesn't match expected content"
    );
    acquire_mock.assert();
}

#[tokio::test]
async fn test_process_next_task_not_found() {
    let mut server = setup_test_server().await;

    // Create mock response
    let acquire_mock = mock_acquire_error(&mut server, 404);

    // Create test datasource and agent
    let datasources = vec![create_test_datasource(vec![
        "http://localhost:8123".to_string()
    ])];
    let agent = create_agent(&server.url(), datasources);

    // Execute test
    let result = agent.process_next().await;

    // Verify results
    assert!(result.is_err(), "Expected an error but got success");
    let error_msg = result.unwrap_err().to_string();
    assert_eq!(
        error_msg, "Failed to acquire next query from server: No tasks available",
        "Error message doesn't match expected content"
    );
    acquire_mock.assert();
}

#[tokio::test]
async fn test_process_next_datasource_not_found() {
    let mut server = setup_test_server().await;

    // Create mock responses
    let error_message = format!(
        "No matching datasource found for query {}",
        TEST_INVALID_DATASOURCE
    );
    let acquire_mock = mock_acquire_success(&mut server, TEST_INVALID_DATASOURCE, TEST_QUERY);
    let submit_error_mock = mock_submit_error(&mut server, &error_message);

    // Create test datasource and agent
    let datasources = vec![create_test_datasource(vec![
        "http://localhost:8123".to_string()
    ])];
    let agent = create_agent(&server.url(), datasources);

    // Execute test
    let result = agent.process_next().await;

    // Verify results
    assert!(result.is_err(), "Expected an error but got success");
    let error_msg = result.unwrap_err().to_string();
    assert_eq!(
        error_msg, error_message,
        "Error message doesn't match expected content"
    );
    acquire_mock.assert();
    submit_error_mock.assert();
}

#[tokio::test]
async fn test_process_next_executor_creation_failure() {
    let mut server = setup_test_server().await;

    // Create mock responses
    let error_message = "No host specified for Clickhouse datasource";
    let acquire_mock = mock_acquire_success(&mut server, TEST_DATASOURCE_NAME, TEST_QUERY);
    let submit_error_mock = mock_submit_error(&mut server, error_message);

    // Create test datasource with empty hosts (to trigger the error)
    let datasources = vec![create_test_datasource(vec![])];
    let agent = create_agent(&server.url(), datasources);

    // Execute test
    let result = agent.process_next().await;

    // Verify results
    assert!(result.is_err(), "Expected an error but got success");
    let error_msg = result.unwrap_err().to_string();
    assert_eq!(
        error_msg, error_message,
        "Error message doesn't match expected content"
    );
    acquire_mock.assert();
    submit_error_mock.assert();
}

#[tokio::test]
async fn test_process_next_execution_failure() {
    let mut server = setup_test_server().await;

    // Expected error message from ClickHouse when syntax is invalid
    let error_message = "Query execution error for query: Query execution error: bad response: Code: 47. DB::Exception: Unknown expression identifier `invalid_syntax` in scope SELECT invalid_syntax. (UNKNOWN_IDENTIFIER)";

    // Create mock responses
    let acquire_mock = mock_acquire_success(&mut server, TEST_DATASOURCE_NAME, INVALID_QUERY);
    let submit_error_mock = mock_submit_error_no_body_matching(&mut server);

    // Create test datasource and agent
    let datasources = vec![create_test_datasource(vec![
        "http://localhost:8123".to_string()
    ])];
    let agent = create_agent(&server.url(), datasources);

    // Execute test
    let result = agent.process_next().await;

    // Verify results
    assert!(result.is_err(), "Expected an error but got success");
    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains(error_message),
        "Error message doesn't contain expected content"
    );
    acquire_mock.assert();
    submit_error_mock.assert();
}

#[tokio::test]
async fn test_high_priority_process_next_success() {
    let mut server = setup_test_server().await;

    // Create mock responses
    let acquire_mock =
        mock_acquire_high_priority_success(&mut server, TEST_DATASOURCE_NAME, TEST_QUERY);
    let submit_mock = mock_submit_high_priority_results(&mut server);

    // Create test datasource and high priority agent
    let datasources = vec![create_test_datasource(vec![
        "http://localhost:8123".to_string()
    ])];
    let agent = create_high_priority_agent(&server.url(), datasources);

    // Execute test
    let result = agent.process_next().await;

    // Verify results
    assert!(
        result.is_ok(),
        "Failed to process query: {:?}",
        result.err()
    );
    acquire_mock.assert();
    submit_mock.assert();
}

#[tokio::test]
async fn test_high_priority_process_next_datasource_not_found() {
    let mut server = setup_test_server().await;

    // Create mock responses
    let error_message = format!(
        "No matching datasource found for query {}",
        TEST_INVALID_DATASOURCE
    );
    let acquire_mock =
        mock_acquire_high_priority_success(&mut server, TEST_INVALID_DATASOURCE, TEST_QUERY);
    let submit_error_mock = mock_submit_high_priority_error(&mut server, &error_message);

    // Create test datasource and high priority agent
    let datasources = vec![create_test_datasource(vec![
        "http://localhost:8123".to_string()
    ])];
    let agent = create_high_priority_agent(&server.url(), datasources);

    // Execute test
    let result = agent.process_next().await;

    // Verify results
    assert!(result.is_err(), "Expected an error but got success");
    let error_msg = result.unwrap_err().to_string();
    assert_eq!(
        error_msg, error_message,
        "Error message doesn't match expected content"
    );
    acquire_mock.assert();
    submit_error_mock.assert();
}

#[tokio::test]
async fn test_high_priority_process_next_execution_failure() {
    let mut server = setup_test_server().await;

    // Expected error message from ClickHouse when syntax is invalid
    let error_message = "Query execution error for query: Query execution error: bad response: Code: 47. DB::Exception: Unknown expression identifier `invalid_syntax` in scope SELECT invalid_syntax. (UNKNOWN_IDENTIFIER)";

    // Create mock responses
    let acquire_mock =
        mock_acquire_high_priority_success(&mut server, TEST_DATASOURCE_NAME, INVALID_QUERY);
    let submit_error_mock = mock_submit_error_no_body_matching(&mut server);

    // Create test datasource and high priority agent
    let datasources = vec![create_test_datasource(vec![
        "http://localhost:8123".to_string()
    ])];
    let agent = create_high_priority_agent(&server.url(), datasources);

    // Execute test
    let result = agent.process_next().await;

    // Verify results
    assert!(result.is_err(), "Expected an error but got success");
    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains(error_message),
        "Error message doesn't contain expected content"
    );
    acquire_mock.assert();
    submit_error_mock.assert();
}
