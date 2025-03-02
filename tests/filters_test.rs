use std::path::Path;
use tsight_agent::config::{Config, GlobalFilters, SqlFilterRules};
use tsight_agent::filters::SqlFilters;

#[test]
fn test_sql_filters() {
    // Create test filter rules
    let mut exclude_rules = SqlFilterRules::default();
    exclude_rules.database_regexes = Some(vec!["^test_.*".to_string(), "^_.*".to_string()]);

    let mut allow_rules = SqlFilterRules::default();
    allow_rules.database_regexes = Some(vec!["^prod_.*".to_string()]);

    let mut global_filters = GlobalFilters::default();
    global_filters.sql_filters_exclude = Some(vec![exclude_rules]);
    global_filters.sql_filters_allow = Some(vec![allow_rules]);

    // Create SQL filters
    let sql_filters = SqlFilters::new(Some(&global_filters)).unwrap();

    // Test database filtering
    assert!(sql_filters.should_exclude_database("test_db"));
    assert!(sql_filters.should_exclude_database("_internal"));
    assert!(!sql_filters.should_exclude_database("prod_db"));
    assert!(sql_filters.should_exclude_database("staging_db")); // Not in allow list
}

#[test]
fn test_load_config_with_filters() {
    // Load the test config file
    let config_path = Path::new("tests/test_configs/combined_sql_filters_config.yaml");
    if !config_path.exists() {
        panic!(
            "Test config file doesn't exist at {}",
            config_path.display()
        );
    }

    let config = Config::load(config_path).unwrap();
    assert!(config.global_filters.is_some());

    if let Some(global_filters) = &config.global_filters {
        assert!(global_filters.sql_filters_exclude.is_some());
        assert!(global_filters.sql_filters_allow.is_some());

        // Create SQL filters from the loaded config
        let sql_filters = SqlFilters::new(Some(global_filters)).unwrap();

        // Test database filtering
        assert!(sql_filters.should_exclude_database("test_db"));
        assert!(sql_filters.should_exclude_database("dev_db"));
        assert!(sql_filters.should_exclude_database("_internal"));
        assert!(!sql_filters.should_exclude_database("logs"));

        // Test table filtering
        assert!(sql_filters.should_exclude_table("tmp_orders"));
        assert!(sql_filters.should_exclude_table("secret_users"));
        assert!(!sql_filters.should_exclude_table("orders"));

        // Test column filtering
        assert!(sql_filters.should_exclude_column("email"));
        assert!(sql_filters.should_exclude_column("first_name"));
        assert!(sql_filters.should_exclude_column("last_name"));
        assert!(!sql_filters.should_exclude_column("status"));
        assert!(!sql_filters.should_exclude_column("country"));

        // Test value filtering
        assert!(sql_filters.should_exclude_value("user@example.com"));
        assert!(sql_filters.should_exclude_value("4111111111111111")); // Visa card pattern
        assert!(!sql_filters.should_exclude_value("US"));
        assert!(!sql_filters.should_exclude_value("pending"));
    }
}
