use crate::client::ServerClient;
use crate::config::GlobalFilters;
use crate::models::DataSource;
use anyhow::Result;
use log::{error, info};

use crate::executors::create_executor;

/// Discover schemas for a single datasource and submit them to the server
pub async fn discover_datasource(
    datasource: &DataSource,
    server_client: &ServerClient,
    global_filters: Option<GlobalFilters>,
) -> Result<()> {
    info!("Discovering schemas for datasource: {}", datasource.name);
    server_client
        .add_datasource(&datasource.name, &datasource.source_type.to_string())
        .await?;

    let mut executor = create_executor(datasource, global_filters).await?;
    executor.connect().await?;

    let schemas = executor.discover_schemas().await?;
    info!("Discovering schemas for datasource: {}", datasource.name);
    server_client
        .submit_schemas(&datasource.name, schemas)
        .await?;

    info!(
        "Successfully submitted schemas for datasource: {}",
        datasource.name
    );
    Ok(())
}

/// Discover and submit schemas for all datasources
pub async fn discover_and_submit_schemas(
    datasources: &[DataSource],
    server_client: &ServerClient,
    global_filters: Option<GlobalFilters>,
) -> Result<()> {
    for datasource in datasources {
        let res = discover_datasource(datasource, server_client, global_filters.clone()).await;
        if res.is_err() {
            error!(
                "Failed to discover schemas for datasource: {}",
                datasource.name
            );
        }
    }
    Ok(())
}
