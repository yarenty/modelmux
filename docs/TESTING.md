# Testing Guide for ModelMux

This document describes the test suite for ModelMux and how to run tests.

## Test Structure

ModelMux includes comprehensive tests organized into several categories:

### 1. Unit Tests

Unit tests are located alongside the source code in `src/` directories with `#[cfg(test)]` modules.

**Location**: `src/*.rs` (inline test modules)

**Coverage**:
- Server module: Client detection, streaming behavior
- Config module: (tests in `tests/config_tests.rs`)
- Error handling

**Run**: `cargo test`

### 2. Integration Tests

Integration tests are in the `tests/` directory and test the full application.

**Files**:
- `tests/cli_tests.rs` - CLI argument parsing (`--version`, `--help`)
- `tests/config_tests.rs` - Configuration loading and validation
- `tests/integration_tests.rs` - Application initialization

**Run**: `cargo test --test <test_file>` or `cargo test` for all

### 3. Homebrew Formula Tests

The Homebrew formula (`modelmux.rb`) includes tests that verify:
- Binary installation
- `--version` and `--help` flags work
- Configuration parsing works correctly

**Run**: `brew test modelmux` (after installation)

## Running Tests

### Run All Tests
```bash
cargo test
```

### Run Specific Test Suite
```bash
# CLI tests
cargo test --test cli_tests

# Config tests
cargo test --test config_tests

# Integration tests
cargo test --test integration_tests
```

### Run with Output
```bash
cargo test -- --nocapture
```

### Run Specific Test
```bash
cargo test test_version_flag
```

## Test Coverage

### CLI Tests (`tests/cli_tests.rs`)
- ✅ `--version` flag output
- ✅ `-V` short version flag
- ✅ `--help` flag output
- ✅ `-h` short help flag

### Config Tests (`tests/config_tests.rs`)
- ✅ Required environment variable validation
- ✅ Default values (port, log level, streaming mode)
- ✅ Custom configuration parsing
- ✅ Invalid configuration error handling
- ✅ LogLevel and StreamingMode enum conversions
- ✅ URL building for streaming/non-streaming

### Server Tests (`src/server.rs`)
- ✅ Client detection (RustRover, IntelliJ, browsers, etc.)
- ✅ Streaming behavior determination
- ✅ Problematic client detection

### Integration Tests (`tests/integration_tests.rs`)
- ✅ Application creation with valid config
- ✅ Error handling for invalid config

## Test Requirements

### Environment Variables

Some tests require environment variables to be set. The config tests handle this by:
1. Setting minimal valid test values
2. Cleaning up after tests
3. Testing both missing and invalid values

### Dependencies

Test dependencies are listed in `Cargo.toml` under `[dev-dependencies]`:
- `tokio-test` - Async test utilities
- `base64` - For testing base64 encoding/decoding
- `reqwest` - For HTTP client testing

## CI/CD Testing

### GitHub Actions

Tests should run automatically on:
- Pull requests
- Pushes to main branch
- Manual workflow dispatch

### Homebrew CI

The Homebrew formula tests run automatically when:
- Formula is updated
- New version is released
- PR is submitted to homebrew-core

## Writing New Tests

### Unit Test Example

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_my_function() {
        assert_eq!(my_function(), expected_value);
    }
}
```

### Integration Test Example

```rust
#[tokio::test]
async fn test_endpoint() {
    let app = create_test_app().await;
    // Test the endpoint
}
```

### Test Helpers

Common test helpers are available in test modules:
- `create_test_config()` - Creates minimal valid configuration
- `set_minimal_config()` - Sets required environment variables

## Test Best Practices

1. **Isolation**: Each test should be independent and not rely on other tests
2. **Cleanup**: Clean up environment variables after tests
3. **Naming**: Use descriptive test names that explain what is being tested
4. **Assertions**: Use clear assertion messages
5. **Async**: Use `#[tokio::test]` for async tests
6. **Mocking**: Mock external dependencies when possible

## Known Limitations

1. **CLI Tests**: Currently use `cargo run` which requires the project to be built. In CI/Homebrew, the binary will be pre-built.
2. **Integration Tests**: Some tests require a running server. Consider using `axum-test` or similar for better integration testing.
3. **Auth Tests**: Full authentication testing requires valid GCP credentials, which are not included in tests.

## Future Improvements

- [ ] Add end-to-end tests with test server
- [ ] Add converter module tests
- [ ] Add performance benchmarks
- [ ] Add fuzz testing for input validation
- [ ] Add property-based tests for format conversion
