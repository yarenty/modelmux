# Changelog

All notable changes to ModelMux will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.3.5] - 2026-07-13

### Added

- **10 URL construction tests** (`tests/url_tests.rs`) proving correct Vertex AI endpoint
  generation for every scenario: standard region, global region, named model inheritance,
  multiple named models, publisher override, explicit entry URL, parent URL override,
  unknown model fallback, end-to-end `Config` routing, and default model via env vars.
  Run with `cargo test --no-fail-fast --test url_tests`.

---

## [1.3.4] - 2026-07-13

### Fixed

- **`global` region now correctly produces `aiplatform.googleapis.com`** host in all URL
  construction paths. Added `vertex_host(region)` helper: `global` → `aiplatform.googleapis.com`,
  anything else → `{region}-aiplatform.googleapis.com`. Applied uniformly to default model,
  env-var model, and all `[[vertex.models]]` entries — no special cases needed in config.
- Removed the `ParsedVertexUrl` struct and URL-parsing fallback; no longer needed now that
  the host is always derived directly and correctly from the `region` field.

---

## [1.3.3] - 2026-07-13

### Fixed

- **Root cause of all multi-model routing failures found and fixed**: the `if let … &&
  !name.is_empty() && let …` chain in `build_predict_url_for_model` (same Edition 2024
  `let`-chain issue as `resolve_predict_url_and_model` in 1.3.1) was silently falling
  through to the default URL for **every** named model request. Rewritten as explicit
  nested `if let` blocks. This was why all named model entries were routing to the default
  URL regardless of which model the client requested.
- Audited all other `&& let` chains in the codebase — the remaining ones in `server.rs`
  and `converter` only chain `let` bindings with no boolean expression in between and are
  not affected.

---

## [1.3.2] - 2026-07-13

### Fixed

- **Multi-model URL construction is now fully structural** — no more string surgery on the
  parent `url`. When `[[vertex.models]]` entries need a URL, all fields (host, project,
  location, publisher) are now resolved by parsing the parent `[vertex].url` into its
  components, then assembling a clean URL with the entry's model ID. This means:
  - Non-standard hosts like `aiplatform.googleapis.com` (global region) are preserved correctly.
  - Model entries with or without `@version` suffixes work regardless of what the parent URL contains.
  - No more fragile `rfind('/')` or `/models/` substring splits.
- Added `ParsedVertexUrl` helper struct and `parse_vertex_url()` to extract host/project/
  location/publisher from any well-formed Vertex AI resource URL.

---

## [1.3.1] - 2026-07-13

### Fixed

- **Default model URL ignored `[vertex].url`**: the `if let … && let … &&` chain in
  `resolve_predict_url_and_model` silently fell through when both `url` and individual
  fields (`project`, `region`, etc.) were set in `[vertex]`, causing the field-based
  builder to run and produce an invalid URL (e.g. `https://global-aiplatform.googleapis.com/…`).
  Rewritten as explicit nested `if let` blocks — `url` now correctly takes priority.
- **Debug logging**: added `DEBUG`-level log lines showing which URL-resolution branch is
  taken at startup, and which model entry is matched per request. Run with
  `log_level = "debug"` to see them.

---

## [1.3.0] - 2026-07-13

### Added

- **`modelmux logs`**: new command to inspect log files without hunting for the directory.
  - Lists all log files (newest first) with sizes.
  - Prints the last 50 lines of the current log.
  - `modelmux logs -f` / `--follow` tails the log live (like `tail -f`), polling every 200 ms.
- **Doctor / validate show all configured models**: `modelmux doctor`, `modelmux validate`,
  and `modelmux config show` now print every model alias and its resolved Vertex AI endpoint,
  making it easy to verify multi-model routing at a glance.

### Fixed

- **Multi-model routing with `url` override** (`[[vertex.models]]`): when the parent `[vertex]`
  block uses `url` instead of individual region/project/location fields, named model entries
  now correctly inherit the parent URL and substitute only the model ID at the end of the path.
  Previously the field-based builder was used, producing an invalid URL (e.g.
  `https://global-aiplatform.googleapis.com/...`) and a 500 error.
- **Publisher override triggers full URL rebuild**: if a `[[vertex.models]]` entry sets
  `publisher` (or any structural field), a full URL is rebuilt from parts so the publisher
  segment is correct, rather than blindly swapping the model ID in the parent URL.

### URL resolution priority for `[[vertex.models]]` entries

