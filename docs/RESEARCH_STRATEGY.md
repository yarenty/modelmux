# ModelMux: Strategic Research & Extension Guide

> *Research-backed recommendations for extending ModelMux — what to support, in what order, and why.*

---

## Executive Summary

ModelMux is well-positioned as a **Vertex AI → OpenAI proxy**. Your architecture (provider abstraction, converter pattern) is solid. The key decisions are:

1. **OpenAI Responses API** — High value, but Vertex/Anthropic don't support it natively; you'd translate Responses ↔ Chat Completions.
2. **Multi-provider (AWS, Azure, vLLM)** — High value for differentiation; each has distinct adoption drivers.
3. **Configuration** — Can be simplified with sensible defaults and `modelmux config init` as the primary path.

---

## 1. How to Extend This Project

### Current Architecture (Strengths)

Your codebase already has the right abstractions:

```
┌─────────────────────────────────────────────────────────────────┐
│  OpenAI Client (any SDK)                                         │
└────────────────────────────┬────────────────────────────────────┘
                              │ /v1/chat/completions, /v1/models
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  ModelMux Server                                                 │
│  ┌─────────────┐  ┌──────────────────┐  ┌────────────────────┐ │
│  │ Routes      │→ │ OpenAI↔Anthropic │→ │ LlmProviderBackend  │ │
│  │ (server.rs) │  │ Converters       │  │ (provider.rs)       │ │
│  └─────────────┘  └──────────────────┘  └────────────────────┘ │
└────────────────────────────┬────────────────────────────────────┘
                              │
         ┌────────────────────┼────────────────────┐
         ▼                    ▼                    ▼
   Vertex AI           (future)              (future)
   (Anthropic)         AWS Bedrock            vLLM
```

**Extension points:**

| Extension | Where | Effort |
|-----------|-------|--------|
| New provider (AWS, Azure, vLLM) | `provider.rs` + new converter | Medium |
| New API (Responses, embeddings) | `server.rs` + new route + converter | Medium–High |
| New tools (Skills, MCP) | Converter layer + provider-specific handling | High |

---

## 2. What to Support Next — Prioritized

### Tier 1: Quick Wins (1–2 weeks each)

| Feature | Why | Notes |
|---------|-----|-------|
| **OpenAI-compatible provider** | You already have `OpenAiCompatibleProvider` stub; vLLM, Mistral, Cloudflare all speak `/v1/chat/completions` | Single implementation unlocks vLLM, Mistral, Cloudflare, local models |
| **Embeddings endpoint** | `/v1/embeddings` — common for RAG, semantic search | Vertex has embeddings; conversion is simpler than chat |
| **Docker image** | Already on roadmap; critical for adoption | `docker run -p 3000:3000 modelmux` |

### Tier 2: Differentiators (2–4 weeks each)

| Feature | Why | Notes |
|---------|-----|-------|
| **vLLM backend** | Local/on-prem inference, no vendor lock-in, cost control | vLLM is OpenAI-compatible — use `OpenAiCompatibleProvider` with `OPENAI_BASE_URL=http://localhost:8000/v1` |
| **AWS Bedrock** | Huge enterprise footprint; many teams standardize on Bedrock | AWS has **Mantle** (official OpenAI-compatible endpoint) — you could proxy to Mantle or implement Bedrock-native |
| **Azure OpenAI** | Enterprise Microsoft shops; Azure is already OpenAI-compatible | Azure OpenAI ≈ OpenAI API with different base URL + auth; minimal conversion |
| **Responses API** | Future-proof; OpenAI recommends for new projects | See Section 3 |

### Tier 3: Advanced (1–2 months)

| Feature | Why | Notes |
|---------|-----|-------|
| **Responses API with translation** | Full parity with OpenAI's direction | Translate `input`/`output` ↔ `messages`; handle `previous_response_id` |
| **Skills** | OpenAI's modular instruction format | Skills are OpenAI-specific; Vertex/Anthropic would need equivalent or passthrough |
| **Routing / load balancing** | "nginx for AI" vision | Route by model, cost, latency, fallback |

