use std::path::PathBuf;
use tsight_agent::{
    config::Config,
    executors::{
        base::QueryExecutor,
        clickhouse_source::{ClickhouseExecutor, TableSchema},
    },
    models::JobType,
};

// Test helpers
struct TestContext {
    executor: ClickhouseExecutor,
}

enum FilterType {
    Exclude,
    Include,
}

impl TestContext {
    async fn new(filter_type: FilterType) -> Self {
        let config_path = match filter_type {
            FilterType::Exclude => {
                PathBuf::from("tests/test_configs/exclude_only_sql_filters_config.yaml")
            }
            FilterType::Include => {
                PathBuf::from("tests/test_configs/include_only_sql_filters_config.yaml")
            }
        };

        let config = Config::load(&config_path).unwrap();
        let datasource = &config.datasources[0];

        let executor = ClickhouseExecutor::with_global_filters(
            &datasource.hosts[0],
            &datasource.username,
            &datasource.password,
            config.global_filters.clone(),
        )
        .expect("Failed to create executor");

        Self { executor }
    }

    async fn new_exclude() -> Self {
        Self::new(FilterType::Exclude).await
    }

    async fn new_include() -> Self {
        Self::new(FilterType::Include).await
    }

    async fn get_schemas(&self) -> Vec<TableSchema> {
        self.executor.discover_schemas().await.unwrap()
    }

    async fn execute_job_query(&self, query: &str) -> Vec<JobType> {
        self.executor.execute_job(query).await.unwrap()
    }

    fn assert_schema_contains_database(
        &self,
        schemas: &[TableSchema],
        db_name: &str,
        should_exist: bool,
    ) {
        let db = schemas.iter().find(|schema| schema.database == db_name);

        if should_exist {
            assert!(db.is_some(), "Database {} should be included", db_name);
        } else {
            assert!(db.is_none(), "Database {} should be excluded", db_name);
        }
    }

    fn assert_schema_contains_table(
        &self,
        schemas: &[TableSchema],
        table_name: &str,
        should_exist: bool,
    ) {
        let table = schemas.iter().find(|schema| schema.table == table_name);

        if should_exist {
            assert!(table.is_some(), "Table {} should be included", table_name);
        } else {
            assert!(table.is_none(), "Table {} should be excluded", table_name);
        }
    }

    fn assert_table_contains_column(
        &self,
        schemas: &[TableSchema],
        table_name: &str,
        column_name: &str,
        should_exist: bool,
    ) {
        let table = schemas
            .iter()
            .find(|schema| schema.table == table_name)
            .expect(&format!("Table {} should be present", table_name));

        if should_exist {
            assert!(
                table.columns.contains_key(column_name),
                "Column {} should be included",
                column_name
            );
        } else {
            assert!(
                !table.columns.contains_key(column_name),
                "Column {} should be excluded",
                column_name
            );
        }
    }

    fn assert_results_contain_values(
        &self,
        results: &[JobType],
        field: &str,
        expected_values: &[&str],
    ) {
        // Check that all expected values are present in the results
        for expected in expected_values {
            let found = results.iter().any(|r| {
                if let Some(value) = r.get(field) {
                    value.as_str().unwrap() == *expected
                } else {
                    false
                }
            });
            assert!(found, "Expected value '{}' not found in results", expected);
        }

        // Check that all values in results are in the expected list
        for result in results {
            if let Some(value) = result.get(field) {
                let value_str = value.as_str().unwrap();
                assert!(
                    expected_values.contains(&value_str),
                    "Unexpected value: {}",
                    value_str
                );
            } else {
                panic!("Field {} missing from result", field);
            }
        }
    }
}

#[tokio::test]
async fn test_global_filters_exclude_database() {
    let ctx = TestContext::new_exclude().await;
    let schemas = ctx.get_schemas().await;

    // The _test_db2 database should be excluded because it starts with "_"
    ctx.assert_schema_contains_database(&schemas, "_test_db2", false);

    // The test_db database should be included
    ctx.assert_schema_contains_database(&schemas, "test_db", true);
}

