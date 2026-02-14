# ModelMux AI Agent Documentation

> **READ THIS FIRST**: This file serves as the single source of truth for any AI agent (Claude, Gemini, Cursor, etc.) working on the ModelMux repository. It aggregates architectural context, development workflows, and behavioral guidelines.

## Table of Contents
1. [Philosophy & Core Principles](#1-philosophy--core-principles)
2. [Project Identity](#2-project-identity)
3. [Architecture & Design Principles](#3-architecture--design-principles)
4. [Technology Stack](#4-technology-stack)
5. [Repository Structure](#5-repository-structure)
6. [Development Workflows](#6-development-workflows)
7. [Quality Standards](#7-quality-standards)
8. [Critical Rules & Protocols](#8-critical-rules--protocols)
9. [Implementation Status](#9-implementation-status)
10. [Common AI Tasks](#10-common-ai-tasks)

---

## 1. Philosophy & Core Principles

### Core Philosophy
- **Incremental progress over big bangs**: Break complex tasks into manageable stages
- **Learn from existing code**: Understand patterns before implementing new features
- **Clear intent over clever code**: Prioritize readability and maintainability
- **Simple over complex**: Keep implementations straightforward - prioritize solving problems over architectural complexity

### The Eight Honors and Eight Shames
| **Shame** | **Honor** |
|-----------|-----------|
| Guessing APIs | Careful research and documentation reading |
| Vague execution | Seeking confirmation before major changes |
| Assuming business logic | Human verification of requirements |
| Creating new interfaces | Reusing existing, proven patterns |
| Skipping validation | Proactive testing and error handling |
| Breaking architecture | Following established specifications |
| Pretending to understand | Honest acknowledgment of uncertainty |
| Blind modification | Careful, incremental refactoring |

### SOLID Principles Integration
Our codebase follows SOLID principles to ensure maintainable, scalable software.

**Quick Reference**: See [`tools/solid_principles_quick_reference.md`](tools/solid_principles_quick_reference.md) for essential patterns and checklists.

**Detailed Guide**: See [`tools/solid_principles_guide.md`](tools/solid_principles_guide.md) for comprehensive examples and implementation strategies.

#### Core SOLID Guidelines for AI Development
- **Single Responsibility (SRP)**: Before adding functionality, ask "Does this belong here?"
- **Open/Closed (OCP)**: Extend behavior through new classes/modules, not modifications
- **Liskov Substitution (LSP)**: Ensure any subclass can replace its parent without breaking functionality
- **Interface Segregation (ISP)**: Design small, specific interfaces rather than large, monolithic ones
- **Dependency Inversion (DIP)**: Inject dependencies rather than creating them directly

---

## 2. Project Identity

**Name**: ModelMux 
**Purpose**: High-performance Rust proxy that converts OpenAI-compatible API requests to Vertex AI (Anthropic Claude) format  
**Core Value Proposition**: Drop-in replacement for OpenAI API while using Vertex AI backend - "Many models → One unified interface"  
**Primary Mechanism**: Request translation layer with streaming, tool calling, and intelligent client detection  
**Target Users**: Development teams standardizing on OpenAI APIs while running on Vertex AI infrastructure  

### Business Context
- **Problem Solved**: Eliminates vendor lock-in by providing OpenAI-compatible interface for Vertex AI, reducing API rewrite costs
- **Success Metrics**: Request throughput, translation accuracy, streaming performance, client compatibility
- **Key Constraints**: Must maintain OpenAI API compatibility while optimizing for Vertex AI capabilities

---

## 3. Architecture & Design Principles

### Architectural Patterns
- **Layered Architecture**: Clear separation between HTTP handling, translation, and provider communication
- **Adapter Pattern**: OpenAI ↔ Anthropic format conversion
- **Proxy Pattern**: Transparent request forwarding with format translation
- **Strategy Pattern**: Multiple streaming modes and client detection strategies

### Design Patterns in Use
- **Repository Pattern**: For data access abstraction
- **Factory Pattern**: For object creation
- **Strategy Pattern**: For algorithm selection
- **Observer Pattern**: For event handling
- **Builder Pattern**: Configuration construction and validation
- **Facade Pattern**: Simple library interface hiding complexity

### Cross-Cutting Concerns
- **Logging**: Structured logging with tracing, configurable log levels (trace, debug, info, warn, error)
- **Error Handling**: Result-based error handling with custom ProxyError types, comprehensive error propagation
- **Security**: Google Cloud OAuth2 authentication, service account key management, request validation
- **Performance**: Async/await throughout, zero-copy parsing, connection pooling, intelligent buffering
- **Monitoring**: Health endpoints, request metrics, streaming performance monitoring, client detection analytics

---

## 4. Technology Stack

### Primary Technologies
- **Language**: Rust Edition 2024
- **Web Framework**: Axum (HTTP server)
- **Async Runtime**: Tokio (full features)
- **HTTP Client**: Reqwest (JSON, streaming)
- **Authentication**: yup-oauth2 (Google Cloud)
- **Serialization**: Serde (JSON handling)
- **Logging**: Tracing + tracing-subscriber
- **Error Handling**: Thiserror + Anyhow

### Development Tools
- **Version Control**: Git with GitHub
- **Build System**: Cargo (Rust native)
- **Testing Framework**: Built-in Rust testing + tokio-test
- **CI/CD**: GitHub Actions (implied from packaging)
- **Code Quality**: rustfmt, clippy, comprehensive test suite (30+ tests)

### External Dependencies
- **Google Cloud Vertex AI**: Primary LLM backend
- **Anthropic Claude Models**: Target AI models via Vertex
- **Base64 Encoding**: Service account key handling
- **Chrono**: Time utilities for requests
- **Futures/Tokio-stream**: Async streaming support

---

## 5. Repository Structure

### Directory Layout
```
modelmux/
├── src/                    # Main source code
│   ├── converter/         # Format conversion modules
│   ├── main.rs            # Binary entry point
│   ├── lib.rs             # Library interface
│   ├── config.rs          # Configuration management
│   ├── auth.rs            # Google Cloud authentication
│   ├── server.rs          # HTTP server and routes
│   ├── provider.rs        # LLM backend abstraction
│   └── error.rs           # Error types and handling
├── tests/                 # Integration and unit tests
├── docs/                  # Documentation and guides
├── tools/                 # Development and build tools
├── target/                # Build artifacts
└── packaging/             # Distribution packages
```

### Component-Specific Documentation
**⚠️ CRITICAL**: Each major component contains its own documentation:

- `src/lib.rs` - Library usage and module overview
- `docs/TESTING.md` - Testing strategy and procedures  
- `docs/RELEASING.md` - Release and deployment processes
- `rust_coding_conventions.md` - Rust-specific coding standards
- Component-level documentation within each module

**Rule**: Before making changes to any component, **always read its specific AGENT.md first** to understand:
- Component architecture and responsibilities
- Development workflows and testing approaches  
- API patterns and integration points
- Common issues and troubleshooting steps
- Technology-specific considerations

### Service Architecture
```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────────┐
│   OpenAI Client │───▶│    ModelMux      │───▶│   Vertex AI (Claude)│
│                 │    │   Proxy Server   │    │                     │
│                 │    │                  │    │                     │
│  - Existing app │    │ - Format trans.  │    │ - Anthropic models  │
│  - Zero changes │    │ - Streaming      │    │ - Google Cloud      │
│  - OpenAI API   │    │ - Client detect. │    │ - OAuth2 auth       │
└─────────────────┘    └──────────────────┘    └─────────────────────┘
```

**Request Flow**: OpenAI format → Translation → Vertex AI → Translation → OpenAI format
**Streaming**: Server-Sent Events with intelligent client detection and buffering strategies

---

## 6. Development Workflows

### Initial Setup
```bash
# Install via Homebrew
brew tap yarenty/tap
brew install modelmux

# Or install via Cargo
cargo install modelmux

# Set up environment
cp .env.example .env
# Edit .env with your GCP credentials and Vertex AI settings

# Run server
modelmux
```

### Daily Development Workflow
1. **Start**: Review current task in `task.md`
2. **Plan**: Update task phases and current status
3. **Research**: Read relevant component documentation
4. **Implement**: Follow incremental development approach
5. **Test**: Validate changes incrementally
6. **Document**: Update task progress and decisions
7. **Review**: Ensure code meets quality standards

### Feature Development Process
1. **Analysis**: Understand requirements and constraints
2. **Design**: Plan implementation following SOLID principles
3. **Implementation**: Write code in small, testable increments
4. **Testing**: Unit tests, integration tests, and manual validation
5. **Documentation**: Update relevant docs and component guides
6. **Review**: Code review and architectural compliance check

### Build and Deployment
```bash
# Development build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo run

# Deploy via Homebrew formula
brew install modelmux
```

### Testing Strategy
- **Unit Tests**: Component-level testing for config, auth, conversion logic
- **Integration Tests**: End-to-end request/response testing with mock providers  
- **CLI Tests**: Command-line interface validation (--version, --help)
- **Validation Tests**: Configuration validation and error handling
- **Performance Tests**: Streaming performance and concurrent request handling

---

## 7. Quality Standards

### Code Quality
- **English Only**: All comments, documentation, and naming in English
- **Self-Documenting Code**: Clear naming conventions over extensive comments
- **No Unnecessary Comments**: Let clear code speak for itself
- **Consistent Style**: Follow established formatting and naming conventions

### Documentation Standards
- **API Documentation**: Rust docs with examples, module-level documentation
- **Architecture Decision Records**: Document significant architectural choices
- **Component Guides**: Maintain up-to-date component-specific documentation  
- **Task Documentation**: Use structured task planning for complex work
- **README Updates**: Keep installation, configuration, and usage current

### Testing Standards
- **Test Coverage**: Comprehensive coverage across all core modules (30+ tests currently)
- **Test Naming**: Clear, descriptive test names that explain intent
- **Test Structure**: Arrange-Act-Assert pattern
- **Integration Testing**: Test component interactions, especially format conversion

### Performance Standards
- **Response Time**: Sub-100ms translation overhead for non-streaming requests
- **Throughput**: High concurrency via async/await, connection pooling
- **Resource Usage**: Memory-efficient with zero-copy parsing where possible
- **Scalability**: Designed for production workloads with intelligent buffering

---

## 8. Critical Rules & Protocols

### Rule 0: Read Component Documentation First
**Before working on any specific component, ALWAYS read its AGENT.md file first.**

Component-specific files contain crucial information about:
- Architecture patterns specific to that component
- Development workflows and testing procedures
- Technology-specific considerations and best practices
- Common issues and troubleshooting steps
- Integration patterns with other services

### Rule 1: Create Plan First
Never start a complex task without creating a `task.md` file. Use the template in `tools/task_template.md`.

**When to create a task plan:**
- Multi-step tasks (3+ steps)
- Research or analysis tasks
- Building/creating new components
- Adding new provider support
- Tasks spanning multiple files or components
- Format conversion modifications
- OpenAI API compatibility changes
- Authentication or provider configuration updates

### Rule 2: The 2-Action Rule
> "After every 2 view/browser/search operations, IMMEDIATELY save key findings to text files."

This prevents loss of visual/multimodal information and maintains context across long sessions.

### Rule 3: Read Before You Decide
Before making major decisions, re-read the plan file and relevant documentation to ensure alignment with goals and architecture.

### Rule 4: Update After You Act
After completing any phase:
- Mark phase status: `pending` → `in_progress` → `complete`
- Log any errors encountered with resolution details
- Note files created, modified, or deleted
- Update decision log with rationale

### Rule 5: Log ALL Errors
Every error goes in the task plan file with:
- Error description
- Attempt number
- Resolution approach
- Lessons learned

```markdown
## Errors Encountered
| Error | Attempt | Resolution | Lessons Learned |
|-------|---------|------------|----------------|
| FileNotFoundError | 1 | Created default config | Check file existence first |
| API timeout | 2 | Added retry logic | Network calls need resilience |
```

### Rule 6: Never Repeat Failures
```
if action_failed:
    next_action != same_action
```
Track what you tried. Mutate the approach. Learn from failures.

### The 3-Strike Error Protocol

```
ATTEMPT 1: Diagnose & Fix
  → Read error message carefully
  → Identify root cause
  → Apply targeted fix

ATTEMPT 2: Alternative Approach  
  → Same error? Try different method
  → Different tool? Different library?
  → NEVER repeat exact same failing action

ATTEMPT 3: Broader Rethink
  → Question initial assumptions
  → Search for solutions and best practices
  → Consider updating the plan or approach

AFTER 3 FAILURES: Escalate to User
  → Explain what you tried in detail
  → Share the specific error messages
  → Ask for guidance or clarification
```

### Context Management Protocol

#### Read vs Write Decision Matrix
| Situation | Action | Reason |
|-----------|--------|--------|
| Just wrote a file | DON'T read | Content still in context |
| Viewed image/PDF | Write findings NOW | Multimodal data doesn't persist |
| Browser returned data | Write to file | Screenshots are temporary |
| Starting new phase | Read plan/findings | Re-orient if context is stale |
| Error occurred | Read relevant files | Need current state to debug |
| Resuming after gap | Read all planning files | Recover full state |

#### The 5-Question Context Check
If you can answer these questions, your context management is solid:

| Question | Answer Source |
|----------|---------------|
| Where am I? | Current phase in task.md |
| Where am I going? | Remaining phases in task.md |
| What's the goal? | Goal statement in plan |
| What have I learned? | Findings and decisions in task.md |
| What have I done? | Progress tracking in task.md |

---

## 9. Implementation Status

### Current Status
**Version**: 0.5.0 (Production Ready)
- ✅ OpenAI to Vertex AI (Claude) proxy
- ✅ Streaming with intelligent client detection  
- ✅ Tool/function calling support
- ✅ Rust Edition 2024 with type safety
- ✅ Comprehensive error handling
- ✅ CLI interface with --version and --help
- ✅ 30+ test suite coverage
- ✅ Homebrew distribution ready
- ✅ Professional packaging structure

### Roadmap
**Phase 1**: Distribution & DevOps (Docker, validation tools, enhanced observability)
**Phase 2**: Multi-Provider Universe (OpenAI, Anthropic Direct, Cohere, AWS Bedrock, Azure)
**Phase 3**: Performance & Scale (caching, load balancing, auto-scaling)
**Phase 4**: Enterprise Features (auth, compliance, cost management)
**Phase 5**: AI-Native Features (model performance prediction, smart routing)
**Phase 6**: Ecosystem & Extensions (plugin architecture, community hub)

See [ROADMAP.md](ROADMAP.md) for detailed plans.

### Technical Debt
- Provider abstraction could be enhanced for future multi-provider support
- Configuration validation could be more comprehensive
- Metrics and observability need Prometheus integration
- Docker containerization not yet implemented

### Known Issues
- Some edge cases in streaming client detection
- Rate limiting not yet implemented at proxy level
- Configuration hot-reloading not supported
- Limited provider health checking

---

## 10. Common AI Tasks

### Code Review Checklist
- [ ] Follows SOLID principles (use [quick reference](tools/solid_principles_quick_reference.md))
- [ ] Maintains existing architectural patterns
- [ ] Includes appropriate tests
- [ ] Updates relevant documentation
- [ ] Handles errors gracefully
- [ ] Follows code quality standards
- [ ] Preserves OpenAI API compatibility
- [ ] Maintains streaming performance
- [ ] Updates format conversion logic if needed
- [ ] Tests client detection behavior
- [ ] Validates configuration changes

### Refactoring Guidelines
- [ ] Understand existing code thoroughly before changing
- [ ] Make small, incremental changes
- [ ] Maintain backward compatibility where possible
- [ ] Update tests to reflect changes
- [ ] Document architectural decisions
- [ ] Test against multiple OpenAI client types
- [ ] Verify streaming modes still work correctly
- [ ] Ensure format conversion accuracy
- [ ] Validate authentication flows

### New Feature Development
- [ ] Create task plan using template
- [ ] Research existing patterns and components
- [ ] Design following SOLID principles
- [ ] Implement incrementally with tests
- [ ] Update component documentation
- [ ] Perform integration testing
- [ ] Test OpenAI API compatibility
- [ ] Validate streaming behavior across client types
- [ ] Check performance impact on translation layer
- [ ] Update configuration validation if needed
- [ ] Document provider-specific considerations

### Debugging Process
- [ ] Reproduce the issue consistently
- [ ] Identify root cause, not just symptoms
- [ ] Apply targeted fix following 3-strike protocol
- [ ] Add tests to prevent regression
- [ ] Document resolution in task plan
- [ ] Check if issue affects specific client types
- [ ] Verify streaming vs non-streaming behavior
- [ ] Test format conversion accuracy
- [ ] Validate authentication and provider connectivity
- [ ] Review logs for request/response patterns

### Documentation Updates
- [ ] Keep component AGENT.md files current
- [ ] Update API documentation for changes
- [ ] Record architectural decisions
- [ ] Maintain task planning discipline
- [ ] Update this master AGENT.md as project evolves
- [ ] Update README.md with configuration examples
- [ ] Document new environment variables
- [ ] Update OpenAI compatibility notes
- [ ] Refresh streaming behavior documentation
- [ ] Update provider setup instructions

---

## Anti-Patterns to Avoid

| ❌ Don't | ✅ Do Instead |
|----------|---------------|
| Use temporary notes for persistence | Create structured files (task.md, findings.md) |
| State goals once and forget | Re-read plans before major decisions |
| Hide errors and retry silently | Log all errors with resolution details |
| Stuff everything in context | Store large content in organized files |
| Start executing immediately | Create task plan FIRST |
| Repeat failed actions | Track attempts, mutate approach systematically |
| Violate SOLID principles for speed | Take time to design proper abstractions |
| Skip component documentation | Always read component AGENT.md first |
| Break OpenAI API compatibility | Test against multiple OpenAI client types |
| Hardcode provider-specific logic | Use configurable provider abstractions |
| Ignore streaming performance | Profile and optimize translation overhead |
| Skip client detection testing | Test with IDEs, browsers, CLI tools, APIs |
| Modify format conversion carelessly | Validate round-trip conversion accuracy |
| Assume all clients handle streaming | Test non-streaming, buffered, and standard modes |
| Skip authentication testing | Validate GCP service account and token flows |

---

**Remember**: This documentation evolves with the project. Keep it updated as architectural decisions are made and new patterns emerge. The goal is to enable efficient, high-quality AI-assisted development that maintains consistency and follows best practices.

---

## ModelMux-Specific Development Patterns

### Format Conversion Best Practices
- **Always validate round-trip conversion**: OpenAI → Anthropic → OpenAI should preserve semantics
- **Handle edge cases gracefully**: Empty messages, unsupported parameters, malformed requests
- **Maintain streaming integrity**: Ensure streaming chunks translate correctly without data loss
- **Preserve tool calling format**: OpenAI function calling must map precisely to Anthropic tools

### Client Detection Guidelines
- **Test across client types**: IDEs (RustRover, VS Code), CLI tools (curl, goose), browsers, API clients
- **Validate User-Agent patterns**: Ensure detection logic covers new clients as they emerge
- **Default to safe behavior**: When in doubt, use non-streaming for unknown clients
- **Log detection decisions**: Track which clients trigger which streaming modes for debugging

### Authentication & Provider Management
- **Never hardcode credentials**: Always use environment variables or secure configuration
- **Handle auth failures gracefully**: Provide clear error messages for configuration issues
- **Support credential rotation**: Design for service account key updates without restart
- **Validate provider connectivity**: Health checks should verify end-to-end provider access

### Performance Considerations
- **Profile translation overhead**: Keep format conversion under 10ms for non-streaming
- **Optimize streaming latency**: Minimize delay between receiving and forwarding chunks
- **Monitor memory usage**: Large requests should not cause memory spikes
- **Connection pooling**: Reuse HTTP connections to Vertex AI for better performance

### Configuration Management
- **Validate on startup**: Catch configuration errors before accepting requests
- **Provide clear error messages**: Help users understand what's wrong and how to fix it
- **Support multiple configuration methods**: Environment variables, files, command-line args
- **Document all configuration options**: Keep README.md examples current with code
