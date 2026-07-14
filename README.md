# Agentic Council

A private, local-first Windows and macOS desktop application for structured multi-model AI deliberation. You assemble a council of AI agents — each running on a different model or provider — give them a topic, and let them debate it across multiple rounds. The app enforces independence, contradiction analysis, anonymized peer scoring, and automatic session recovery, all without sending any data anywhere except the LLM provider APIs you explicitly configure.

---

## Quick start (no technical experience needed)

| Platform | What to do |
|---|---|
| **Windows** | Right-click **`install.bat`** → **Run as administrator** |
| **macOS** | Open Terminal, type `bash ` then drag **`install.sh`** into the window, press Enter |

The scripts automatically install Node.js, Rust, the necessary build tools, and all dependencies, then compile and launch the app. **First launch takes 10–20 minutes** because the Rust backend is compiled on your machine; every subsequent launch is instant.

For detailed step-by-step instructions and a troubleshooting table, see **[INSTALL.md](INSTALL.md)**.

---

## What the app does

Agentic Council runs a gated, multi-round debate between AI agents. Each round executes a four-stage pipeline:

1. **Parallel independent generation** — every agent answers simultaneously without seeing each other's output, eliminating anchoring bias.
2. **Contradiction and gap analysis** — the Orchestrator makes a dedicated pass to flag contradictions between agents, analytical omissions, and unsupported assertions.
3. **Friction injection** — the detected issues are rendered as moderator commentary cards and injected into the next round's context, forcing agents to address them rather than repeating settled ground.
4. **Metadata-anonymized peer scoring** — each agent scores every other agent's response (never its own) without knowing which model or persona authored it. Scores are aggregated by median to resist outlier manipulation, with full voting transparency available per data point.

---

## Feature reference

### Supported providers and models

The built-in catalog covers 17 providers out of the box:

| Provider | Protocol |
|---|---|
| OpenAI | Native (GPT-4.1, o3, …) |
| Anthropic | Native (Claude Sonnet 4, …) |
| Google Gemini | Native (Gemini 2.5 Pro, …) |
| DeepSeek | OpenAI-compatible (V4 Flash, V4 Pro) |
| xAI | OpenAI-compatible (Grok 3) |
| OpenRouter | OpenAI-compatible + live model discovery |
| Groq | OpenAI-compatible + live model discovery |
| Together AI | OpenAI-compatible + live model discovery |
| Fireworks AI | OpenAI-compatible + live model discovery |
| SiliconFlow | OpenAI-compatible + live model discovery |
| Hugging Face Inference | OpenAI-compatible + live model discovery |
| Perplexity | OpenAI-compatible (Sonar Pro) |
| Alibaba DashScope | OpenAI-compatible (Qwen Plus) |
| Moonshot AI | OpenAI-compatible |
| Z.AI | OpenAI-compatible (GLM 5.1) |
| Inference.net | OpenAI-compatible |
| Custom OpenAI-compatible | Configurable endpoint and timeout |
| **Local Demo** | Credential-free offline mode, zero cost |

Providers that support live model discovery (OpenRouter, Groq, Together AI, SiliconFlow, Hugging Face) fetch their full model roster from the provider API and cache it locally. All other providers use a versioned local catalog to guarantee offline availability.

You can mix models from different providers freely — e.g., one agent on Claude, another on Grok, another on a local Ollama endpoint — within a single session.

---

### Agent and persona system

- **Minimum quorum:** one Orchestrator and two council members are required to start a session. The UI enforces this and disables the start button with a tooltip explaining why.
- **Model clones with distinct personas:** you can assign the same model to multiple agents but differentiate them with different personas, enabling multi-perspective brainstorming from a single API subscription.
- **Built-in persona library** (six cognitive archetypes):
  - *Devil's Advocate* — challenges assumptions, probes logic, exposes omissions
  - *Visionary Product Innovator* — prioritizes UX, simplicity, and disruptive thinking
  - *First-Principles Simplifier* — decomposes to fundamentals, removes jargon
  - *Pragmatic Strategist* — analyzes incentives, competitive position, and hidden risk
  - *Technical Architect* — evaluates design quality, performance, and failure modes
  - *Ethical Guardian* — assesses long-term consequences, fairness, and resilience
