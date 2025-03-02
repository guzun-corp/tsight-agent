use clickhouse::Client;
use std::path::PathBuf;
use tsight_agent::config::Config;
use tsight_agent::models::DataSource;

#[tokio::test]
async fn test_clickhouse_connection() {
    let config_path: PathBuf = PathBuf::from("tests/test_configs/simple_config.yaml");
    let config = Config::load(&config_path);
    assert!(config.is_ok(), "Failed to load test config");
    let config = config.unwrap();

    let datasource: &DataSource = &config.datasources[0];

    let client = Client::default()
        .with_url(datasource.hosts[0].as_str())
        .with_user(datasource.username.as_str())
        .with_password(datasource.password.as_str());

    let result = client.query("SELECT 1").execute().await;
    assert!(result.is_ok(), "Failed to connect to ClickHouse");
}