```
1. entry.url set                    → use directly
2. entry overrides publisher/region/project/location → rebuild full URL from parts
3. parent [vertex].url set          → swap model ID at end of parent URL
4. fallback                         → build from parent fields + entry model ID
```

---

## [1.2.0] - 2026-07-13

### Added

- **Multi-model routing**: configure multiple Vertex AI models under `[[vertex.models]]`;
  clients select a model by name via the `"model"` field in their OpenAI request.
  Unset fields on each entry inherit from the parent `[vertex]` block (project, region,
  location, publisher) — only `name` and `model` are required per entry.
- **`/v1/models` lists all configured models** — previously returned only the default model.
- **`publisher` override per model entry** — each `[[vertex.models]]` entry can specify its
  own `publisher`, `region`, `project`, `location`, or `url` when routing to a different
  backend than the default.

### Example config

```toml
[vertex]
project   = "my-project"
region    = "europe-west1"
location  = "europe-west1"
publisher = "anthropic"
model     = "claude-sonnet-4@20250514"

[[vertex.models]]
name  = "claude-opus"
model = "claude-opus-4@20250514"

[[vertex.models]]
name  = "claude-sonnet"
model = "claude-sonnet-4@20250514"
```

---

## [1.1.0] - 2026-05-23

### Changed

#### Predictable macOS Configuration Paths
- **`~/.config/modelmux/` on macOS too**: ModelMux now stores its configuration
  under `~/.config/modelmux/` on macOS (XDG-style), matching Linux. No more
  hunting through `~/Library/Application Support/com.SkyCorp.modelmux/`.
- **New defaults on macOS**:
  - Config: `~/.config/modelmux/config.toml`
  - Service account: `~/.config/modelmux/service-account.json`
  - Cache: `~/.cache/modelmux/`
  - Data: `~/.local/share/modelmux/`
- **System-wide config on macOS** moved from `/Library/Preferences/modelmux/`
  to `/etc/modelmux/`, for consistency with Linux.
- Linux and Windows defaults are unchanged.

### Added

#### Automatic, Idempotent macOS Config Migration
- On startup, ModelMux now auto-migrates any existing configuration from the
  legacy `~/Library/Application Support/com.SkyCorp.modelmux/` (or
  `…/modelmux/`) into `~/.config/modelmux/`:
  - **Idempotent**: short-circuits once `~/.config/modelmux/config.toml`
    exists; safe to call on every startup.
  - **Non-destructive**: never overwrites a file that already exists in the
    new location. Conflicting files stay at the legacy path and the user is
    told about them once.
  - **Path-aware**: rewrites absolute references to the legacy directory
    inside `config.toml` (e.g. `service_account_file = ".../Library/Application
    Support/com.SkyCorp.modelmux/service-account.json"`) so configs keep
    working without manual editing.
  - **Best-effort cleanup**: empties and removes the legacy directory once
    everything has been moved.
- The migration prints a single, clear stderr report on the run that performs
  the move and is silent on every subsequent run.
- Implementation lives in `src/config/migration.rs`, covered by unit tests
  for the success path, idempotent no-op, no-clobber behaviour, and the
  empty-legacy case.

#### Rotating log files
- **Rotating log files** via `tracing-appender`. Logs go to stdout AND to a
  daily-rotating file in the OS-conventional per-user log directory
  (`~/Library/Logs/modelmux/` on macOS, `~/.local/state/modelmux/` on Linux
  per XDG `$XDG_STATE_HOME`, `%LOCALAPPDATA%/modelmux/Logs/` on Windows),
  keeping the last **30 files** (≈ last month). Fixes unbounded log growth
  under `brew services` — the Homebrew formula no longer redirects stdout to
  a single growing `var/log/modelmux.log`.

#### Legacy macOS Fallback in the Loader
- `with_user_config` keeps a safety-net fallback that reads from the legacy
  macOS path if migration was skipped or failed.

---

## [1.0.0] - 2026-02-17

### Added

#### Configuration System
- **Multi-layered configuration hierarchy**: CLI args > env vars > user config > system config > defaults
- **Platform-native directories**: Uses XDG Base Directory Specification on Linux, standard directories on macOS/Windows
  - Linux/Unix: `~/.config/modelmux/config.toml`
  - macOS: `~/Library/Application Support/modelmux/config.toml`
  - Windows: `%APPDATA%/modelmux/config.toml`
- **TOML configuration format**: Human-readable, industry-standard configuration files
- **Secure service account handling**: File-based storage with proper permissions (chmod 600)
- **Path expansion**: Support for tilde (`~`) and environment variable expansion

