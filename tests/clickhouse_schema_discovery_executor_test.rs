use std::path::PathBuf;
use tsight_agent::{config::Config, models::DataSource};

#[tokio::test]
async fn test_schema_discovery() {
    let config_path: PathBuf = PathBuf::from("tests/test_configs/simple_config.yaml");
    let config = Config::load(&config_path).unwrap();

    let datasource: &DataSource = &config.datasources[0];

    let executor: tsight_agent::executors::clickhouse_source::ClickhouseExecutor =
        tsight_agent::executors::clickhouse_source::ClickhouseExecutor::with_global_filters(
            &datasource.hosts[0],
            &datasource.username,
            &datasource.password,
            config.global_filters.clone(),
        )
        .expect("Failed to create executor");

    let schemas = executor.discover_schemas().await.unwrap();

    assert_eq!(schemas.len(), 6);

    // Find the "orders" table schema
    let schema = schemas
        .into_iter()
        .find(|sch| sch.table == "orders")
        .expect("Orders table schema not found");

    assert_eq!(schema.database, "test_db");
    assert_eq!(schema.table, "orders");
    assert_eq!(schema.row_count, 14);

    // Verify all columns exist with correct types
    let columns: &std::collections::HashMap<
        String,
        tsight_agent::executors::clickhouse_source::ColumnInfo,
    > = &schema.columns;
    assert_eq!(columns.get("id").unwrap().type_name, "string");
    assert_eq!(columns.get("user_id").unwrap().type_name, "string");
    assert_eq!(
        columns
            .get("notification_recipient_email")
            .unwrap()
            .type_name,
        "string"
    );
    assert_eq!(columns.get("order_name").unwrap().type_name, "string");
    assert_eq!(columns.get("order_cost").unwrap().type_name, "string");
    assert_eq!(columns.get("created_at").unwrap().type_name, "datetime");
    assert_eq!(columns.get("updated_at").unwrap().type_name, "datetime");
    assert_eq!(columns.get("status").unwrap().type_name, "string");
    assert_eq!(columns.get("is_deleted").unwrap().type_name, "bool");

    // Verify cardinality for each column
    assert_eq!(columns.get("id").unwrap().cardinality, Some(14));
    assert_eq!(columns.get("user_id").unwrap().cardinality, Some(14));
    assert_eq!(
        columns
            .get("notification_recipient_email")
            .unwrap()
            .cardinality,
        Some(5)
    );
    assert_eq!(columns.get("order_name").unwrap().cardinality, Some(4));
    assert_eq!(columns.get("order_cost").unwrap().cardinality, Some(5));
    assert_eq!(columns.get("created_at").unwrap().cardinality, Some(10));
    assert_eq!(columns.get("status").unwrap().cardinality, Some(6));
    assert_eq!(columns.get("is_deleted").unwrap().cardinality, Some(1));
}
