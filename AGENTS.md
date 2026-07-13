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
11. [Living Documentation Contract](#11-living-documentation-contract)

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

**Rule**: Before making changes to any component, **always read its specific AGENTS.md first** to understand:
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
1. **Start**: Review the current task in your private working notes (kept locally, never committed — see "Critical Rules & Protocols")
2. **Plan**: Update task phases and current status in those notes
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
**Before working on any specific component, ALWAYS read its AGENTS.md file first.**

Component-specific files contain crucial information about:
- Architecture patterns specific to that component
- Development workflows and testing procedures
- Technology-specific considerations and best practices
- Common issues and troubleshooting steps
- Integration patterns with other services

### Rule 1: Create Plan First — Privately
Never start a complex task without writing down a plan **locally**. Use the
template in `tools/task_template.md` to scaffold a private working file
(typical names: `task.md`, `plan.md`, or anything under `tasks/`). These
locations are gitignored on purpose — they are scratch notes for the agent
and the maintainer, never published.

**When to create a task plan:**
- Multi-step tasks (3+ steps)
- Research or analysis tasks
- Building/creating new components
- Adding new provider support
- Tasks spanning multiple files or components
- Format conversion modifications
- OpenAI API compatibility changes
- Authentication or provider configuration updates

**Rule 1a — Never publish planning artefacts.** Do not commit `plan.md`,
`task.md`, or anything under `tasks/`. Do not link to them from `README.md`,
`ROADMAP.md`, `CHANGELOG.md`, or any other published doc. Public surfaces
describe shipped behaviour and direction; the path to get there stays
private.

### Rule 2: The 2-Action Rule
> "After every 2 view/browser/search operations, IMMEDIATELY save key findings to text files."

This prevents loss of visual/multimodal information and maintains context across long sessions.

### Rule 3: Read Before You Decide
Before making major decisions, re-read the plan file and relevant documentation to ensure alignment with goals and architecture.

### Rule 4: Update After You Act
After completing any phase (in your private notes):
- Mark phase status: `pending` → `in_progress` → `complete`
- Log any errors encountered with resolution details
- Note files created, modified, or deleted
- Update decision log with rationale

### Rule 5: Log ALL Errors
Every error goes in your private task notes with:
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
| Where am I? | Current phase in your private task notes |
| Where am I going? | Remaining phases in your private task notes |
| What's the goal? | Goal statement in your private plan |
| What have I learned? | Findings and decisions in your private notes |
| What have I done? | Progress tracking in your private notes |

---

## 9. Implementation Status

### Current Status
**Version**: 1.3.0 (Production Ready) — released 2026-07-13
- ✅ OpenAI to Vertex AI (Claude) proxy
- ✅ OpenAI-compatible provider abstraction (`LlmProviderBackend`)
- ✅ Streaming with intelligent client detection
- ✅ Tool/function calling support
- ✅ **Multi-model routing**: clients select a model by name; proxy routes to the correct Vertex AI endpoint via `[[vertex.models]]` config
- ✅ **`modelmux logs` / `logs -f`**: inspect and live-tail log files from the CLI
- ✅ Rust Edition 2024 with type safety
- ✅ Comprehensive error handling
- ✅ CLI: `--version`, `--help`, `config {init,show,validate,edit}`, `doctor`, `validate`, `logs`
- ✅ TOML configuration with platform-native paths (XDG-style on Linux and macOS, Known-Folder on Windows)
- ✅ **Automatic, idempotent macOS config migration to `~/.config/modelmux/`** (1.1.0)
- ✅ Homebrew, systemd, .deb packaging
- ✅ 37 unit tests green

### Roadmap

See [ROADMAP.md](ROADMAP.md) — the single, public source of truth for where
ModelMux is going. Per-task breakdowns and weekly plans are tracked in
private working notes (gitignored); only public docs describe direction.

### Technical Debt
- Provider abstraction works but needs a second concrete implementation (OpenAI-compatible) to validate the trait surface
- Configuration validation could be more comprehensive (esp. provider-specific sections)
- No Prometheus / OpenTelemetry integration yet
- No Docker image yet
- Legacy macOS-config fallback in the loader can be removed once 1.1.0 is widely deployed (next major release)

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
- [ ] Keep component AGENTS.md files current
- [ ] Update API documentation for changes
- [ ] Record architectural decisions
- [ ] Maintain task planning discipline (in private notes)
- [ ] Update this master AGENTS.md as project evolves
- [ ] Update README.md with configuration examples
- [ ] Document new environment variables
- [ ] Update OpenAI compatibility notes
- [ ] Refresh streaming behavior documentation
- [ ] Update provider setup instructions

---

## 11. Living Documentation Contract

> ModelMux ships with four **public** documents that the team treats as a
> single coherent surface. They must stay in sync. Private working notes
> (gitignored) are for the agent and maintainer only and never substitute
> for these public docs.

### The four canonical public documents

| File | Owns |
|------|------|
| [`README.md`](README.md) | What ModelMux is, how to install and run it, the smallest correct quickstart, links to the rest |
| [`CHANGELOG.md`](CHANGELOG.md) | Released history (Keep-a-Changelog format, SemVer) + an `[Unreleased]` section as a holding pen |
| [`ROADMAP.md`](ROADMAP.md) | Vision and direction. Where we are, where we're going, *why* — not how. No internal task numbers. |
| [`AGENTS.md`](AGENTS.md) | This file: principles, workflow, quality bars, and the sync contract that keeps the above three honest |

### Private (gitignored, never published)

- `plan.md`, `task.md`, anything under `tasks/`, `docs/RESEARCH_STRATEGY.md`
- Scratch notes, per-feature breakdowns, research dumps
- The agent uses these freely; **none of it appears in published docs**

### Rules

1. **Single PR, single state.** Any code change that ships behaviour visible
   to users must, in the same PR, update at least:
   - `CHANGELOG.md` under `[Unreleased]` (or the just-cut release section), and
   - whichever of `README.md` / `ROADMAP.md` / `AGENTS.md` describe the
     surface that changed.

2. **Cutting a release rolls the docs.** When a version number is bumped in
   `Cargo.toml`:
   - Move `[Unreleased]` entries into a new `[X.Y.Z] - YYYY-MM-DD` section in `CHANGELOG.md`.
   - Add the row to the Version History footer and a compare-link.
   - Update the "Current release" banner in `ROADMAP.md`.
   - Update `AGENTS.md` §9 "Current Status".
   - Bump the URL in `packaging/homebrew/modelmux.rb` and update its `sha256`.
   - Follow [`docs/RELEASING.md`](docs/RELEASING.md) for the rest.

3. **README is not a changelog and not a roadmap.** Keep it tight; link to
   `CHANGELOG.md` for history and `ROADMAP.md` for direction.

4. **ROADMAP communicates vision, not internal numbering.** It says where
   we're going and shows it (small code blocks, examples). It does not link
   to task files, weekly plans, or anything from the private notes.

5. **Private notes never leak.** Don't reference `plan.md`, `task.md`,
   `tasks/`, internal `TASK-NNN` IDs, or research-only docs from any
   published file. If a piece of context is worth publishing, lift it into
   `README` / `CHANGELOG` / `ROADMAP` / `AGENTS` in shippable form.

6. **The 5-minute sync check.** Before opening a PR, grep your diff for the
   new version number and confirm each hit is consistent across the four
   public documents and `Cargo.toml`. If any doc still describes the old
   behaviour, fix it now, not in a follow-up.

### Why this matters

ModelMux is small enough that "the code is the spec" is tempting. It's also
a trap: the moment a user reads `ROADMAP.md` and the code does something
else, they file a confused issue and trust drops. We prefer boring,
machine-greppable consistency on the public surface — and keep the messy
day-to-day planning private.

---

## Anti-Patterns to Avoid

| ❌ Don't | ✅ Do Instead |
|----------|---------------|
| Use temporary notes for persistence | Create structured private notes (e.g. local `task.md`, `findings.md`) |
| State goals once and forget | Re-read plans before major decisions |
| Hide errors and retry silently | Log all errors with resolution details |
| Stuff everything in context | Store large content in organized files |
| Start executing immediately | Create task plan FIRST |
| Repeat failed actions | Track attempts, mutate approach systematically |
| Violate SOLID principles for speed | Take time to design proper abstractions |
| Skip component documentation | Always read component AGENTS.md first |
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