- **Custom personas:** create new archetypes by specifying a name, system prompt, and key directives; they are persisted locally and appear alongside built-ins.
- The Rust backend wraps each agent's prompt with its persona's instruction set before sending it to the provider.

---

### Gated lifecycle (state machine)

```
[Compose] → [Clarify] → [Approve Aspects] → [Round Loop] → [Review] → [Finalized]
```

**Phase 1 — Pre-session evaluation and clarification**
The Orchestrator scores the prompt's information density and flags ambiguity. If the ambiguity score crosses a threshold, it pauses and asks the user targeted follow-up questions. This loop repeats until the objective is clear, avoiding wasted API spend on vague prompts.

**Phase 2 — Aspect configuration and user gate**
The Orchestrator outputs 3–5 structured discussion aspects (e.g., Scalability, Security, Regulatory Compliance). The user must explicitly approve, reject, or edit them before any agent generation begins. Thumbs down returns the session to prompt editing without spending any tokens.

**Phase 3 — The round loop**
Parallel generation → contradiction analysis → friction injection → peer scoring (described above). After each round the lifecycle pauses at the Post-Round Command Center.

**Phase 4 — Dynamic context compaction**
Older rounds are summarized into high-density structural records by a fast model call, replacing raw transcripts. This prevents context window saturation and controls token costs across multi-round sessions.

**Phase 5 — Post-Round Command Center**
After each round the user sees:
- **Semantic similarity** — string-similarity percentage across agent responses; high similarity signals you need more diverse models or personas.
- **Consensus level** — score variance across agents; low variance signals general agreement, high variance highlights contested areas.

Available actions: inject new arguments, update the aspect matrix, export the current round, start the next round, or finalize.

**Phase 6 — Final synthesis**
The Orchestrator produces a comprehensive comparison summary with historical performance indexes, aggregated score matrices, and an option to export the complete session.

---

### Session import and export

| Format | Description |
|---|---|
| **Export Markdown (.md)** | Engineering-grade log with structured YAML frontmatter, chronological transcripts, agent identities, raw token/latency metadata per response, friction items, and full peer score matrices including individual votes and outlier flags. |
| **Export PDF (.pdf)** | Presentation-grade document compiled via the Rust-native Typst engine directly in the backend. Bypasses `window.print()` entirely to guarantee consistent cross-platform layout. Includes all transcript rounds, moderator friction blocks, peer score tables, radar charts, and a final synthesis section. |
| **Import Markdown** | Upload a previously exported session file to fully restore agent assignments, aspects, compacted history, and all round transcripts. The session can then continue from the exact lifecycle phase it was in. |

Exported files are written to the location you choose via a native file picker. No copies are retained by the application after export. The embedded state in Markdown files uses Base64-encoded JSON; imported sessions are validated against the current schema version.

---

### Crash recovery and checkpoint persistence

- Every state-machine transition boundary (after aspect approval, after each round, after scoring) triggers an **atomic checkpoint write** using a write-to-temp-then-rename strategy to prevent corruption from partial writes.
- On next launch, if an interrupted session is found, a recovery dialog offers: **Resume session** (restores to the last completed boundary) or **Discard and start fresh**.
- In-flight streams that had not completed at the time of interruption are discarded; the user can re-run that round from the Post-Round Command Center.

---

### Resilience and error handling

**Timeout configuration (four independent per-provider windows):**
| Timeout type | Default |
|---|---|
| Connection | 10 s |
| First token | 30 s (113 s for DeepSeek reasoning models) |
| Idle stream (between chunks) | 15 s |
| Total request wall-clock | 300 s |

**Retry policy:**
- Transient failures (HTTP 429, 500/502/503, network timeout): exponential backoff with jitter, up to 3 attempts.
- Permanent failures (HTTP 401, 403, invalid model ID): never retried; immediate user-facing diagnostic.
- Partial-stream retries: the notice log warns that already-consumed tokens may be re-billed.

