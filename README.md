# ModelMux - Vertex AI to OpenAI Proxy (Rust)

<p align="center">
  <img src="docs/img/logo.png" alt="ModelMux Logo" width="120" height="120">
</p>

<p align="center">

      |   |   |   |   |
       \  |   |   |  /
        \ |   |   | /
         \|   |   |/
          +----------->

</p>

<h1 align="center">ModelMux</h1>

<p align="center">
<strong>High-performance proxy server converting OpenAI API requests to Vertex AI (Anthropic Claude) format.</strong><br/>
<em>Many models → One unified interface.</em>
</p>

<p align="center">
<a href="https://crates.io/crates/modelmux"><img src="https://img.shields.io/crates/v/modelmux.svg"></a>
<a href="https://docs.rs/modelmux"><img src="https://docs.rs/modelmux/badge.svg"></a>
<a href="https://github.com/yarenty/modelmux/blob/main/LICENSE-MIT"><img src="https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg"></a>
</p>


ModelMux is a production-ready, async Rust proxy that acts as a drop-in replacement for the OpenAI API.
It translates OpenAI-compatible requests into Google Vertex AI (Anthropic Claude) calls while preserving streaming, tool/function calling, and error semantics.
Designed for performance, safety, and clean architecture, ModelMux is ideal for teams standardizing on OpenAI APIs while running on Vertex AI infrastructure.


<p align="center">
<a href="#installation">Installation</a> •
<a href="#quick-start">Quick Start</a> •
<a href="#features">Features</a> •
<a href="#configuration">Configuration</a> •
<a href="#roadmap">Roadmap</a>
</p>

---

> *"The internet is like a vast electronic library. But someone has scattered all the books on the floor."* — Lao Tzu

---

## What is ModelMux?

ModelMux is a **high-performance Rust proxy server** that seamlessly converts OpenAI-compatible API requests to Vertex AI (Anthropic Claude) format. Built with Rust Edition 2024 for maximum performance and type safety.

<!-- "I have never killed anyone, but I have read some obituaries with great pleasure." - Mark Twain -->

- 🔁 Drop-in OpenAI replacement — zero client changes
- ⚡ High performance — async Rust with Tokio
- 🧠 Full tool/function calling support
- 📡 Streaming (SSE) compatible
- 🛡 Strong typing & clean architecture
- ☁️ Built for Vertex AI (Claude)

*Use ModelMux to standardize on the OpenAI API while keeping full control over your AI backend.*


> Stop rewriting API glue code. Start muxing.

---

## Features

- **🔌 OpenAI-Compatible API**: Drop-in replacement for OpenAI API endpoints
- **🛠️ Tool/Function Calling**: Full support for OpenAI tool calling format
- **📡 Smart Streaming**: Server-Sent Events (SSE) with intelligent client detection
- **🎯 Client Detection**: Automatically adjusts behavior for IDEs, browsers, and CLI tools
- **⚡ High Performance**: Async Rust with Tokio for maximum concurrency
- **🔒 Type Safety**: Leverages Rust's type system for compile-time guarantees
- **🔄 Retry Logic**: Configurable retry mechanisms with exponential backoff
- **📊 Observability**: Structured logging and health monitoring
- **🧩 Clean Architecture**: SOLID principles with modular design

---

## Installation

### Homebrew (macOS)

```bash
brew tap yarenty/tap
brew install modelmux
```

### Cargo

```bash
cargo install modelmux
```

### From Source

```bash
git clone https://github.com/yarenty/modelmux
cd modelmux
cargo build --release
./target/release/modelmux
```

### As a Library

Add to your `Cargo.toml`:

```toml
[dependencies]
modelmux = "0.2"
```

---

## Quick Start

<!-- "The trouble with having an open mind, of course, is that people keep coming along and sticking things into it." - Terry Pratchett -->

### 1. Set up your environment

Create a `.env` file:

```env
# Either set full URL (overrides provider-specific fields):
# LLM_URL="https://europe-west1-aiplatform.googleapis.com/v1/projects/MY_PROJECT/locations/europe-west1/publishers/anthropic/models/claude-sonnet-4@20250514"

# Or set Vertex-specific fields (LLM_PROVIDER=vertex):
LLM_PROVIDER=vertex
GCP_SERVICE_ACCOUNT_KEY="your-base64-encoded-key-here"
VERTEX_REGION=europe-west1
VERTEX_PROJECT=my-gcp-project
VERTEX_LOCATION=europe-west1
VERTEX_PUBLISHER=anthropic
VERTEX_MODEL_ID=claude-sonnet-4@20250514

# Optional: Server and streaming
PORT=3000
LOG_LEVEL=info
STREAMING_MODE=auto
```

### 2. Run ModelMux

```bash
modelmux
# or
cargo run --release
```

### 3. Send OpenAI-compatible requests

```bash
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-sonnet-4",
    "messages": [{"role": "user", "content": "Hello, ModelMux!"}],
    "stream": false
  }'
```

That's it! Your OpenAI code now talks to Vertex AI.

---

## Configuration

### Environment Variables

```bash
# Either LLM_URL (full resource URL) or Vertex-specific:
# export LLM_URL="https://europe-west1-aiplatform.googleapis.com/v1/projects/.../models/claude-sonnet-4@20250514"
export LLM_PROVIDER=vertex
export GCP_SERVICE_ACCOUNT_KEY="your-base64-encoded-key-here"
export VERTEX_REGION=europe-west1
export VERTEX_PROJECT=my-gcp-project
export VERTEX_LOCATION=europe-west1
export VERTEX_PUBLISHER=anthropic
export VERTEX_MODEL_ID=claude-sonnet-4@20250514

# Optional
export PORT=3000
export LOG_LEVEL=info
export STREAMING_MODE=auto
```

