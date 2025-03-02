use super::base::{QueryError, QueryExecutor};
use crate::config::GlobalFilters;
use crate::filters::SqlFilters;
use crate::models::{JobType, Record};
use async_trait::async_trait;
use clickhouse::Client;
use reqwest;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Information about a database column
#[derive(Debug, serde::Serialize)]
pub struct ColumnInfo {
    /// Simplified type name (int, float, string, etc.)
    pub type_name: String,
    /// Number of unique values in the column (if available)
    pub cardinality: Option<u64>,
}

/// Schema information for a database table
#[derive(Debug, serde::Serialize)]
pub struct TableSchema {
    /// Database name
    pub database: String,
    /// Table name
    pub table: String,
    /// Number of rows in the table
    pub row_count: u64,
    /// Map of column names to their information
    pub columns: HashMap<String, ColumnInfo>,
}

/// Configuration for database and table filtering
#[derive(Debug, Clone)]
pub struct FilterConfig {
    /// Databases to exclude from schema discovery
    excluded_databases: HashSet<String>,
    /// Tables to exclude from schema discovery
    excluded_tables: HashSet<String>,
    /// SQL filters from global configuration
    sql_filters: Option<SqlFilters>,
}

impl Default for FilterConfig {
    fn default() -> Self {
        let mut excluded_databases = HashSet::new();
        excluded_databases.insert("system".to_string());
        excluded_databases.insert("INFORMATION_SCHEMA".to_string());
        excluded_databases.insert("information_schema".to_string());

        let excluded_tables = HashSet::new();

        Self {
            excluded_databases,
            excluded_tables,
            sql_filters: None,
        }
    }
}

impl FilterConfig {
    /// Create a new filter config with global filters
    pub fn with_global_filters(global_filters: Option<&GlobalFilters>) -> Result<Self, QueryError> {
        let mut config = Self::default();

        if let Some(global_filters) = global_filters {
            match SqlFilters::new(Some(global_filters)) {
                Ok(filters) => config.sql_filters = Some(filters),
                Err(e) => {
                    return Err(QueryError::ExecutionError(format!(
                        "Failed to create SQL filters: {}",
                        e
                    )))
                }
            }
        }

        Ok(config)
    }

    /// Check if a database should be excluded
    pub fn should_exclude_database(&self, db_name: &str) -> bool {
        // Check built-in exclusions
        if self.excluded_databases.contains(db_name) {
            return true;
        }

        // Check global filters
        if let Some(filters) = &self.sql_filters {
            if filters.should_exclude_database(db_name) {
                return true;
            }
        }

        false
    }

    /// Check if a table should be excluded
    pub fn should_exclude_table(&self, table_name: &str) -> bool {
        // Check built-in exclusions
        if self.excluded_tables.contains(table_name) {
            return true;
        }

        // Check global filters
        if let Some(filters) = &self.sql_filters {
            if filters.should_exclude_table(table_name) {
                return true;
            }
        }

        false
    }

    /// Check if a column should be excluded
    pub fn should_exclude_column(&self, column_name: &str) -> bool {
        if let Some(filters) = &self.sql_filters {
            return filters.should_exclude_column(column_name);
        }

        false
    }

    /// Check if a value should be excluded
    pub fn should_exclude_value(&self, value: &str) -> bool {
        if let Some(filters) = &self.sql_filters {
            return filters.should_exclude_value(value);
        }

        false
    }
}

/// Executor for ClickHouse databases
pub struct ClickhouseExecutor {
    url: String,
    username: String,
    password: String,
    client: Arc<Client>,
    filter_config: FilterConfig,
}

impl ClickhouseExecutor {
    /// Get list of databases from the ClickHouse server
    async fn get_databases(&self) -> Result<Vec<String>, QueryError> {
        let query = "SELECT name FROM system.databases";
        let databases = self
            .client
            .query(query)
            .fetch_all::<String>()
            .await
            .map_err(|e| QueryError::ExecutionError(e.to_string()))?;

        // Apply database filtering
        let filtered_databases = databases
            .into_iter()
            .filter(|db| !self.filter_config.should_exclude_database(db))
            .collect();

        Ok(filtered_databases)
    }

    /// Get list of tables in a database
    async fn get_tables(&self, database: &str) -> Result<Vec<String>, QueryError> {
        let query = format!("SHOW TABLES FROM {}", database);
        let tables = self
            .client
            .query(&query)
            .fetch_all::<String>()
            .await
            .map_err(|e| QueryError::ExecutionError(e.to_string()))?;

        // Apply table filtering
        let filtered_tables = tables
            .into_iter()
            .filter(|table| !self.filter_config.should_exclude_table(table))
            .collect();

        Ok(filtered_tables)
    }