#[tokio::test]
async fn test_global_filters_exclude_table() {
    let ctx = TestContext::new_exclude().await;
    let schemas = ctx.get_schemas().await;

    // The some_secret_db table should be excluded because it contains "secret"
    ctx.assert_schema_contains_table(&schemas, "some_secret_db", false);

    // The _my_lovely_tmp_table should be excluded because it starts with "_"
    ctx.assert_schema_contains_table(&schemas, "_my_lovely_tmp_table", false);

    // The orders table should be included
    ctx.assert_schema_contains_table(&schemas, "orders", true);

    // we forgot to exclude card_numbers table, but we will filter all values that match a credit card pattern
    ctx.assert_schema_contains_table(&schemas, "card_numbers", true);
}

#[tokio::test]
async fn test_global_filters_exclude_column() {
    let ctx = TestContext::new_exclude().await;
    let schemas = ctx.get_schemas().await;

    // The user_id column should be excluded
    ctx.assert_table_contains_column(&schemas, "orders", "user_id", false);

    // The order_cost column should be excluded because it contains "cost"
    ctx.assert_table_contains_column(&schemas, "orders", "order_cost", false);

    // The order_name column should be included
    ctx.assert_table_contains_column(&schemas, "orders", "order_name", true);
}

#[tokio::test]
async fn test_global_filters_exclude_values() {
    // each row contains notification_recipient_email. We will filter all values that match a credit card pattern
    let ctx = TestContext::new_exclude().await;

    // Execute a query that would return email addresses and credit card numbers
    let query = "SELECT notification_recipient_email, status FROM test_db.orders GROUP BY notification_recipient_email, status";
    let results = ctx.execute_job_query(query).await;

    // Check that results are empty because notification_recipient_email should trigger value filtering
    dbg!("results {:?}", &results);

    // All rows should be filtered out because they contain email addresses
    assert!(
        results.is_empty(),
        "Results should be empty due to email filtering"
    );
}

#[tokio::test]
async fn test_global_filters_exclude_order_status_values() {
    // we have card number and email values in the status column. Let's check that we filter them out
    let ctx = TestContext::new_exclude().await;

    // Execute a query that would return email addresses and credit card numbers
    let query = "SELECT status FROM test_db.orders GROUP BY status";
    let results = ctx.execute_job_query(query).await;

    // Check that results contain only valid statuses (sensitive values filtered out)
    dbg!("results {:?}", &results);

    // Verify the expected statuses are present
    assert_eq!(results.len(), 4, "Should have exactly 4 status values");

    // Create a set of expected statuses
    let expected_statuses = vec!["new", "completed", "cancelled", "processing"];

    // Check that all values match expected ones
    ctx.assert_results_contain_values(&results, "status", &expected_statuses);
}

#[tokio::test]
async fn test_global_filters_exclude_card_numbers_values() {
    // we have card number and email values in the status column. Let's check that we filter them out
    let ctx = TestContext::new_exclude().await;

    // Execute a query that would return email addresses and credit card numbers
    let query = "SELECT card_number FROM test_db.card_numbers GROUP BY card_number";
    let results = ctx.execute_job_query(query).await;

    // Check that results contain only valid card numbers (sensitive values filtered out)
    dbg!("results {:?}", &results);

    // Verify the expected card numbers are present
    assert_eq!(results.len(), 1, "Should have exactly 1 card number value");

    // Create a set of expected card numbers
    let expected_card_numbers = vec!["3530111333300000"];

    // Check that all values match expected ones
    ctx.assert_results_contain_values(&results, "card_number", &expected_card_numbers);
}

// Tests for include filters
#[tokio::test]
async fn test_global_filters_include_database() {
    let ctx = TestContext::new_include().await;
    let schemas = ctx.get_schemas().await;

    // The _test_db2 database should be excluded because it starts with "_" (not matching the allow pattern)
    ctx.assert_schema_contains_database(&schemas, "_test_db2", false);

    // The test_db database should be included because it matches the allow pattern
    ctx.assert_schema_contains_database(&schemas, "test_db", true);
}

