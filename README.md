# TSight Agent

## Table of Contents

- [Overview](#overview)
- [Features](#features)
- [Architecture](#architecture)
- [Getting Started](#getting-started)
  - [Prerequisites](#prerequisites)
  - [One-row installation](#one-row-installation)
  - [Install from source code](#install-from-source-code)
- [Configuration](#configuration)
  - [Basic Configuration](#basic-configuration)
  - [Data Source Support](#data-source-support)
  - [Schema Discovery](#schema-discovery)
  - [Filtering Options](#filtering-options)
  - [Example Configurations](#example-configurations)
- [Security](#security)
- [Contributing](#contributing)
- [License](#license)
- [Support](#support)

## Overview

TSight Agent is a client-side component of the [TSight.app](https://tsight.app) platform, designed to enable anomaly detection and observability for your systems. The agent runs on your infrastructure and securely connects to the TSight platform to provide real-time data analysis and anomaly detection capabilities.

## Features

- **Secure Data Collection**: Connects to your data sources while keeping your data secure within your infrastructure
- **Filtering Capabilities**: Provides robust data filtering to control what data is processed using include/exclude patterns
- **Schema Discovery**: Automatically discovers and maps your data source schemas to provide intelligent monitoring
- **Job Processing**: Handles both observation and job processing tasks
- **High Priority Queue**: Supports prioritized processing for critical monitoring needs

## Architecture

The TSight Agent operates with the following components:

- **Observation Agent**: Processes time series data for anomaly detection
- **Job Agent**: Handles scheduled and on-demand data processing tasks
- **Server Client**: Manages secure communication with the TSight platform
- **Executors**: Connect to and query your data sources
- **Filters**: Apply data filtering rules to protect sensitive information

## Getting Started

### Prerequisites

- Access to a [TSight.app](https://tsight.app) account and [API key](https://tsight.app/settings/api-keys)
- One or more supported data sources
- Rust 1.84 or higher (Optional: If you want to build from source code, see [Install from source code](#install-from-source-code))

### One-row installation

WIP

### Install from source code

1. Clone the repository:

   ```
   git clone https://github.com/tsightapp/tsight-agent.git
   cd tsight-agent
   ```

2. Build the agent:

   ```
   cargo build --release
   ```

3. Configure the agent (see Configuration section)

4. Run the agent:
   ```
   ./target/release/tsight-agent
   ```

## Configuration

Create a configuration file with your TSight [API key](https://tsight.app/settings/api-keys), server URL, and data source information:

### Configuration File Location

The agent looks for configuration in the following locations (in order):

1. **Linux**: `/home/username/.config/tsight_agent/config.yaml`
2. **macOS**: `~/Library/Application Support/tsight_agent/config.yaml`
3. **Local directory**: `./config.yaml` (fallback for all platforms)

The agent will automatically create the necessary directories if they don't exist.

### Basic Configuration

```yaml
server:
  api_key: "your-api-key"
  server_url: "https://api.tsight.app"

datasources:
  - name: "my_clickhouse"
    source_type: "clickhouse"
    hosts:
      - "http://localhost:8123"
    username: "default"
    password: ""
    database: "default"
```

### Data Source Support

The TSight Agent currently supports the following data sources:

- **ClickHouse**: Full support with schema discovery and filtering
- **MySQL**: Coming soon
- **PostgreSQL**: Coming soon
- **Prometheus**: Coming soon

### Schema Discovery

When you start the agent, it automatically discovers the schema of your data sources, including:

- Databases
- Tables
- Columns and their data types
- Row counts
- Cardinality of each column

This information is used to provide intelligent monitoring and anomaly detection tailored to your specific data structures.

### Filtering Options

You can use either include or exclude filtering methods (or both, though using both can make rules harder to understand):

#### Exclude Filtering Example

This configuration excludes system databases and tables starting with underscore:

```yaml
global_filters:
  sql_filters_exclude:
    - database_regexes:
        - "^test"
    - table_regexes:
        # Exclude tables that start with "_"
        - "^_.*"
    - column_name_regexes:
        # Exclude column names contains "secret" or "password"
        - "password"
        - "secret"
    - column_value_regexes:
        # Exclude values that match typical email patterns
        - "^[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\\.[A-Za-z]{2,}$"
```

#### Include Filtering Example

This configuration only includes specific databases, tables:

```yaml
global_filters:
  sql_filters_allow:
    - database_regexes:
        - "^production$"
        - "^analytics$"
    - table_regexes:
        - "^users$"
        - "^events$"
```

### Example Configurations

For more detailed configuration examples, check out our test configuration files:

- [Exclude-only SQL Filters](tests/test_configs/exclude_only_sql_filters_config.yaml) - Example of using exclude patterns
- [Include-only SQL Filters](tests/test_configs/include_only_sql_filters_config.yaml) - Example of using include patterns
- [Combined Filters](tests/test_configs/combined_sql_filters_config.yaml) - Example of using both include and exclude patterns

## Security

The TSight Agent is designed with security in mind:

- All data processing happens on your infrastructure
- Only aggregated results are sent to the TSight platform
- Data filtering allows you to exclude sensitive information
- Communication with the TSight platform is encrypted
- API key authentication ensures secure access

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the [MIT License](LICENSE).

## Support

For support, please contact [ayuguzun@gmail.com](mailto:ayuguzun@gmail.com)
