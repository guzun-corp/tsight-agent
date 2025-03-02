use anyhow::{anyhow, Context, Result};
use log::{error, info};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use tsight_agent::agent::{discover_and_submit_schemas, initialize_agents};
use tsight_agent::client::ServerClient;
use tsight_agent::config::Config;

/// Get the platform-specific default config path
fn get_default_config_path() -> PathBuf {
    if cfg!(target_os = "linux") {
        // Linux: /home/username/.config/tsight_agent/config.yaml
        let home = env::var("HOME").unwrap_or_else(|_| String::from("/home/user"));
        PathBuf::from(home).join(".config").join("tsight_agent").join("config.yaml")
    } else if cfg!(target_os = "macos") {
        // macOS: ~/Library/Application Support/tsight_agent/config.yaml
        let home = env::var("HOME").unwrap_or_else(|_| String::from("/Users/user"));
        PathBuf::from(home).join("Library").join("Application Support").join("tsight_agent").join("config.yaml")
    } else {
        // Default to local config.yaml for other platforms (including Windows)
        PathBuf::from("config.yaml")
    }
}

/// Ensure the configuration directory exists
fn ensure_config_dir_exists() -> Result<()> {
    let default_path = get_default_config_path();
    let config_dir = default_path.parent().ok_or_else(|| 
        anyhow!("Could not determine parent directory of config path")
    )?;
    
    if !config_dir.exists() {
        info!("Creating configuration directory: {}", config_dir.display());
        fs::create_dir_all(config_dir).context("Failed to create configuration directory")?;
    }
    
    Ok(())
}

/// Load configuration from the default paths
pub fn load_config() -> Result<Config> {
    // First try platform-specific default location
    let default_path = get_default_config_path();
    
    if default_path.exists() {
        info!("Using configuration from system path: {}", default_path.display());
        return load_config_from_path(&default_path);
    }
    
    // Then try local config.yaml
    let local_path = Path::new("config.yaml");
    if local_path.exists() {
        info!("Using configuration from local path: {}", local_path.display());
        return load_config_from_path(local_path);
    }
    
    // Ensure the config directory exists for future use
    if let Err(e) = ensure_config_dir_exists() {
        info!("Note: {}", e);
    }
    
    // No config found, return error with expected location
    Err(anyhow!("Configuration file not found. Expected at: {}", default_path.display()))
}

/// Load configuration from a specific path
pub fn load_config_from_path(path: &Path) -> Result<Config> {
    info!("Loading configuration from {:?}...", path);
    let config = Config::load(path).context(
        "Failed to load config file. Please ensure it exists and contains valid configuration",
    )?;
    info!("Configuration loaded successfully from {:?}", path);
    Ok(config)
}

/// Start schema discovery process
pub async fn start_schema_discovery(config: &Config) -> Result<()> {
    info!("Starting schema discovery...");
    let server_client = ServerClient::new(
        config.server.api_key.clone(),
        config.server.server_url.clone(),
    );
    let datasources = config.datasources.clone();
    let global_filters = config.global_filters.clone();

    discover_and_submit_schemas(&datasources, &server_client, global_filters).await
}

#[tokio::main]
async fn main() {
    env_logger::init();
    info!("Starting TSight Agent");

    // Load configuration
    let config = match load_config() {
        Ok(config) => {
            info!("Configuration loaded successfully");
            config
        }
        Err(e) => {
            error!("{:#}", e);
            std::process::exit(1);
        }
    };

    // Initialize all agents
    let (hp_agent, job_agent, main_agent) = initialize_agents(&config);

    // Spawn high priority queue agent
    tokio::spawn(async move { hp_agent.run().await });

    // Spawn job processing agent
    tokio::spawn(async move { job_agent.run().await });

    // Start schema discovery
    tokio::spawn(async move {
        if let Err(e) = start_schema_discovery(&config).await {
            error!("Failed to discover schemas: {:#}", e);
        }
    });

    info!("Starting main processing loop");
    main_agent.run().await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_load_config_from_path() {
        // Create a temporary directory
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.yaml");

        // Create a simple test config
        let config_content = r#"
server:
  api_key: test_key
  server_url: http://test-server.com
datasources:
  - name: test_source
    source_type: Clickhouse
    hosts:
      - http://localhost:8123
    username: default
    password: ""
"#;
        fs::write(&config_path, config_content).unwrap();

        // Test loading the config
        let config = load_config_from_path(&config_path).unwrap();
        assert_eq!(config.server.api_key, "test_key");
        assert_eq!(config.server.server_url, "http://test-server.com");
        assert_eq!(config.datasources.len(), 1);
        assert_eq!(config.datasources[0].name, "test_source");
    }
    
    #[test]
    fn test_get_default_config_path() {
        // This test just ensures the function returns a path
        let path = get_default_config_path();
        assert!(path.to_str().is_some());
        
        // The path should end with config.yaml
        assert!(path.to_str().unwrap().ends_with("config.yaml"));
    }
}