**Mid-stream agent failure (graceful degradation):**
If one agent fails during a round, the remaining agents continue uninterrupted. The failed agent's panel shows an inline error badge. The Orchestrator's scoring matrix automatically adjusts to score only agents that delivered complete responses. A one-click **Retry failed agent** action re-runs just that agent against the same round context.

**Test Connection:**
Each provider has an explicit "Test Connection" button in Settings that performs a lightweight validation call (does not affect agents in session). Keys are always saved to the OS keychain regardless; untested keys display an "Untested" badge.

**User-facing error categorization:**
- *Informational* — "Model X is warming up, retrying…"
- *Warning* — "Agent 3 timed out; round continues with remaining agents"
- *Critical* — "All agents failed — check your API keys in Settings"

A collapsible diagnostic log panel (top bar → Log) shows all events for the session. All log entries are sanitized: API keys are redacted to `sk-****…****`, prompt content is truncated to the first 50 characters followed by `[REDACTED]`, and full response text is never persisted.

---

### Document and image ingestion

Accepted formats: **PDF, TXT, Markdown, CSV, JSON, DOCX, PNG, JPG, WEBP**

Resource limits:
- 20 files per session
- 25 MiB per file
- 100 MiB total batch
- 80 megapixels per image
- 2,000,000 extracted characters

Edge-case handling:
- **Scanned / image-only PDFs:** extraction is attempted; if fewer than 24 characters of trustworthy text are recovered the user is warned that OCR was inconclusive.
- **Password-protected PDFs:** rejected immediately with a specific diagnostic.
- **Complex DOCX tables:** nested or spanning-cell tables fall back to linearized text tagged with `[Table extraction approximate]`.
- **Context overflow:** if the total bundle (prompt + documents + images) exceeds the smallest assigned model's context window, the oldest/lowest-priority sections are condensed; the user is warned with the overflow amount in tokens and offered the option to remove files or switch to a larger-context model.
- **Malformed or corrupted files:** each file is reported individually; one bad file does not block the rest.
- **Unsupported types:** rejected at the drag-and-drop stage with a clear message listing supported formats.
- **Prompt injection via documents:** ingested content is clearly delimited from system instructions in the LLM prompt structure; executable content (scripts, macros) is stripped during extraction.

Images are Base64-encoded and routed to vision-capable models. If a model does not support vision and an image is attached, the user is notified to either remove the image or replace that model.

---

### Security and credential management

- **No keys in the frontend:** API keys are piped directly from the Settings form to the OS-native credential manager via the Rust `keyring` crate (Windows Credential Manager / macOS Keychain). They are never written to disk, localStorage, or sessionStorage, and are never returned to the webview layer.
- **Secrets are zeroized in memory** using the `zeroize` crate after use.
- **Endpoint validation (SSRF controls):** custom endpoints are validated before every request. HTTPS is required for remote hosts; plain HTTP is permitted only for loopback addresses (e.g., a local Ollama server). Link-local, metadata, multicast, and unspecified destinations are blocked.
- **Content Security Policy:** the webview runs under a strict CSP (`default-src 'self'`; no `eval`, no external scripts, no unrestricted network access).
- **Hard Clear:** Settings includes a "Wipe Credentials & Clear Cache" function backed by a confirmation dialog that explicitly lists every category of data it will delete:
  - All API keys from the OS credential manager (local removal only; keys must be separately revoked at the provider)
  - All cached model roster data
  - All temporary extraction files
  - All local session data, checkpoints, and compacted history
  - All exported document caches and Typst intermediates
  - In-memory buffers and runtime credential references

---

### Token and cost telemetry

A persistent tracker is accessible from the top bar (Usage button). It shows:
- Input and output token counts per model, with USD cost calculated from the local pricing tables
- Session totals across all models
- Resets automatically on new session or app launch
- Gracefully shows "N/A" for providers that do not return usage metadata; cost tracking never blocks the agent loop

