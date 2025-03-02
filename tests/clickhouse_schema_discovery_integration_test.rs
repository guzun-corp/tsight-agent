use mockito::{Mock, Server, ServerGuard};
use serde_json::{json, Value};
use std::path::Path;
use tsight_agent::{agent::discover_and_submit_schemas, client::ServerClient, config::Config};

// Test constants
const TEST_API_KEY: &str = "test-api-key";
const TEST_BEARER_HEADER: &str = "Bearer test-api-key";
const TEST_DATASOURCE_NAME: &str = "test_clickhouse";
const TEST_DATASOURCE_TYPE: &str = "clickhouse";

/// Test schema discovery with no filters
#[tokio::test]
async fn test_schema_discovery_integration_with_no_filters() {
    run_schema_discovery_test(
        "tests/test_configs/simple_config.yaml",
        create_no_filters_schema_data(),
    )
    .await;
}

/// Test schema discovery with filters
#[tokio::test]
async fn test_schema_discovery_integration_with_filters() {
    run_schema_discovery_test(
        "tests/test_configs/include_only_sql_filters_config.yaml",
        create_filtered_schema_data(),
    )
    .await;
}

/// Common test runner for schema discovery tests
async fn run_schema_discovery_test(config_path: &str, schema_data: Value) {
    // ARRANGE: Set up test environment
    let mut server = Server::new_async().await;
    let test_config_path = Path::new(config_path);

    // Set up mocks
    let mocks = setup_mocks(&mut server, schema_data).await;

    // Create test dependencies
    let config = Config::load(test_config_path).expect("Failed to load test config");
    let server_client = ServerClient::new(TEST_API_KEY.to_string(), server.url());

    // ACT: Run the function under test
    let result = discover_and_submit_schemas(
        &config.datasources,
        &server_client,
        config.global_filters.clone(),
    )
    .await;

    // ASSERT: Verify results
    assert!(
        result.is_ok(),
        "Schema discovery failed: {:?}",
        result.err()
    );

    // Verify all mocks were called with expected parameters
    for mock in mocks {
        mock.assert_async().await;
    }
}

/// Sets up all mock endpoints needed for the test
async fn setup_mocks(server: &mut ServerGuard, schema_data: Value) -> Vec<Mock> {
    let mut mocks = Vec::new();

    // Mock for adding datasource
    mocks.push(mock_add_datasource(server).await);

    // Mock for schema discovery
    mocks.push(mock_schema_discovery(server, schema_data).await);

    mocks
}

/// Create mock for adding datasource
async fn mock_add_datasource(server: &mut ServerGuard) -> Mock {
    server
        .mock(
            "POST",
            format!("/datasource/{}/add", TEST_DATASOURCE_NAME).as_str(),
        )
        .match_header("Authorization", TEST_BEARER_HEADER)
        .match_body(mockito::Matcher::Json(
            json!({"datasource_type": TEST_DATASOURCE_TYPE}),
        ))
        .with_status(200)
        .create_async()
        .await
}

/// Create mock for schema discovery
async fn mock_schema_discovery(server: &mut ServerGuard, schema_data: Value) -> Mock {
    server
        .mock(
            "POST",
            format!("/datasource/{}/discovery", TEST_DATASOURCE_NAME).as_str(),
        )
        .match_header("Authorization", TEST_BEARER_HEADER)
        .with_status(200)
        .match_body(mockito::Matcher::Json(schema_data))
        .create_async()
        .await
}

/// Create schema data for test with no filters
fn create_no_filters_schema_data() -> Value {
    json!({
        "schemas": [
            create_table_schema("_test_db2", "table1", 1, vec![
                ("id", "string", 1),
                ("data", "string", 1),
            ]),
            create_table_schema("_test_db2", "table2", 1, vec![
                ("id", "string", 1),
                ("data", "string", 1),
            ]),
            create_table_schema("test_db", "_my_lovely_tmp_table", 5, vec![
                ("id", "string", 5),
                ("country", "string", 5),
                ("currency", "string", 4),
            ]),
            create_table_schema("test_db", "card_numbers", 5, vec![
                ("card_number", "string", 5),
                ("id", "string", 5),
                ("is_deleted", "bool", 1),
            ]),
            create_table_schema("test_db", "orders", 14, vec![
                ("is_deleted", "bool", 1),
                ("notification_recipient_email", "string", 5),
                ("user_id", "string", 14),
                ("order_cost", "string", 5),
                ("updated_at", "datetime", 1),
                ("id", "string", 14),
                ("created_at", "datetime", 10),
                ("order_name", "string", 4),
                ("status", "string", 6),
            ]),
            create_table_schema("test_db", "some_secret_db", 2, vec![
                ("master_password", "string", 2),
                ("id", "string", 2),
            ]),
        ]
    })
}

/// Create schema data for test with filters
fn create_filtered_schema_data() -> Value {
    json!({
        "schemas": [
            create_table_schema("test_db", "_my_lovely_tmp_table", 5, vec![
                ("currency", "string", 4),
                ("country", "string", 5),
            ]),
            create_table_schema("test_db", "orders", 14, vec![
                ("order_name", "string", 4),
                ("status", "string", 6),
            ]),
        ]
    })
}

/// Helper to create a table schema with the given parameters
fn create_table_schema(
    database: &str,
    table: &str,
    row_count: u64,
    columns: Vec<(&str, &str, u64)>,
) -> Value {
    let mut column_map = serde_json::Map::new();

    for (name, type_name, cardinality) in columns {
        column_map.insert(
            name.to_string(),
            json!({
                "type_name": type_name,
                "cardinality": cardinality
            }),
        );
    }

    json!({
        "database": database,
        "table": table,
        "row_count": row_count,
        "columns": column_map
    })
}