    /// Discover schemas for all databases and tables
    pub async fn discover_schemas(&self) -> Result<Vec<TableSchema>, QueryError> {
        log::debug!("Discovering clickhouse schemas");

        let mut schemas: Vec<TableSchema> = Vec::new();

        // Get list of databases
        let databases = self.get_databases().await.map_err(|e: QueryError| {
            QueryError::ExecutionError(format!("Failed to get databases list: {}", e))
        })?;

        for db in databases {
            log::debug!("Discovering database: {}", db);

            // Get tables for this database
            let tables = self.get_tables(&db).await.map_err(|e| {
                QueryError::ExecutionError(format!(
                    "Failed to get tables for database {}: {}",
                    db, e
                ))
            })?;

            // Process tables in parallel for better performance
            let table_schemas = self.discover_tables(&db, &tables).await?;
            schemas.extend(table_schemas);
        }

        Ok(schemas)
    }

    /// Discover schema information for tables in a database
    async fn discover_tables(
        &self,
        db: &str,
        tables: &[String],
    ) -> Result<Vec<TableSchema>, QueryError> {
        let mut table_futures = Vec::new();
        let mut table_schemas = Vec::new();

        // Create a future for each table
        for table in tables {
            // Convert &str to String to own the data
            let db_owned = db.to_string();
            let table_owned = table.clone();
            let client = self.client.clone();
            let filter_config = self.filter_config.clone();

            table_futures.push(tokio::spawn(async move {
                log::debug!("Discovering table: {}.{}", db_owned, table_owned);
                Self::discover_table_schema(&client, &db_owned, &table_owned, Some(&filter_config))
                    .await
            }));
        }

        // Wait for all table discoveries to complete
        for future in table_futures {
            match future.await {
                Ok(Ok(schema)) => table_schemas.push(schema),
                Ok(Err(e)) => log::error!("Table discovery error: {}", e),
                Err(e) => log::error!("Task join error: {}", e),
            }
        }

        Ok(table_schemas)
    }

    /// Discover schema for a single table
    async fn discover_table_schema(
        client: &Client,
        db: &String,
        table: &String,
        filter_config: Option<&FilterConfig>,
    ) -> Result<TableSchema, QueryError> {
        // Get columns
        let columns_query = format!(
            "SELECT name, type FROM system.columns WHERE database = '{}' AND table = '{}'",
            db, table
        );

        let columns: Vec<(String, String)> = client
            .query(&columns_query)
            .fetch_all()
            .await
            .map_err(|e| QueryError::ExecutionError(e.to_string()))?;

        let mut column_info = HashMap::new();

        // Get cardinality for each column
        for (name, type_) in columns {
            log::debug!("Discovering column: {}.{}.{}", db, table, name);

            // Skip columns that should be excluded based on global filters
            if let Some(filter_config) = filter_config {
                if filter_config.should_exclude_column(&name) {
                    log::debug!("Skipping excluded column: {}.{}.{}", db, table, name);
                    continue;
                }
            }

            let cardinality_query = format!("SELECT uniq({}) FROM {}.{}", name, db, table);

            let cardinality: Option<u64> = match client.query(&cardinality_query).fetch_one().await
            {
                Ok(count) => Some(count),
                Err(e) => {
                    log::warn!(
                        "Failed to get cardinality for {}.{}.{}: {}",
                        db,
                        table,
                        name,
                        e
                    );
                    None
                }
            };

            column_info.insert(
                name,
                ColumnInfo {
                    type_name: simplify_type(&type_),
                    cardinality,
                },
            );
        }

        // Get row count
        let count_query = format!("SELECT count() FROM {}.{}", db, table);
        let row_count = client.query(&count_query).fetch_one().await.map_err(|e| {
            QueryError::ExecutionError(format!(
                "Failed to get row count for {}.{}: {}",
                db, table, e
            ))
        })?;

        Ok(TableSchema {
            database: db.to_string(),
            table: table.to_string(),
            row_count,
            columns: column_info,
        })
    }

    /// Create a new ClickHouse executor with default filter configuration
    pub fn new(host: &str, username: &str, password: &str) -> Result<Self, QueryError> {
        Self::with_global_filters(host, username, password, None)
    }

    /// Create a new ClickHouse executor with global filters
    pub fn with_global_filters(
        host: &str,
        username: &str,
        password: &str,
        global_filters: Option<GlobalFilters>,
    ) -> Result<Self, QueryError> {
        let filter_config = FilterConfig::with_global_filters(global_filters.as_ref())?;

        let client = Client::default()
            .with_url(host)
            .with_user(username)
            .with_password(password)
            .with_database("default");

        Ok(Self {
            client: Arc::new(client),
            url: host.to_string(),
            username: username.to_string(),
            password: password.to_string(),
            filter_config,
        })
    }

