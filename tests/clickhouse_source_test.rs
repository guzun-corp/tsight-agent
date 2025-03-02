use anyhow::Result;
use std::path::PathBuf;
use tsight_agent::config::Config;
use tsight_agent::executors::base::{QueryError, QueryExecutor};
use tsight_agent::executors::clickhouse_source::ClickhouseExecutor;

// Helper function to create a test executor
async fn create_test_executor() -> ClickhouseExecutor {
    let config_path = PathBuf::from("tests/test_configs/simple_config.yaml");
    let config = Config::load(&config_path).expect("Failed to load test config");
    let datasource = &config.datasources[0];

    let host = &datasource.hosts[0];
    let username = &datasource.username;
    let password = &datasource.password;

    ClickhouseExecutor::new(host, username, password).expect("Failed to create executor")
}

#[tokio::test]
async fn test_execute_ts() -> Result<()> {
    let executor = create_test_executor().await;

    // Query that returns time series data
    let result = executor.execute_ts("SELECT toUInt32(toUnixTimestamp(created_at)) as t, count() as cnt FROM test_db.orders GROUP BY t").await?;

    // Verify we got results
    assert!(!result.is_empty());

    // Verify the structure of the results
    for record in &result {
        assert!(record.t > 0);
        assert!(record.cnt > 0.0);
    }

    Ok(())
}

#[tokio::test]
async fn test_execute_job() -> Result<()> {
    let executor = create_test_executor().await;

    // Query that returns job data
    let result = executor
        .execute_job(
            "SELECT notification_recipient_email, order_name, status FROM test_db.orders LIMIT 3",
        )
        .await?;

    // Verify we got exactly 3 results
    assert_eq!(result.len(), 3);

    // Verify the structure of the results
    for record in &result {
        assert!(record.contains_key("notification_recipient_email"));
        assert!(record.contains_key("order_name"));
        assert!(record.contains_key("status"));

        // Check that the values are strings
        assert!(record["notification_recipient_email"].is_string());
        assert!(record["order_name"].is_string());
        assert!(record["status"].is_string());
    }

    Ok(())
}

#[tokio::test]
async fn test_execute_ts_error() -> Result<()> {
    let executor = create_test_executor().await;

    // Invalid query that should cause an error
    let result = executor
        .execute_ts("SELECT invalid_column FROM non_existent_table")
        .await;

    // Verify we got an error
    assert!(result.is_err());

    // Check the error type
    match result {
        Err(QueryError::ExecutionError(_)) => {
            // This is the expected error type
            Ok(())
        }
        _ => {
            panic!("Expected ExecutionError");
        }
    }
}

#[tokio::test]
async fn test_execute_job_error() -> Result<()> {
    let executor = create_test_executor().await;

    // Invalid query that should cause an error
    let result = executor
        .execute_job("SELECT invalid_column FROM non_existent_table")
        .await;

    // Verify we got an error
    assert!(result.is_err());

    // Check the error type
    match result {
        Err(QueryError::ExecutionError(_)) => {
            // This is the expected error type
            Ok(())
        }
        _ => {
            panic!("Expected ExecutionError");
        }
    }
}

#[tokio::test]
async fn test_connect() -> Result<()> {
    let mut executor = create_test_executor().await;

    // Test the connection
    let result = executor.connect().await;

    // Verify the connection was successful
    assert!(result.is_ok());

    Ok(())
}

#[tokio::test]
async fn test_connect_error() -> Result<()> {
    // Create an executor with invalid credentials
    let mut executor =
        ClickhouseExecutor::new("http://localhost:8123", "invalid_user", "invalid_password")
            .unwrap();

    // Test the connection
    let result = executor.connect().await;

    // Verify the connection failed
    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_discover_schemas() -> Result<()> {
    let executor = create_test_executor().await;

    // Discover schemas
    let schemas = executor.discover_schemas().await?;

    // Verify we got some schemas
    assert!(!schemas.is_empty());

    // Look for our test_db.orders table
    let orders_schema = schemas
        .iter()
        .find(|s| s.database == "test_db" && s.table == "orders");
    assert!(
        orders_schema.is_some(),
        "test_db.orders table not found in schema discovery"
    );

    let orders = orders_schema.unwrap();

    // Verify the row count matches our test data
    assert!(
        orders.row_count >= 12,
        "Expected at least 12 rows in test_db.orders"
    );

    // Verify some of the columns exist and have the right types
    assert!(orders.columns.contains_key("id"));
    assert!(orders.columns.contains_key("notification_recipient_email"));
    assert!(orders.columns.contains_key("order_name"));
    assert!(orders.columns.contains_key("status"));

    // Check column types
    assert_eq!(
        orders
            .columns
            .get("notification_recipient_email")
            .unwrap()
            .type_name,
        "string"
    );
    assert_eq!(
        orders.columns.get("created_at").unwrap().type_name,
        "datetime"
    );
    assert_eq!(orders.columns.get("is_deleted").unwrap().type_name, "bool");

    Ok(())
}

#[tokio::test]
async fn test_card_numbers_table() -> Result<()> {
    let executor = create_test_executor().await;

    // Query the card_numbers table
    let result = executor
        .execute_job("SELECT card_number FROM test_db.card_numbers")
        .await?;

    // Verify we got 2 results
    assert_eq!(result.len(), 5);

    // Check the values
    let card_numbers: Vec<&str> = result
        .iter()
        .map(|r| r.get("card_number").unwrap().as_str().unwrap())
        .collect();

    assert!(card_numbers.contains(&"4222222222222"));
    assert!(card_numbers.contains(&"42222 222 22222"));

    Ok(())
}