#[tokio::test]
async fn test_global_filters_include_table() {
    let ctx = TestContext::new_include().await;
    let schemas = ctx.get_schemas().await;

    // The some_secret_db table should be excluded because it doesn't match any allow pattern
    ctx.assert_schema_contains_table(&schemas, "some_secret_db", false);

    // The _my_lovely_tmp_table should be included because it's explicitly allowed
    ctx.assert_schema_contains_table(&schemas, "_my_lovely_tmp_table", true);

    // The orders table should be included because it matches the allow pattern
    ctx.assert_schema_contains_table(&schemas, "orders", true);

    // The card_numbers table should be excluded because it doesn't match any allow pattern
    ctx.assert_schema_contains_table(&schemas, "card_numbers", false);
}

#[tokio::test]
async fn test_global_filters_include_column() {
    let ctx = TestContext::new_include().await;
    let schemas = ctx.get_schemas().await;

    // The status column should be included because it matches the allow pattern
    ctx.assert_table_contains_column(&schemas, "orders", "status", true);

    // The order_name column should be included because it matches the allow pattern
    ctx.assert_table_contains_column(&schemas, "orders", "order_name", true);

    // The user_id column should be excluded because it doesn't match any allow pattern
    ctx.assert_table_contains_column(&schemas, "orders", "user_id", false);

    // The order_cost column should be excluded because it doesn't match any allow pattern
    ctx.assert_table_contains_column(&schemas, "orders", "order_cost", false);
}

#[tokio::test]
async fn test_global_filters_include_values_country_currency() {
    let ctx = TestContext::new_include().await;

    // Execute a query that would return country and currency codes
    let query =
        "SELECT country, currency FROM test_db._my_lovely_tmp_table GROUP BY country, currency";
    let results = ctx.execute_job_query(query).await;

    // Check that results contain only the allowed country and currency codes
    dbg!("results {:?}", &results);

    // All rows should be included because they match the allow pattern for 2-3 uppercase letters
    assert_eq!(
        results.len(),
        5,
        "Should have exactly 5 country/currency pairs"
    );

    // Check country values
    let expected_countries = vec!["US", "GR", "CY", "UK", "SA"];
    ctx.assert_results_contain_values(&results, "country", &expected_countries);

    // Check currency values
    let expected_currencies = vec!["USD", "EUR", "GBP", "SAR"];
    ctx.assert_results_contain_values(&results, "currency", &expected_currencies);
}

#[tokio::test]
async fn test_global_filters_include_order_status_values() {
    let ctx = TestContext::new_include().await;

    // Execute a query that would return status values
    let query = "SELECT status FROM test_db.orders GROUP BY status";
    let results = ctx.execute_job_query(query).await;

    // Check that results contain only the allowed status values
    dbg!("results {:?}", &results);

    // Only the statuses that match the allow pattern should be included
    assert_eq!(results.len(), 4, "Should have exactly 4 status values");

    // Create a set of expected statuses
    let expected_statuses = vec!["new", "completed", "cancelled", "processing"];

    // Check that all values match expected ones
    ctx.assert_results_contain_values(&results, "status", &expected_statuses);
}

#[tokio::test]
async fn test_global_filters_include_numeric_values() {
    let ctx = TestContext::new_include().await;

    // Execute a query that would return numeric values
    let query = "SELECT COUNT(*) as cnt FROM test_db.orders GROUP BY status";
    let results = ctx.execute_job_query(query).await;

    // Check that results contain only the allowed numeric values (1-3 digits)
    dbg!("results {:?}", &results);

    // Verify all values are numeric and within the allowed range
    for result in &results {
        if let Some(value) = result.get("cnt") {
            let value_str = value.as_str().unwrap();
            assert!(
                value_str.parse::<u32>().is_ok(),
                "Value should be a valid number"
            );
            assert!(value_str.len() <= 3, "Value should be 1-3 digits");
        }
    }
}