---

### UI layout

- **Top bar:** brand lockup, live phase indicator with current round number, New Session, Usage, Log, and Settings controls.
- **Session Setup:** objective text area, multi-file drag-and-drop attachment zone, agent assignment table (provider + model + optional persona per slot), quorum indicator.
- **Clarification panel:** interactive Q&A loop with the Orchestrator.
- **Aspect gate:** editable aspect cards with thumbs-up / thumbs-down / inline rename controls.
- **Roundtable:** live streaming panels per agent, moderator friction card at center with type badges (⚡ Contradiction, 🕳️ Gap Detected, ⚠️ Unsubstantiated), interactive radar/spider charts powered by Recharts, per-data-point voting breakdown cards.
- **Zoom control:** 80 % – 150 % in-app zoom with Tauri webview native scaling; preference persisted to localStorage.
- **Info tooltips (ℹ):** glassmorphic overlay cards adjacent to every advanced concept (Ambiguity Score, Aspects, Semantic Similarity, Consensus, Cost metrics, Compaction) with fade-in micro-animation, keyboard accessibility, and aria labels.
- **Toast notifications:** live severity-categorized notices stacked in the bottom corner; dismissible individually.
- **Recovery dialog:** appears on launch if an interrupted session is found; non-dismissible until the user makes a choice.

---

## Developer reference

### Prerequisites

- Node.js 22+
- Rust 1.97 (pinned in `rust-toolchain.toml`)
- Tauri v2 platform prerequisites (see [tauri.app/start/prerequisites](https://tauri.app/start/prerequisites/))

### Run locally (dev mode)

```powershell
npm install
npm run tauri -- dev
```

The app starts with the zero-cost offline Local Demo council. Add provider API keys from **Settings**; they are written directly to the OS credential manager and never returned to the webview.

### Browser-only UI preview (no Rust compilation)

```powershell
npm run dev
```

### Verify and build

```powershell
npm test
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --all-targets
npm run tauri -- build --debug --no-bundle
```

### Architecture

| Path | Responsibility |
|---|---|
| `src/` | React UI, Zustand session state, Tauri IPC bridge, browser demo backend |
| `src-tauri/src/engine.rs` | Council state machine and concurrent orchestration |
| `src-tauri/src/providers.rs` | Streaming transports, timeout/retry policy, offline provider |
| `src-tauri/src/catalog.rs` | Built-in provider, model, and persona definitions |
| `src-tauri/src/security.rs` | Credential storage, error redaction, endpoint validation, SSRF controls |
| `src-tauri/src/ingestion.rs` | Bounded local document and image extraction |
| `src-tauri/src/checkpoint.rs` | Atomic session checkpoint write and schema-versioned restore |
| `src-tauri/src/report.rs` | Markdown export/import and headless Typst PDF compilation |
| `src-tauri/src/state.rs` | Shared application state (RwLock/Mutex), telemetry accumulation |
| `src-tauri/src/commands.rs` | Tauri IPC command handlers |

### Release packaging

Release installers are configured for **NSIS** on Windows and **DMG / .app** on macOS (`bundle.targets` in `tauri.conf.json`). Public distribution also requires platform code-signing credentials, macOS notarization, and Tauri's signed auto-update manifests — these are not stored in this repository.

---

## Current limitations

- **Production model catalog:** model IDs and prices in the local catalog are seed data; verify them before paid use. Runtime pricing-catalog updates are not yet signed or published.
- **Enterprise cloud adapters:** Azure AI Foundry, Amazon Bedrock, and Vertex AI require provider-specific authentication adapters not yet implemented.
- **OCR:** scanned-PDF OCR is attempted via `pdf-extract`; a high-confidence production OCR model is not bundled.
- **Code signing / notarization:** required before distributing to end users outside of dev mode; credentials must be supplied externally.
- **SSRF defense depth:** custom endpoint validation rejects known dangerous destinations; DNS-resolution pinning and redirect re-validation are future hardening items.