---

## 3. OpenAI Responses API, Skills, Tools — Deep Dive

### Responses API vs Chat Completions

| Aspect | Chat Completions | Responses API |
|--------|------------------|---------------|
| Endpoint | `POST /v1/chat/completions` | `POST /v1/responses` |
| Input | `messages: [...]` | `input` (string or items), `instructions` |
| Output | `choices[].message` | `output` (array of items) |
| State | Manual context management | `previous_response_id`, `store: true` |
| Tools | Custom functions only | Built-in: web_search, file_search, code_interpreter, MCP |
| Structured output | `response_format` | `text.format` |
| Function calling | Externally tagged | Internally tagged, `call_id` correlation |

**Key insight:** Vertex AI and Anthropic use **Chat Completions–style** APIs (messages, tools as functions). The Responses API is OpenAI-specific. To support it in ModelMux you have two options:

1. **Proxy mode:** Accept Responses API requests, translate to Chat Completions format, send to Vertex, translate response back to Responses format. Some features (e.g. `web_search`, `store`) won't map 1:1.
2. **Passthrough mode:** If backend is OpenAI or an OpenAI-compatible service that supports Responses (e.g. vLLM has `/v1/responses`), proxy without translation.

### Skills

- **What:** Versioned bundles of files with `SKILL.md` manifest; reusable instructions.
- **Where:** Used with `tools[].environment.skills` in the Responses API.
- **Relevance for ModelMux:** Skills are an OpenAI platform feature. Vertex/Anthropic don't have equivalents. Supporting Skills would mean either (a) translating to system/instruction context, or (b) only supporting when backend is OpenAI/vLLM.

### Built-in Tools (Responses API)

| Tool | Vertex/Anthropic equivalent | Feasibility |
|------|-----------------------------|-------------|
| `web_search` | None native | You'd need to implement or integrate (e.g. Serper, Tavily) |
| `file_search` | None native | Complex; would require RAG pipeline |
| `code_interpreter` | None native | Very complex |
| `computer_use` | None | OpenAI-specific |
| `remote MCP` | None | OpenAI-specific |
| Custom functions | ✅ Both support | Already working in ModelMux |

**Recommendation:** Focus on **custom functions** (you have this) and **structured outputs**. Native OpenAI tools (web_search, etc.) are high effort and backend-specific; consider as Phase 4+.

---

## 4. More Endpoint Types — Is There a Point?

### AWS Bedrock

| Factor | Assessment |
|--------|------------|
| **Demand** | High — AWS is default for many enterprises |
| **OpenAI compatibility** | AWS **Mantle** offers official OpenAI-compatible API; Bedrock-native uses different format |
| **Effort** | Low if proxying to Mantle (just URL + AWS auth). Medium if Bedrock-native (new converter) |
| **Differentiation** | Many proxies exist (bedrock-access-gateway, etc.); Rust + multi-provider could differentiate |

**Verdict:** Worth it. Start with **Mantle** as backend (OpenAI-compatible) — minimal work. Add Bedrock-native later if you want full control.

### Azure OpenAI

| Factor | Assessment |
|--------|------------|
| **Demand** | High — Microsoft-heavy enterprises |
| **OpenAI compatibility** | **Very high** — Azure OpenAI is co-developed with OpenAI; same API shape |
| **Effort** | Low — different base URL, API key or Entra ID auth |
| **Differentiation** | Fewer Azure-specific proxies; good niche |

**Verdict:** Worth it. Easiest addition — use `OpenAiCompatibleProvider` with Azure base URL + `api-key` header.

### vLLM (Local / Self-Hosted)

| Factor | Assessment |
|--------|------------|
| **Demand** | Growing — privacy, cost, air-gapped environments |
| **OpenAI compatibility** | **Full** — vLLM implements `/v1/chat/completions`, `/v1/embeddings`, `/v1/responses` |
| **Effort** | Very low — vLLM is OpenAI-compatible; use `OpenAiCompatibleProvider` |
| **Differentiation** | "One config to rule them all" — same client code for Vertex, vLLM, Mistral |