#### CLI Configuration Management
- **`modelmux config init`**: Interactive configuration setup wizard
- **`modelmux config show`**: Display current configuration from all sources
- **`modelmux config validate`**: Comprehensive configuration validation
- **`modelmux config edit`**: Open configuration file in default editor

#### New Dependencies
- **`directories = "5.0"`**: Cross-platform directory resolution
- **`toml = "0.8"`**: TOML parsing and serialization
- **`shellexpand = "3.1"`**: Path expansion with environment variables

### Changed

#### Configuration Structure
- **Structured configuration**: Replaced flat environment variables with organized TOML sections:
  ```toml
  [server]
  port = 3000
  log_level = "info"
  
  [auth]
  service_account_file = "~/.config/modelmux/service-account.json"
  
  [streaming]
  mode = "auto"
  ```
- **Modular architecture**: Clean separation of concerns across config modules:
  - `config/mod.rs`: Main config types and public API
  - `config/loader.rs`: Multi-layered configuration loading
  - `config/paths.rs`: Platform-native path resolution
  - `config/validation.rs`: Comprehensive validation
  - `config/cli.rs`: Interactive CLI commands

#### Security Improvements
- **File-based credentials**: Service accounts stored as JSON files instead of base64 environment variables
- **Permission validation**: Automatic checking of file permissions for security
- **Secure defaults**: Proper file permissions set by default

#### Backward Compatibility
- **Legacy environment variables**: Still supported with deprecation warnings
- **Graceful migration**: Existing `.env` usage continues to work
- **Progressive adoption**: Users can migrate at their own pace

### Deprecated

- **`.env` file configuration**: Use TOML configuration files instead
- **GCP_SERVICE_ACCOUNT_KEY environment variable**: Use `service_account_file` or `service_account_json` in config
- **Flat environment variables**: Use structured TOML configuration

### Technical Improvements

#### Architecture
- **SOLID principles**: Enhanced modular design with clear separation of concerns
- **Builder pattern**: Flexible configuration loading with method chaining
- **Type safety**: Full compile-time validation of configuration structure
- **Error handling**: Comprehensive error messages with actionable suggestions

#### Testing
- **31 passing tests**: Comprehensive unit and integration test coverage
- **Cross-platform testing**: Configuration loading, path resolution, file handling
- **CLI testing**: Interactive commands and validation scenarios
- **Security testing**: File permission and credential validation

#### Developer Experience
- **Professional CLI**: Industry-standard configuration management interface
- **Clear documentation**: Module-level documentation with examples
- **Helpful errors**: Detailed error messages with fix suggestions
- **Example configurations**: Well-commented TOML examples

---

## [0.5.0] - 2026-02-10

### Added

- **LLM provider abstraction**: Configuration is driven by `LLM_PROVIDER`; only the selected backend is loaded.
- **`LlmProviderBackend` trait**: All backends implement `id()`, `build_request_url()`, `display_model_name()`, `auth_strategy()`.
- **Vertex provider**: Supports either full URL override (`LLM_URL`) or Google-docs-style vars (`VERTEX_REGION`, `VERTEX_PROJECT`, `VERTEX_LOCATION`, `VERTEX_PUBLISHER`, `VERTEX_MODEL_ID`).
- **OpenAI-compatible provider stub**: Template for future Mistral/OpenAI-compatible backends (not yet implemented).
- **`AuthStrategy`**: `GcpOAuth2(ServiceAccountKey)` or `BearerToken(String)` per provider.
- **`RequestAuth`**: Unified request auth (GCP or Bearer); server uses it for the `Authorization` header.

### Changed

- **Config**: Replaced flat `predict_resource_url` / `llm_model` / `service_account_key` with `llm_provider: LlmProviderConfig`. `build_predict_url()` and `llm_model()` delegate to the provider.
- **.env**: Use `LLM_PROVIDER=vertex` and either `LLM_URL` or Vertex vars. See `.env.example`.

### Documentation

- **Help / doctor**: Updated to describe only override and Vertex config.
- **.env.example**: Shows Vertex vars and optional `LLM_URL` override.

---

## [0.3.1] - 2026-02-10

### Added

