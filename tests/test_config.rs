use std::path::PathBuf;
use tsight_agent::config::Config;
use tsight_agent::models::{DataSource, DataSourceType};

#[tokio::test]
async fn test_config_loading() {
    let config_path: PathBuf = PathBuf::from("tests/test_configs/simple_config.yaml");
    let config: Result<Config, _> = Config::load(&config_path);

    assert!(config.is_ok(), "Failed to load test config");
    let config = config.unwrap();

    assert_eq!(config.server.api_key, "test-api-key");
    assert_eq!(config.datasources.len(), 1);

    let datasource: &DataSource = &config.datasources[0];
    assert_eq!(datasource.name, "test_clickhouse");
    assert_eq!(datasource.source_type, DataSourceType::Clickhouse);
    assert_eq!(datasource.username, "test_user");
    assert_eq!(datasource.password, "test_password");
    assert_eq!(datasource.hosts.len(), 1);
    assert_eq!(datasource.hosts[0], "http://localhost:8123");
}