**Verdict:** **Highest ROI.** Implementing `OpenAiCompatibleProvider` gives you vLLM, Mistral, Cloudflare, and any OpenAI-compatible endpoint for free.

---

## 5. Recommended Roadmap (Concrete)

### Phase A: Unlock Multi-Backend (4–6 weeks)

1. **Implement `OpenAiCompatibleProvider`** (1 week)
   - Config: `OPENAI_BASE_URL`, `OPENAI_API_KEY` (or similar)
   - Auth: `BearerToken`
   - Path: `/v1/chat/completions` (configurable)
   - **Unlocks:** vLLM, Mistral, Cloudflare, Azure OpenAI, OpenAI itself

2. **Add provider config to TOML** (2–3 days)
   ```toml
   [provider]
   type = "openai_compatible"  # or "vertex"
   base_url = "http://localhost:8000/v1"  # for vLLM
   api_key = "${OPENAI_API_KEY}"
   model = "meta-llama/Llama-3-8B-Instruct"
   ```

3. **Embeddings endpoint** (1 week)
   - Vertex has `textembedding-gecko`; add `/v1/embeddings` route + converter

### Phase B: Cloud Providers (4–6 weeks)

4. **Azure OpenAI** — Use OpenAiCompatibleProvider with Azure endpoint
5. **AWS Bedrock via Mantle** — Same approach; AWS SigV4 or API key auth
6. **AWS Bedrock native** (optional) — New `BedrockProvider` + converter for Converse API

### Phase C: Responses API (6–8 weeks)

7. **`/v1/responses` endpoint** — Accept Responses format
8. **Translation layer** — `input`/`output` ↔ `messages`; `previous_response_id` → context injection
9. **Structured outputs** — Map `text.format` ↔ `response_format` where possible

### Phase D: Polish

10. **Configuration simplification** — Single `modelmux config init` flow; sensible defaults
11. **Docker + binary releases**
12. **Prometheus/OpenTelemetry**

---

## 6. Configuration Simplification Ideas

Current pain: Many env vars, config file sections, precedence rules.

**Suggestions:**

1. **Single source of truth:** `modelmux config init` creates a minimal config; everything else is optional override.
2. **Presets:** `modelmux config init --preset vertex` vs `--preset vllm` vs `--preset azure` — generates appropriate template.
3. **Validation with fix hints:** `modelmux config validate` already exists; add "run `modelmux config init --preset X` to fix" in error messages.
4. **Reduce env vars:** For Vertex, require only `project`, `region`, `model`; infer `location` from `region`, `publisher` from model prefix.
5. **Documentation:** One-page "choose your backend" flowchart.

---

## 7. Competitive Landscape

| Project | Focus | ModelMux differentiator |
|---------|-------|-------------------------|
| **bedrock-access-gateway** | AWS Bedrock → OpenAI | Node.js; single provider |
| **LiteLLM** | Multi-provider proxy | Python; broader but heavier |
| **OpenAI-compatible proxies** (various) | Single provider | Often vendor-specific |
| **ModelMux** | Vertex-first, multi-provider | Rust (performance, small binary), clean architecture |

**Positioning:** "The high-performance, Rust-based proxy for teams using Vertex AI (and soon: vLLM, Azure, Bedrock) who want a single OpenAI-compatible interface."

---

## 8. Summary: What to Do Next

| Priority | Action | Impact |
|----------|--------|--------|
| **1** | Implement `OpenAiCompatibleProvider` | Unlocks vLLM, Azure, Mistral, local models |
| **2** | Add `/v1/embeddings` | Common use case; moderate effort |
| **3** | Docker image | Deployment readiness |
| **4** | Simplify config with presets | Better DX |
| **5** | Responses API translation | Future-proofing |
| **6** | AWS Bedrock (Mantle first) | Enterprise reach |
| **7** | Skills / native tools | Lower priority; OpenAI-specific |

**Bottom line:** Your architecture supports extension. The highest-leverage move is **completing `OpenAiCompatibleProvider`** — it immediately enables vLLM, Azure, and any OpenAI-compatible backend with one implementation.