- **Homebrew tap**: Install with `brew tap yarenty/tap` and `brew install modelmux` ([yarenty/homebrew-tap](https://github.com/yarenty/homebrew-tap))

### Changed

- **Docs**: Simplified RELEASING.md (tap publish steps only); README Installation lists Homebrew first

---

## [0.3.0] - 2026-02-10

### Fixed

- **Doctor command**: Fixed `modelmux doctor` to properly load `.env` file before checking environment variables
- **Port binding errors**: Enhanced error messages for port binding failures with actionable suggestions:
  - Instructions to find and kill processes using the port
  - Suggestion to use `killport` utility
  - Guidance on changing port via environment variable

### Technical Details

- **Environment loading**: `run_doctor()` now calls `dotenvy::dotenv()` before checking environment variables
- **Error handling**: Port binding errors now provide comprehensive troubleshooting guidance
- **Test improvements**: Updated CLI tests to handle port binding errors as valid diagnostic output

---

## [0.2.0] - 2026-02-10

### Added

#### CLI Interface
- **Command-line flags**: Added `--version` (`-V`) and `--help` (`-h`) flags for Homebrew compatibility
- **Pre-config CLI handling**: CLI flags now work without requiring configuration, enabling proper Homebrew installation testing
- **Help documentation**: Comprehensive help output with usage instructions and environment variable documentation

#### Testing Infrastructure
- **Comprehensive test suite**: Added 30+ tests covering critical functionality
  - CLI argument parsing tests (`tests/cli_tests.rs`)
  - Configuration validation and parsing tests (`tests/config_tests.rs`)
  - Integration tests for application initialization (`tests/integration_tests.rs`)
  - Enhanced server tests for client detection and streaming behavior
- **Test utilities**: Added test helpers and fixtures for consistent testing
- **Test documentation**: Created `docs/TESTING.md` with comprehensive testing guide

#### Homebrew Deployment
- **Homebrew formula**: Created professional Homebrew formula (`packaging/homebrew/modelmux.rb`)
  - Comprehensive test block verifying binary installation and CLI flags
  - Configuration validation tests
  - Proper base64 encoding for test credentials
- **Packaging structure**: Organized packaging files into `packaging/homebrew/` directory
- **Release documentation**: Created `docs/RELEASING.md` with step-by-step release process

#### Documentation
- **Release guide**: Added `docs/RELEASING.md` with copy-pasteable release commands
- **Testing guide**: Added `docs/TESTING.md` with comprehensive testing documentation
- **Packaging README**: Added `packaging/homebrew/README.md` explaining formula structure

### Changed

- **Project structure**: Reorganized Homebrew formula into professional `packaging/homebrew/` directory
- **Documentation organization**: Improved documentation structure with dedicated guides for testing and releasing

### Technical Details

- **CLI implementation**: CLI arguments are handled before configuration loading, ensuring `--version` and `--help` work in all scenarios
- **Test coverage**: Achieved comprehensive coverage of:
  - CLI functionality (4 tests)
  - Configuration management (15+ tests)
  - Server logic (9 tests)
  - Integration scenarios (2 tests)
- **Homebrew compliance**: Meets all Homebrew requirements including proper test blocks and binary verification

---

## [0.1.0] - 2024 (Initial Release)

### Status: Production Ready ✅

Initial release of ModelMux - a high-performance Rust proxy server that converts OpenAI-compatible API requests to Vertex AI (Anthropic Claude) format.

### Core Features

#### API Compatibility
- **OpenAI-compatible API**: Drop-in replacement for OpenAI API endpoints
- **Full endpoint support**: `/v1/chat/completions` and `/v1/models` endpoints
- **Health monitoring**: `/health` endpoint with metrics

#### Format Conversion
- **Bidirectional conversion**: Seamless translation between OpenAI and Anthropic formats
- **Tool/Function calling**: Full support for OpenAI tool calling format
- **Message handling**: Complete support for system, user, assistant, and tool messages
- **Content types**: Support for text and structured content blocks

#### Streaming Support
- **Server-Sent Events (SSE)**: Full streaming support with SSE format
- **Smart client detection**: Automatically detects client capabilities:
  - Forces non-streaming for IDEs (RustRover, IntelliJ, VS Code)
  - Uses buffered streaming for web browsers
  - Uses standard streaming for API clients
- **Streaming modes**: Configurable modes (auto, non-streaming, standard, buffered)
- **Intelligent buffering**: Accumulates chunks for better client compatibility

#### Performance & Reliability
- **High performance**: Async Rust with Tokio for maximum concurrency
- **Type safety**: Leverages Rust's type system for compile-time guarantees
- **Error handling**: Comprehensive error handling with proper Result types
- **Retry logic**: Configurable retry mechanisms with exponential backoff
- **Connection pooling**: Efficient HTTP client with connection reuse

#### Configuration
- **Environment-based config**: All configuration via environment variables
- **Flexible logging**: Configurable log levels (trace, debug, info, warn, error)
- **Port configuration**: Customizable server port (default: 3000)
- **Streaming configuration**: Multiple streaming modes for different use cases

#### Architecture
- **Clean architecture**: SOLID principles with modular design
- **Dependency inversion**: Depends on abstractions rather than concrete implementations
- **Separation of concerns**: Clear separation between config, auth, server, and converter modules
- **Library support**: Can be used as both binary and library crate

#### Observability
- **Structured logging**: Comprehensive logging with tracing
- **Health metrics**: Request counters, success/failure tracking, quota error monitoring
- **Error tracking**: Detailed error information for debugging

#### Authentication
- **Google Cloud authentication**: Full support for GCP service account authentication
- **OAuth2 integration**: Secure token management with yup-oauth2
- **Base64 key encoding**: Secure handling of service account keys

### Technical Stack

- **Language**: Rust Edition 2024
- **HTTP Server**: Axum 0.8
- **Async Runtime**: Tokio 1.x
- **HTTP Client**: Reqwest 0.13
- **Authentication**: yup-oauth2 12.1
- **Serialization**: Serde + serde_json
- **Logging**: tracing + tracing-subscriber

### License

Dual licensed under MIT OR Apache-2.0

---

## [Unreleased]

### Planned Features

- Docker container images
- Configuration validation tools (`modelmux doctor`, `modelmux validate`)
- Enhanced metrics and monitoring (Prometheus, OpenTelemetry)
- Multiple provider support (OpenAI, Anthropic, Cohere, etc.)
- Request/response caching layer
- Web UI for configuration and monitoring

See [ROADMAP.md](ROADMAP.md) for detailed future plans.

---

## Version History

- **1.3.5** (2026-07-13): 10 URL construction tests proving correct endpoint generation for all routing scenarios
- **1.3.4** (2026-07-13): `global` region correctly maps to `aiplatform.googleapis.com` in all URL paths
- **1.3.3** (2026-07-13): Fix root cause of multi-model routing — `build_predict_url_for_model` let-chain fallthrough
- **1.3.2** (2026-07-13): Multi-model URL construction fully structural — parse parent `url` into components, no string surgery
- **1.3.1** (2026-07-13): Fix default model URL ignoring `[vertex].url` when individual fields also set
- **1.3.0** (2026-07-13): `modelmux logs` command; doctor/validate show model routing; fix multi-model URL resolution
- **1.2.0** (2026-07-13): Support for multiple models — route requests by model name via `[[vertex.models]]`
- **1.1.0** (2026-05-23): macOS `~/.config/modelmux/` paths + auto-migration; rotating logs (~30 days retention)
- **1.0.0** (2026-02-17): Brew services, systemd daemon, .deb packaging, Linux release
- **0.6.0** (2026-02-14): Professional configuration system, TOML, CLI management
- **0.5.0** (2026-02-10): Provider abstraction, LLM_PROVIDER, Vertex/override config; legacy config removed
- **0.3.1** (2026-02-10): Homebrew tap published; docs simplified
- **0.3.0** (2026-02-10): Fixed doctor command, improved error messages, documentation cleanup
- **0.2.0** (2026-02-10): CLI interface, comprehensive tests, Homebrew deployment readiness
- **0.1.0** (2024): Initial production release with core proxy functionality

[1.3.5]: https://github.com/yarenty/modelmux/compare/v1.3.4...v1.3.5
[1.3.4]: https://github.com/yarenty/modelmux/compare/v1.3.3...v1.3.4
[1.3.3]: https://github.com/yarenty/modelmux/compare/v1.3.2...v1.3.3
[1.3.2]: https://github.com/yarenty/modelmux/compare/v1.3.1...v1.3.2
[1.3.1]: https://github.com/yarenty/modelmux/compare/v1.3.0...v1.3.1
[1.3.0]: https://github.com/yarenty/modelmux/compare/v1.2.0...v1.3.0
[1.2.0]: https://github.com/yarenty/modelmux/compare/v1.1.0...v1.2.0
[1.1.0]: https://github.com/yarenty/modelmux/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/yarenty/modelmux/compare/v0.6.0...v1.0.0
[0.6.0]: https://github.com/yarenty/modelmux/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/yarenty/modelmux/compare/v0.3.1...v0.5.0
[0.3.1]: https://github.com/yarenty/modelmux/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/yarenty/modelmux/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/yarenty/modelmux/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/yarenty/modelmux/releases/tag/v0.1.0
