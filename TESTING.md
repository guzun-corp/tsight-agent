# Testing Guide

## Running Tests

Run all tests with:

```sh
cargo test
```

Run a specific test with:

```sh
cargo test test_name
```

Run tests for a specific module:

```sh
cargo test --package tsight-agent --test clickhouse_filters_test
```

## Code Coverage

To generate code coverage reports:

```sh
# Install the coverage tool (only needed once)
cargo install cargo-llvm-cov

# Generate and view coverage report
cargo llvm-cov

# Generate HTML report
cargo llvm-cov --html

# Generate coverage for a specific test
cargo llvm-cov --test clickhouse_filters_test
```

## Mock Debugging

When working with mockito for HTTP mocking:

1. Add this line to your test to enable logging:

   ```rust
   let _ = env_logger::try_init();
   ```

2. Run tests with debug logging enabled:

   ```sh
   RUST_LOG=mockito=debug cargo test
   ```

3. To see all HTTP interactions:
   ```sh
   RUST_LOG=mockito=trace cargo test
   ```

## Integration Test Tips

- Mock external services with mockito
- Use test configs in `tests/test_configs/` directory
- For database tests, consider using Docker containers
