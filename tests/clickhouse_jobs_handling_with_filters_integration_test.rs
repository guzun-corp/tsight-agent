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

const TEST_QUERY_2: &str = "
SELECT
    count() as cnt,
    notification_recipient_email,
    order_name,
    status
FROM test_db.orders
WHERE created_at >= '2025-01-30 00:00:00'
GROUP BY
    status,
    notification_recipient_email,
    order_name,
    status
";

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
    tsight_agent::agent::factory::create_job_agent(
        TEST_API_KEY.to_string(),
        server_url.to_string(),
        datasources,
        None,
    )
}

fn mock_acquire_success(
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

fn mock_submit_results(server: &mut mockito::ServerGuard) -> Mock {
    server
        .mock("POST", format!("/jobs/{}/submit", TEST_TASK_ID).as_str())
        .match_body(mockito::Matcher::Json(json!(
{
"records":
[
{"status":"user4@example.com","order_name":"Fourth Order","notification_recipient_email":"user4@example.com","cnt":"1"},
{"notification_recipient_email":"user2@example.com","cnt":"1","status":"processing","order_name":"Second Order"},
{"cnt":"1","status":"completed","order_name":"Third Order","notification_recipient_email":"user3@example.com"},
{"notification_recipient_email":"user4@example.com","order_name":"Fourth Order","cnt":"7","status":"cancelled"},
{"order_name":"First Order","status":"new","notification_recipient_email":"user1@example.com","cnt":"1"},
{"notification_recipient_email":"user0@example.com","order_name":"First Order","status":"new","cnt":"1"},
{"notification_recipient_email":"user4@example.com","status":"4222 2222 2222 2","cnt":"1","order_name":"Fourth Order"},
{"status":"cancelled","cnt":"1","notification_recipient_email":"user3@example.com","order_name":"Third Order"}
]
}
        )))
        .match_header("Authorization", TEST_BEARER_HEADER)
        .with_status(200)
        .create()
}

#[tokio::test]
async fn test_process_next_success_no_filters() {
    let mut server = setup_test_server().await;

    // Create mock responses
    let acquire_mock = mock_acquire_success(&mut server, TEST_DATASOURCE_NAME, TEST_QUERY_2);
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
