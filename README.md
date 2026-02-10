# ModelMux - Vertex AI to OpenAI Proxy (Rust)

A high-performance (Rust) proxy server that converts OpenAI-compatible API requests to Vertex AI (Anthropic Claude) format.

## Features

- **OpenAI-compatible API**: Drop-in replacement for OpenAI API endpoints
- **Tool/Function Calling**: Full support for OpenAI tool calling format
- **Streaming Support**: Server-Sent Events (SSE) streaming responses
- **Error Handling**: Comprehensive error handling with proper Result types
- **SOLID Principles**: Clean architecture with separation of concerns
- **Type Safety**: Leverages Rust's type system for safety
- **Performance**: Async/await with Tokio for high concurrency
- **Logging**: Configurable logging levels with tracing

## Prerequisites

- Rust 1.70+ (2021 edition)
- Google Cloud Platform service account key (base64 encoded)
- Vertex AI API access

## Configuration

Create a `.env` file in the project root:

```env
# Required: Base64-encoded Google Cloud service account key
GCP_SERVICE_ACCOUNT_KEY="your-base64-encoded-key-here"

# Optional: Vertex AI configuration
LLM_URL="https://europe-west1-aiplatform.googleapis.com/v1/projects/<your_project>/locations/<your_location>/publishers/"
LLM_CHAT_ENDPOINT=<your_model>:streamRawPredict"
LLM_MODEL="claude-sonnet-4"

# Optional: Server configuration
PORT=3000
LOG_LEVEL=info  # trace, debug, info, warn, error

# Optional: Streaming configuration
STREAMING_MODE=auto  # auto, non-streaming, standard, buffered

# Optional: Retry configuration
ENABLE_RETRIES=true
MAX_RETRY_ATTEMPTS=3
```

## Building

```bash
# Build in release mode
cargo build --release

# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo run
```

## Running

```bash
# Run directly
cargo run

# Or run the release binary
./target/release/modelmux
```

The server will start on `http://localhost:3000` (or the port specified in `PORT`).

## Streaming Modes

The proxy supports different streaming modes to optimize compatibility with various clients:

- **`auto`** (default): Automatically detects client capabilities and chooses the best streaming mode
  - Forces non-streaming for IDEs (RustRover, IntelliJ, VS Code) and CLI tools (goose, curl)
  - Uses buffered streaming for web browsers
  - Uses standard streaming for other clients
- **`non-streaming`**: Forces all responses to be non-streaming (complete JSON responses)
- **`standard`**: Uses word-by-word streaming as received from Vertex AI
- **`buffered`**: Accumulates text chunks and sends them in larger batches for better client compatibility

### Client Detection

The proxy automatically detects problematic clients based on User-Agent headers and Accept headers:

**Clients forced to non-streaming mode:**
- JetBrains IDEs (RustRover, IntelliJ, PyCharm, etc.)
- CLI tools (goose, curl, wget, httpie)
- API testing tools (Postman, Insomnia, Thunder Client)
- Clients that don't accept `text/event-stream`

**Clients using buffered streaming:**
- Web browsers (Chrome, Firefox, Safari, Edge)
- VS Code and similar editors

This ensures optimal compatibility while maintaining good performance for each client type.

## Usage

### OpenAI-Compatible Endpoints

- `POST /v1/chat/completions` - Chat completions (supports streaming)
- `GET /v1/models` - List available models
- `GET /health` - Health check

### Example Request

```bash
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-sonnet-4",
    "messages": [
      {"role": "user", "content": "Hello!"}
    ],
    "stream": false
  }'
```

### Example with Tools

```bash
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-sonnet-4",
    "messages": [
      {"role": "user", "content": "List files in the current directory"}
    ],
    "tools": [
      {
        "type": "function",
        "function": {
          "name": "list_directory",
          "description": "List files in a directory",
          "parameters": {
            "type": "object",
            "properties": {
              "path": {"type": "string"}
            },
            "required": ["path"]
          }
        }
      }
    ],
    "stream": false
  }'
```

## Project Structure

```
modelmux/
├── Cargo.toml          # Dependencies and project metadata
├── README.md           # This file
├── .env.example        # Example environment variables
└── src/
    ├── main.rs         # Application entry point
    ├── config.rs       # Configuration management
    ├── auth.rs         # Google Cloud authentication
    ├── error.rs        # Error types
    ├── server.rs       # HTTP server and routes
    └── converter/      # Format conversion modules
        ├── mod.rs
        ├── openai_to_anthropic.rs
        └── anthropic_to_openai.rs
```


## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

## Comparison with Node.js Version

| Feature        | Node.js    | Rust         |
|----------------|------------|--------------|
| Performance    | Good       | Excellent    |
| Memory Usage   | Higher     | Lower        |
| Type Safety    | Runtime    | Compile-time |
| Error Handling | Try/catch  | Result types |
| Concurrency    | Event loop | Async/await  |
| Startup Time   | Fast       | Very Fast    |