<!-- "Time flies like an arrow; fruit flies like a banana." - Groucho Marx -->

### Streaming Modes

ModelMux intelligently adapts its streaming behavior based on the client:

- **`auto`** (default): Automatically detects client capabilities and chooses the best streaming mode
  - Forces non-streaming for IDEs (RustRover, IntelliJ, VS Code) and CLI tools (goose, curl)
  - Uses buffered streaming for web browsers
  - Uses standard streaming for API clients
- **`non-streaming`**: Forces complete JSON responses for all clients
- **`standard`**: Word-by-word streaming as received from Vertex AI
- **`buffered`**: Accumulates chunks for better client compatibility

### Client Detection

ModelMux automatically detects problematic clients:

**Non-streaming clients:**
- JetBrains IDEs (RustRover, IntelliJ, PyCharm, etc.)
- CLI tools (goose, curl, wget, httpie)
- API testing tools (Postman, Insomnia, Thunder Client)
- Clients that don't accept `text/event-stream`

**Buffered streaming clients:**
- Web browsers (Chrome, Firefox, Safari, Edge)
- VS Code and similar editors

---

## API Endpoints

### Chat Completions
```
POST /v1/chat/completions
```

OpenAI-compatible chat completions with full tool calling support.

### Models
```
GET /v1/models
```

List available models in OpenAI format.

### Health Check
```
GET /health
```

Service health and metrics endpoint.

---

## Library Usage

Use ModelMux programmatically in your Rust applications:

```rust
use modelmux::{Config, create_app};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration from environment
    let config = Config::from_env()?;
    
    // Create the application
    let app = create_app(config).await?;
    
    // Start server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}
```

---

## Architecture

<!-- "I cook with wine, sometimes I even add it to the food." - W.C. Fields -->

```
OpenAI Client ──► ModelMux ──► Vertex AI (Claude)
     │               │              │
     │               │              │
  OpenAI API ──► Translation ──► Anthropic API
  Format         Layer         Format
```

**Core Components:**

- **`config`** - Configuration management and environment handling
- **`auth`** - Google Cloud authentication for Vertex AI
- **`server`** - HTTP server with intelligent routing
- **`converter`** - Bidirectional format translation
- **`error`** - Comprehensive error types and handling

---

## Project Structure

```
modelmux/
├── Cargo.toml              # Dependencies and metadata
├── README.md               # This file
├── LICENSE-MIT             # MIT license
├── LICENSE-APACHE          # Apache 2.0 license
├── docs/
└── src/
    ├── main.rs             # Application entry point
    ├── lib.rs              # Library interface
    ├── config.rs           # Configuration management
    ├── auth.rs             # Google Cloud authentication
    ├── error.rs            # Error types
    ├── server.rs           # HTTP server and routes
    └── converter/          # Format conversion modules
        ├── mod.rs
        ├── openai_to_anthropic.rs
        └── anthropic_to_openai.rs
```

---

## Examples

### Tool/Function Calling

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
    ]
  }'
```

### Streaming Response

```bash
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Accept: text/event-stream" \
  -d '{
    "model": "claude-sonnet-4",
    "messages": [{"role": "user", "content": "Write a haiku about Rust"}],
    "stream": true
  }'
```

---

## Performance

<!-- "The real problem is not whether machines think but whether men do." - B.F. Skinner -->

ModelMux is built for production workloads:

- **Zero-copy** JSON parsing where possible
- **Async/await** throughout for maximum concurrency
- **Connection pooling** for upstream requests
- **Intelligent buffering** for streaming responses
- **Memory efficient** request/response handling

---

## Comparison with Node.js Version

| Feature        | Node.js    | ModelMux (Rust) |
|----------------|------------|-----------------|
| Performance    | Good       | Excellent       |
| Memory Usage   | Higher     | Lower           |
| Type Safety    | Runtime    | Compile-time    |
| Error Handling | Try/catch  | Result types    |
| Concurrency    | Event loop | Async/await     |
| Startup Time   | Fast       | Very Fast       |
| Binary Size    | Large      | Small           |

---

## Observability

### Health Endpoint

```bash
curl http://localhost:3000/health
```

Returns service metrics:

```json
{
  "status": "ok",
  "metrics": {
    "total_requests": 1337,
    "successful_requests": 1300,
    "failed_requests": 37,
    "quota_errors": 5,
    "retry_attempts": 42
  }
}
```

### Logging

Configure log levels via environment:

```bash
export LOG_LEVEL=debug
export RUST_LOG=modelmux=trace
```

<!-- "I haven't failed. I've just found 10,000 ways that won't work." - Thomas Edison -->

---

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

---

## Contributing

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

### Development

```bash
git clone https://github.com/yarenty/modelmux
cd modelmux
cargo test
cargo run
```

---

## Roadmap

*See [ROADMAP.md](ROADMAP.md) for detailed future plans.*

**Near term:**
- Docker container images
- Configuration validation tools
- Enhanced metrics and monitoring

**Future:**
- Multiple provider support (OpenAI, Anthropic, Cohere, etc.)
- Intelligent request routing and load balancing
- Request/response caching layer
- Web UI for configuration and monitoring
- Advanced analytics and usage insights

---

<!-- "The best way to predict the future is to invent it." - Alan Kay -->

<p align="center">

      |   |   |   |   |
       \  |   |   |  /
        \ |   |   | /
         \|   |   |/
          +----------->

</p>

<p align="center">
<em>Many models enter. One response leaves.</em>
</p>

<p align="center">
<strong>ModelMux</strong> - Because your AI shouldn't be tied to one vendor.
</p>

---

## Activity

![Alt](https://repobeats.axiom.co/api/embed/b86b498b31f051472e6f18dff8cf6297dd51be6a.svg "Repobeats analytics image")

---