    /// Create a new ClickHouse executor with custom filter configuration
    pub fn with_filter_config(
        host: &str,
        username: &str,
        password: &str,
        filter_config: FilterConfig,
    ) -> Result<Self, QueryError> {
        let client = Client::default()
            .with_url(host)
            .with_user(username)
            .with_password(password)
            .with_database("default");

        Ok(Self {
            client: Arc::new(client),
            url: host.to_string(),
            username: username.to_string(),
            password: password.to_string(),
            filter_config,
        })
    }
}

/// Convert ClickHouse type to simplified type name
fn simplify_type(ch_type: &str) -> String {
    if ch_type.starts_with("Int") || ch_type.starts_with("UInt") {
        "int".into()
    } else if ch_type.starts_with("Float") {
        "float".into()
    } else if ch_type == "Bool" || ch_type == "Boolean" {
        "bool".into()
    } else if ch_type == "Date" {
        "date".into()
    } else if ch_type.starts_with("DateTime") {
        "datetime".into()
    } else {
        "string".into()
    }
}

#[async_trait]
impl QueryExecutor for ClickhouseExecutor {
    async fn discover_schemas(&self) -> Result<Vec<TableSchema>, QueryError> {
        self.discover_schemas().await
    }

    async fn execute_ts(&self, query: &str) -> Result<Vec<Record>, QueryError> {
        log::debug!("Executing time series query: {}", query);

        let rows: Vec<Record> = self
            .client
            .query(query)
            .fetch_all::<Record>()
            .await
            .map_err(|e| {
                log::error!("Query execution error: {}", e);
                QueryError::ExecutionError(e.to_string())
            })?;

        log::debug!("Query executed successfully, returned {} rows", rows.len());

        if log::log_enabled!(log::Level::Trace) {
            log::trace!("Query results: {:?}", &rows);
        }

        Ok(rows)
    }

    /// Filter job results based on global filters
    fn filter_job_results(&self, rows: Vec<JobType>) -> Vec<JobType> {
        if self.filter_config.sql_filters.is_none() {
            return rows;
        }

        let mut filtered_rows = Vec::new();

        for row in rows {
            let mut should_include_row = true;

            // Check each value in the row
            for (key, value) in &row {
                // Check if column should be excluded
                if self.filter_config.should_exclude_column(key) {
                    should_include_row = false;
                    break;
                }

                // Check if value should be excluded
                if let Some(value_str) = value.as_str() {
                    // Remove all spaces from the value before checking
                    let trimmed_value = value_str.replace(" ", "");
                    if self.filter_config.should_exclude_value(&trimmed_value) {
                        should_include_row = false;
                        break;
                    }
                }
            }

            // Only include the row if it passed all filters
            if should_include_row {
                filtered_rows.push(row);
            }
        }

        filtered_rows
    }

    async fn execute_job(&self, query: &str) -> Result<Vec<JobType>, QueryError> {
        log::debug!("Executing job query: {}", query);

        // Use reqwest client for JSONEachRow format
        let client = reqwest::Client::new();
        let full_query = format!("{} FORMAT JSONEachRow", query);

        // Send request to ClickHouse server
        let response = client
            .post(self.url.clone())
            .basic_auth(self.username.clone(), Some(self.password.clone()))
            .body(full_query)
            .send()
            .await
            .map_err(|e| {
                log::error!("HTTP request error: {}", e);
                QueryError::ConnectionError(e.to_string())
            })?
            .error_for_status()
            .map_err(|e| {
                log::error!("HTTP response error: {}", e);
                QueryError::ExecutionError(e.to_string())
            })?;

        // Parse response text
        let text = response
            .text()
            .await
            .map_err(|e| QueryError::ExecutionError(e.to_string()))?;

        // Parse each line as a JSON object
        let rows_res: Result<Vec<HashMap<String, Value>>, _> = text
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| {
                serde_json::from_str(line).map_err(|e| {
                    log::error!("JSON parsing error for line: {}", line);
                    e
                })
            })
            .collect();

        let mut rows = rows_res.map_err(|e| QueryError::ExecutionError(e.to_string()))?;

        // Apply filters to the result rows
        if self.filter_config.sql_filters.is_some() {
            rows = self.filter_job_results(rows);
        }

        log::debug!(
            "Job query executed successfully, returned {} rows",
            rows.len()
        );

        Ok(rows)
    }

    async fn connect(&mut self) -> Result<(), QueryError> {
        log::debug!("Testing connection to ClickHouse server at {}", self.url);

        match self.execute_ts("SELECT 1").await {
            Ok(_) => {
                log::info!("Successfully connected to ClickHouse server");
                Ok(())
            }
            Err(e) => {
                log::error!("Failed to connect to ClickHouse server: {}", e);
                Err(e)
            }
        }
    }
}
