Master Implementation Plan: Multi-Agent Brainstorming Roundtable called Agentic Council 

1. System Architecture & Tech Stack
Goal: A blazing-fast, secure, cross-platform desktop application requiring zero user-managed dependencies, optimized for both macOS and Windows.
	Application Shell: Tauri v2 (Rust backend for heavy lifting, OS-native webview for rendering).
	Frontend: Vite + React + TypeScript + Tailwind CSS.
	Charts/Visualization: Recharts (for radar/spider charts and matrices).
	State Management: Zustand (for fast, reactive frontend state) + Rust Mutex/RwLock (for backend agent loop state).
	Document Compilation: Rust-native typst crate for deterministic, high-fidelity PDF rendering.
	API Core Architecture: Extensible unified API layer implemented in Rust via reqwest + tokio. Instead of a single client, a polymorphic provider trait abstraction routes structured streaming data across diverse public and private model endpoints.

2. Multi-Provider API Integration & Model Management
The application offers native support for an expandable catalog of cloud-hosted and custom inference providers. Users can seamlessly mix, match, and assign specific models to individual agents at the roundtable.
Supported API Ecosystem
	Primary Frontier Foundations: OpenAI API, DeepSeek API, Anthropic API, Google Gemini API, xAI API.
	Aggregators & Routers: OpenRouter, Groq, Together AI, Fireworks AI, SiliconFlow, Hugging Face Inference API.
	Specialized & Enterprise Platforms: Microsoft Azure AI Foundry, Amazon Bedrock, Google Vertex AI, Perplexity API, Alibaba Cloud (DashScope), Moonshot AI API, Z.AI API, Inference.net.
Model Discovery & Pricing Engine
	Unified Roster Resolution: On launch, the backend attempts to fetch dynamic, real-time model rosters for metadata-supported aggregators (e.g., OpenRouter, Together AI). For structured standalone APIs, the app utilizes localized, version-controlled JSON schema configs to guarantee offline availability.
	Metrics Mapping: The app standardizes disparate provider metadata to display token metrics uniformly, such as Blended Price per 1M Tokens (USD) [Info Tooltip: "An estimated pricing metric calculated using a standard ratio of input to output tokens (typically 3:1) to simplify cost comparisons across different providers."] to guide model selection for specific agent assignments.
Persona-Enabled Agent Customization
	Optional Persona Selection: Alongside the model selector dropdown for each agent, the UI includes an optional selection bar to assign a specific persona.
	Brainstorming with Model Clones: Users can assign the exact same model to multiple agents at the table but differentiate them using distinct personas [Info Tooltip: "Allows instantiating multiple agents running the exact same model under different archetype-based personas (e.g., Devil's Advocate, First-Principles Thinker), enabling you to explore diverse angles of a problem using a single premium API."], allowing users to leverage a single high-quality model for diverse, multi-perspective brainstorming.
	Minimum Quorum Rules: A valid council session requires at least one Orchestrator agent and two council member agents. Peer scoring is not meaningful with only a single agent, and two agents provide very limited independent evaluation. The UI enforces this minimum by disabling the session start button until the quorum threshold is met, with a tooltip explaining the requirement.
	Built-in Archetype-Based Persona Library: Personas are defined as configurable thinking archetypes rather than imitations of real people. Each archetype encapsulates a cognitive style and evaluation lens. Users can customize the name, system prompt, and behavioral parameters of any archetype, and create entirely new custom archetypes.
		The Devil's Advocate: Questions assumptions, probes logic, exposes omissions, and challenges the consensus.
		The Visionary Product Innovator: Prioritizes user experience, simplicity, aesthetic detail, and disruptive thinking.
		The First-Principles Simplifier: Explains complex concepts in simple terms, identifies unnecessary jargon, and anchors analysis to fundamental truth.
		The Pragmatic Strategist: Analyzes power dynamics, competitive advantages, hidden risks, and pragmatic viability.
		The Technical Architect: Evaluates structural design, computational complexity, scalability, and code quality.
		The Ethical Guardian: Assesses long-term consequences, ethical concerns, balance, and resilience.
	Custom Archetype Creation: Users can define new persona archetypes via a creation form specifying the archetype name, a system prompt describing the cognitive lens, and key behavioral directives. Custom archetypes are persisted locally and appear alongside built-in archetypes in the persona selector.
	Backend Persona Wrapper: When a persona is selected, the Rust backend wraps the agent prompt with the specialized archetype instruction set before constructing the payload sent to the LLM endpoint.

3. Core Security & Credential Ledger
	API Key Storage: API keys must never persist in the volatile frontend webview (localStorage or sessionStorage). All credentials are dynamically piped straight to the OS-native credential manager via the Rust keyring crate (Windows Credential Manager / macOS Keychain) [Info Tooltip: "For maximum security, API keys are never stored in the browser frontend or local storage. They are written directly to your operating system's native secure keychain (Windows Credential Manager / macOS Keychain)."].
	Isolated Execution: The Rust backend uniquely initializes client configurations using securely pulled vault credentials. The frontend remains fully decoupled, receiving only clean text chunks via Tauri's IPC (tauri::Window::emit).
	Hard Clear Feature: The settings module features an immutable "Wipe Credentials & Clear Cache" function with a confirmation dialog that explicitly itemizes what will be deleted:
		All stored API keys from the OS-native credential manager (note: this removes local keys only and does not revoke keys at the provider — users must revoke keys from their provider dashboards independently).
		All cached model roster data and pricing tables.
		All temporary files generated during document ingestion (extracted text, OCR intermediates).
		All local session data including saved roundtable state, checkpoint files, and compacted history.
		All exported document caches and intermediate Typst compilation artifacts.
		In-memory buffers and any runtime credential references.
	The confirmation dialog presents this list to the user before execution so the scope of the wipe is fully transparent.

4. Privacy & Threat Model
All user data is processed and retained locally. The application does not operate any remote telemetry, analytics, or cloud storage service.
	Prompts & Documents: User prompts, attached documents, and extracted text remain on the local filesystem and are never transmitted to any endpoint other than the user-selected LLM provider APIs. Temporary extraction artifacts (OCR intermediates, parsed text) are stored in a sandboxed temp directory and purged at session end or via the Hard Clear function (Section 3).
	API Responses & Logs: Model responses are held in local session state only. The application maintains an optional diagnostic log file for troubleshooting. All log entries are sanitized before writing: API keys are fully redacted (replaced with a masked placeholder, e.g., sk-****…****), and prompt content is truncated to the first 50 characters with a [REDACTED] suffix to prevent sensitive user input from persisting in log files.
	Exports: Exported Markdown and PDF reports are written exclusively to the user's chosen local directory. No copies are retained by the application after export.
	Session Checkpoints: Checkpoint and auto-save files (Section 5) are stored in the application's local data directory. They contain serialized session state including round transcripts and aspect matrices but never contain raw API keys.
	Network Boundary: The Rust backend communicates only with explicitly user-configured LLM provider endpoints. No other outbound network calls are made except for optional model roster fetches from provider APIs (Section 2). All network requests use TLS.
	Threat Mitigations:
		Credential Exfiltration: API keys are stored in the OS-native credential manager (Section 3) and are never written to disk, logs, or frontend state.
		Prompt Injection via Documents: Ingested documents are treated as data-only context. The extraction pipeline strips executable content (scripts, macros) and the document payload is clearly delimited from system instructions in the LLM prompt structure.
		Local Data at Rest: Session files and checkpoints are stored unencrypted on the local filesystem, relying on OS-level user account protections. Users handling highly sensitive data are advised to enable full-disk encryption.

5. Cancellation, Crash Recovery & Checkpoint Persistence
Users must be able to stop an active round at any point without leaving unfinished provider requests or corrupting the session.
	User-Initiated Cancellation: A prominent "Stop Round" button is visible in the UI during any active round. Pressing it triggers the following sequence:
		All in-flight HTTP streams to LLM providers are immediately aborted via tokio::CancellationToken propagation to every spawned request task.
		Partially received agent responses are marked with a "Cancelled" status and retained in session state for reference but excluded from scoring.
		The session state machine transitions back to the Post-Round Command Center (or Pre-Session if cancellation occurs during Phase 1/2), leaving the session in a clean, resumable state.
		No orphaned background tasks or dangling provider connections remain after cancellation.
	Checkpoint Persistence: The backend automatically writes a checkpoint file to the local data directory at every state-machine transition boundary (e.g., after aspect approval, after each round completion, after scoring). Checkpoints are atomic writes using a write-to-temp-then-rename strategy to prevent corruption from partial writes. Each checkpoint captures: the current lifecycle phase, round index, agent assignments, approved aspects, compacted history, all completed round transcripts, and scoring matrices.
	Crash Recovery: On application launch, the backend checks for the presence of a checkpoint file. If one is found from an interrupted session, the user is presented with a recovery dialog offering two options:
		"Resume Session": Restores the full session state from the checkpoint and returns the user to the exact lifecycle phase where the interruption occurred.
		"Discard & Start Fresh": Deletes the checkpoint file and proceeds to a clean new session.
	If the application crashes mid-stream (e.g., during Stage 1 parallel generation), the last completed checkpoint is the recovery point. Any in-flight responses that were not fully received are discarded, and the user can re-run the interrupted round from the Post-Round Command Center.

6. Error Handling & Resilience Strategy
Robust fault tolerance is critical for a multi-provider, multi-agent system where external API failures are not a question of "if" but "when."
	Granular Timeout Configuration: Each API provider is assigned independent, configurable timeout windows across four distinct timeout types:
		Connection Timeout: Maximum time to establish a TCP/TLS connection to the provider endpoint (default: 10 seconds).
		First-Token Timeout: Maximum time to receive the first token after the request is sent. Frontier reasoning models (e.g., DeepSeek R1, o3) that routinely take 30–90+ seconds for first-token delivery receive extended thresholds, while standard chat models default to a shorter window (e.g., 30 seconds).
		Idle-Stream Timeout: Maximum time between consecutive chunks during an active stream (default: 15 seconds). Detects stalled connections where the provider stops sending data without closing the stream.
		Total Request Timeout: Absolute maximum wall-clock time for the entire request lifecycle (default: 300 seconds). Acts as a hard ceiling regardless of other timeout states.
	If any timeout is exceeded, the request is terminated gracefully without blocking the entire round.
	Retry Policy with Exponential Backoff: Transient failures (HTTP 429 rate limits, 500/502/503 server errors, network timeouts) trigger an automatic retry sequence using exponential backoff with jitter (e.g., 1s → 2s → 4s, ±random offset). Each provider enforces a maximum retry cap (default: 3 attempts). Permanent failures (HTTP 401 unauthorized, 403 forbidden, invalid model ID) are never retried and immediately surface a diagnostic error to the user.
		Partial-Stream Retry Warning: If a retry is triggered after a partially streamed response (i.e., some tokens were already received before a failure), the system logs a warning that the retried request may result in duplicate API charges for the already-consumed tokens. The user-facing error log notes this possibility so users are aware of potential cost implications.
	Mid-Stream Agent Failure & Graceful Degradation: If an agent's stream drops or errors out during an active round, the system applies a degraded-quorum policy rather than aborting the entire round. The failed agent's panel displays a clear inline error badge (e.g., "Agent failed: timeout after 60s — response excluded from scoring"). The remaining agents continue uninterrupted, and the Orchestrator's scoring matrix automatically adjusts to evaluate only the agents that delivered complete responses. The user is offered a one-click "Retry Failed Agent" action in the Post-Round Command Center to re-run the failed agent against the same round context without repeating the entire cycle.
	API Key Validation — Optional "Test Connection": Rather than automatically validating API keys on every save (which consumes provider quota and may incur cost), the settings UI provides an explicit "Test Connection" button adjacent to each API key input. When clicked, the Rust backend performs a lightweight validation request (e.g., a list-models or minimal-completion call) against the provider. If validation fails, a diagnostic message is displayed (e.g., "Invalid key", "Insufficient permissions", "Quota exceeded"). Keys are always saved to the OS keychain regardless of whether the user chooses to test them, but untested keys display a subtle "Untested" badge in the settings panel.
	User-Facing Error Notifications: All backend errors are translated into human-readable, non-technical toast notifications in the frontend. Errors are categorized by severity: informational (e.g., "Model X is warming up, retrying..."), warning (e.g., "Agent 3 timed out, round continues with remaining agents"), and critical (e.g., "All agents failed — check your API keys in Settings"). A collapsible error log panel is accessible from the top bar for users who want raw diagnostic details.

7. Global Telemetry & Cost Tracker
To provide full visibility into API consumption, a persistent telemetry module is anchored to the top navigation bar of the application.
	Top-Bar Tracker Dashboard: Clicking the tracker opens a summarized, real-time table displaying cumulative consumption metrics for the active session and also each model separately. 
	Data Aggregation: The Rust backend parses the usage metadata returned by each API payload, tracking:
	Input Tokens (per model) and also including the cost of each input tokens (per model)
	Output Tokens (per model) and also including the cost of each output tokens (per model)
	Approximate USD Cost (calculated dynamically using the cached provider pricing tables mapping).
	Lifecycle Management: This tracker represents current session data only. It automatically resets to zero upon application launch or when the user initiates a completely new brainstorming session.
	Failsafe Execution: Cost tracking is implemented as a non-blocking background thread. If a specific provider's API does not return token usage data, the tracker gracefully displays "N/A" for that specific node without interrupting the primary Orchestrator or UI loops.

8. Universal Ingestion & Multimodal Processing
Beside standard plaintext prompt queries, the system accepts rich multi-format payloads to build a comprehensive data bundle before activating the orchestrator.
	Multimodal Image Parsing: Users can attach multiple visual data (PNG, JPG, WEBP). The Rust backend encodes these assets and routes them to vision-capable Orchestrator and council models (if there is/are models that doesn't accept image or any specific type of file and then user upload that specific type of files, the app must let the user know to either remove the attached file or change those models not supporting the file), allowing the agents to "see" architectural diagrams, charts, or sketches alongside text.
	Document Pipeline: Supports multi-file drag-and-drop (PDF, TXT, MD, CSV, JSON, DOCX). An advanced background extraction layer (supporting tables in docs) parses structural document text locally before bundling.
	Document Ingestion Edge-Case Handling: The ingestion pipeline must explicitly handle the following failure modes and edge cases:
		Scanned / Image-Only PDFs: If a PDF contains no extractable text layer, the pipeline attempts OCR via a local Rust-native OCR engine (e.g., leptess/tesseract bindings). If OCR fails or produces low-confidence output (below a configurable threshold), the user is notified with a warning: "This PDF appears to be scanned. OCR extraction was attempted but may contain errors. Please verify the extracted text."
		Large Files & Context-Size Overflow: Each ingested file's extracted text is measured against the target model's context window budget. If the total context bundle (prompt + all documents + images) exceeds the allocated context budget, the system applies a prioritized truncation strategy: (1) older/lower-priority documents are summarized first, (2) the user is warned with the exact overflow amount and given the option to remove files, switch to a larger-context model, or accept the truncated bundle.
		Unsupported or Complex Tables: Tables in PDFs and DOCX files are extracted with best-effort structural preservation. If the extraction engine detects a table it cannot reliably parse (e.g., deeply nested, merged cells, spanning pages), it falls back to a linearized plain-text representation and tags the output with a "[Table extraction approximate]" marker.
		Malformed or Corrupted Files: Files that fail to parse (corrupted PDFs, invalid JSON, password-protected documents) are rejected with a specific diagnostic message per file rather than silently dropping or crashing the entire ingestion batch. The user sees which files succeeded and which failed, with actionable error messages.
		Unsupported File Types: Files with unrecognized extensions are rejected at the drag-and-drop stage with a clear message listing the supported formats.
	Payload Structuring: The compiled prompt plus the extracted text/image payload is structured into an optimized context bundle object, which acts as the foundational ground-truth directory for the Orchestrator and competing agents.

9. Session Resumption & High-Fidelity Reporting
[Import MD Session] -> Restores Full Context, Aspects, & Transcript History
[Export MD Report]  -> Raw Markdown, metadata headers, and formatted structural text
[Export PDF Report] -> Presentation-grade document via Rust-compiled Typst
Session Resumption (Import)
	Before starting the first round of a debate, users can upload an existing session Markdown file via a native file picker (tauri-plugin-dialog).
	The system parses structured frontmatter metadata containing past agent assignments, generated discussion aspects, similarity configurations, and historical round transcripts, completely reconstructing the local application state to seamlessly resume old brainstorming sessions.
Report Generation (Export)
At the end of any individual discussion round or during the final synthesis phase, users can trigger an on-demand export:
	Markdown (.md) Export: Generates an engineering-focused log documenting structural system prompt details, chronological round transcripts, agent identities, and raw evaluation matrices.
	Rust-Sourced PDF Compiling: The standard frontend window.print() is entirely bypassed to prevent cross-platform layout distortion. Instead, the Rust backend leverages the typst compilation crate. It feeds the transcript, charts, and matrices into a headless print pipeline, outputting a highly professional, typographically accurate, and cleanly paginated PDF executive summary directly to the user's local filesystem.

10. The UI / UX Layout: "The Roundtable"
	Visual Concept: The main workspace presents a modern, minimal top-down view of a council roundtable — clean and intuitive, designed so council members (agents) are seated around the table in a brainstorming session. The aesthetic is contemporary and uncluttered, prioritizing readability and ease of use.
	The Actors: The Orchestrator Agent occupies the head of the roundtable as the session moderator, while user-selected council member avatars (labeled with their assigned models and optional personas) are arranged around the table, each with their own dedicated response panel.
	Stream Optimization: Split-terminal nodes display live model streaming text without stuttering, using performance-tuned UI bindings to decouple heavy layout repaints from incoming data buffers.
	Info Indicator ("i") Tooltips: Reusable info indicators (small circular "i" icons with a subtle glow) are placed adjacent to all advanced terminology, complex configurations, or metric calculations.
		Visual Design: Clean, low-profile inline icons using Lucide icons (`Info` or `HelpCircle`) styled with a subtle indigo or amber hover-glow.
		Interaction Model: Clicking the icon opens a responsive, absolute-positioned glassmorphic tooltip card that overlays the parent element. It features a micro-animation (fade-in & slide-up) and can be dismissed by clicking a close button, clicking outside, or pressing Escape.
		Accessibility & UX: Fully focusable via keyboard (`Tab` indexing) with screen-reader friendly descriptive tags (`aria-label`) and high-contrast color styling.

11. The Gated Debate Lifecycle & State Machine
[User Input + doc files (optional)+ Images(optional)] 
       │
       ▼
[Orchestrator Evaluation] ──► Needs Clarity? ──► [Interactive Follow-up Loop]
       │                                                      │
       ├────────────────◄─────────────────────────────────────┘
       ▼
[Aspect Generation] ──► [User Gate: Thumbs Up / Down / Adjust(add/remove) Aspects by text input]
       │
       ▼ Approved
[The Round Loop] ──► [Dynamic Context Compaction(with summarized bullet points)] ──► [Checkpoint / Export Option]

Phase 1: Pre-Session Evaluation & Clarification Loop
	Payload Ingestion: The user submits their initial query alongside optional context files and images.
	Ambiguity Scoring: The Orchestrator conducts a rapid pre-flight analysis of the payload to score information density and logical gaps. An info "i" icon is provided adjacent to the Ambiguity Score [Info Tooltip: "The Orchestrator's initial assessment of your prompt's clarity. If the score is low, the app prompts you with clarifying questions to avoid wasting API tokens on vague requirements."].
	Interactive Clarification: If ambiguity crosses an established threshold, the Orchestrator pauses progression and presents targeted, multi-part follow-up questions to the user. This interactive dialog loops until the query's criteria are clear.

Phase 2: Aspect Configuration & Gated Controls
	Structural Generation: Once the objective is finalized, the Orchestrator outputs a structured JSON matrix mapping out 3–5 Discussion Aspects (e.g., Scalability, Regulatory Compliance, Complexity). An info "i" icon is attached to the Aspects title [Info Tooltip: "The evaluation dimensions (e.g., Scalability, Security) that the Orchestrator uses to score the agents. You can modify these to focus the debate on specific objectives."].
	The User Gate: The UI locks progression at a critical checkpoint. The user interacts with the generated roadmap using explicit control elements:
	Thumbs Up: Confirms current configurations, aspects, and structural bounds, initiating the live roundtable session.
	Thumbs Down / Stop: Halts execution, blocks model processing, and returns the application state back to prompt modification to avoid unnecessary API consumption.
	Manual Refinement: Users can add, rename, or delete aspects inline before giving execution authorization.

Phase 3: The Round Loop & Forceful Friction
This phase is the core engine of Agentic Council. Each round executes a four-stage pipeline: Parallel Generation → Contradiction Analysis → Friction Injection → Structured Scoring.

	Stage 1 — Parallel Independent Generation:
	All participating agents receive an identical state packet containing the finalized objective, the approved aspect matrix, the compacted history of prior rounds (if any), and any user-injected arguments. Agents generate their responses concurrently and independently — no agent sees another's output during generation. This eliminates anchoring bias and ensures each perspective is genuinely independent. The frontend renders each agent's streamed response in its dedicated panel in real time.

	Stage 2 — Post-Round Contradiction & Gap Analysis:
	Once all agent streams have completed (or timed out per the resilience policy in Section 4), the Orchestrator executes a dedicated analysis pass. This is a separate LLM call where the Orchestrator receives the full text of every agent's response for the current round and is prompted to perform a structured comparative review. The Orchestrator's analysis prompt instructs it to:
		Identify Direct Contradictions: Flag cases where two or more agents make mutually exclusive claims or recommend incompatible approaches (e.g., Agent A recommends a microservices architecture while Agent B argues for a monolith, without either acknowledging the trade-off).
		Detect Analytical Omissions: Identify critical dimensions of the problem that no agent addressed (e.g., all agents discussed scalability but none addressed regulatory compliance, which is a defined aspect).
		Surface Unsupported Assertions: Highlight claims made without reasoning or evidence — statements that read as assumptions rather than argued positions.
		Map Consensus vs. Divergence: Categorize which aspects have strong agreement across agents and which remain contested, producing a structured divergence map.
	The Orchestrator outputs this analysis as a structured JSON object containing an array of friction items, each tagged with a type (contradiction, omission, unsupported_claim, consensus), the involved agent IDs, the relevant aspect, and a concise natural-language explanation.

	Stage 3 — Friction Injection & Moderator Commentary:
	The parsed friction items are rendered in the UI as a distinct Orchestrator commentary block, visually separated from agent responses (e.g., a highlighted moderator card at the center of the roundtable, using a differentiated color accent). Each friction item is displayed as a digestible card with:
		A friction-type badge (e.g., "⚡ Contradiction", "🕳️ Gap Detected", "⚠️ Unsubstantiated").
		The specific agents involved, linked to their response panels for quick cross-reference.
		The Orchestrator's synthesized challenge or question (e.g., "Agents 1 and 3 propose conflicting database strategies. Neither addresses failover — how does each approach handle a region-level outage?").
	This friction commentary is then injected into the context for the next round, ensuring that agents in Round N+1 are forced to directly address the identified contradictions and gaps rather than repeating stale arguments.

	Stage 4 — Metadata-Anonymized Round-Robin Peer Scoring:
	Rather than relying on the Orchestrator as a single evaluator (which introduces single-point bias), the system conducts an internal metadata-anonymized peer voting round. Each council agent independently scores every other agent's response — but never its own — without being told which model or persona authored it. This produces evaluation scores that are free from explicit identity bias, though it should be noted that agents may still infer authorship from writing style, vocabulary patterns, or content signatures. The anonymization removes structural metadata cues but cannot guarantee complete blindness.
		Metadata-Anonymization Layer: Before distributing responses for peer review, the Rust backend strips all identifying metadata (model name, persona label, agent slot index) from each response and assigns a randomized temporary alias (e.g., "Response Alpha", "Response Beta"). This prevents agents from gaming scores based on perceived model reputation or persona alignment. Note: this is metadata-level anonymization — the content of responses is not altered, so stylistic inference of authorship remains theoretically possible.
		Peer Voting Prompt: Each agent receives the full set of anonymized peer responses alongside the approved aspect matrix and is prompted with a strict scoring rubric:
			Scoring Scale (0–10 per aspect per response): 0–2 (Critical gaps or factually incorrect), 3–4 (Superficial treatment, major omissions), 5–6 (Adequate but generic, lacks depth), 7–8 (Strong analysis with supporting reasoning), 9–10 (Exceptional insight, novel perspective, directly actionable).
			Anti-Verbosity Bias Rule: The rubric explicitly instructs each voting agent that response length must not influence scoring. A concise, precise 200-word response that directly addresses the aspect should score higher than a verbose 800-word response that circles around it without adding substance.
			Self-Exclusion Enforcement: Each agent's own response is excluded from the batch it receives for scoring, ensuring no agent ever evaluates itself.
		Structured Output: Each voting agent returns its scores as a JSON matrix: { "response_alias": { "aspect_name": score, ... }, ... }.
		Orchestrator Aggregation & Outlier Detection: The Orchestrator collects all peer vote matrices and computes the final score for each agent-aspect pair by taking the median of all peer scores (median is preferred over mean to resist outlier manipulation). If any individual peer vote deviates more than ±3 points from the median for a given aspect, it is flagged as an outlier and annotated in the voting transparency log but still included in the median calculation. The Orchestrator then maps the randomized aliases back to real agent identities and publishes the aggregated score matrix to the frontend.
		Frontend Scoring Transparency: The aggregated scores update the Recharts radar/spider charts in real time. Each data point on the radar chart is interactive — clicking or hovering reveals a voting breakdown card showing: the final median score, each peer's individual score (still anonymized by voting alias to prevent retaliation bias in subsequent rounds), and any outlier flags. This gives users full transparency into how consensus scores were formed without compromising the anonymity of the voting process.

	Round Progression Logic:
	After scoring completes, the lifecycle transitions to Phase 5 (Post-Round Command Center). If the user chooses to continue to Round N+1, the next round's state packet includes: (a) the compacted history, (b) the Orchestrator's friction items as explicit challenges the agents must address, and (c) any user-injected arguments. This creates a directed evolutionary pressure where each successive round is forced to resolve prior weaknesses rather than rehashing settled ground.

Phase 4: Dynamic Context Compaction
	To minimize context window saturation and control API overhead costs, the backend triggers automatic background pruning. An info "i" icon next to the Compaction status explains the process [Info Tooltip: "A background optimization that condenses older debate rounds into high-density summaries. This prevents context window bloat and controls API token consumption."]. Rounds N-2 and N-1 are summarized by a fast model into high-density structural records containing resolved definitions and lingering friction points, replacing bloated raw transcripts in preceding prompt indices.

Phase 5: The Post-Round Command Center
	At the end of every round, the lifecycle pauses to evaluate metrics and give the user full control over next steps:
	Analysis Metrics: Displays embeddings/semantic comparison (Semantic Duplicate Detection) [Info Tooltip: "Measures the text similarity between agent responses. A high percentage indicates that models are repeating each other, signaling that you may need more diverse models or personas."] percentages (calculating structural drift using fast string similarity calculations) and Consensus Levels [Info Tooltip: "Measures agreement among the council agents based on the variance of their evaluation scores. High consensus means general alignment; low consensus highlights key areas of debate."] (tracking score variances between active models). Both metrics feature info "i" icons for immediate user clarification.
	User Actions: The user can instantly inject new arguments into the table, update the aspect matrix, execute an export of the current round (MD or PDF), progress to the next iteration, or finalize and freeze the roundtable.

Phase 6: Final Synthesis
	Upon finalization, the Orchestrator generates a comprehensive comparison summary, outputting historical charts, final aggregated performance indexes, filterable parameter rankings, and an open gateway to save the complete session.

12. Log Sanitization & Diagnostic Logging
	The application maintains an optional diagnostic log file for advanced troubleshooting. All log entries are sanitized before writing to prevent sensitive data persistence:
		API Key Redaction: Any string matching known API key patterns (e.g., sk-*, key-*, Bearer tokens) is replaced with a masked placeholder (e.g., sk-****…****).
		Prompt Content Redaction: User prompt content in log entries is truncated to the first 50 characters followed by a [REDACTED] marker. Full prompt text is never written to the log file.
		Response Content: Model response content is logged only at summary level (token count, latency, status) — full response text is not persisted in logs.
	Log files are stored in the application's local data directory and are included in the scope of the Hard Clear function (Section 3).


13. Implementation Milestones for coding
	Step 1: Initialize Tauri v2 shell using Vite/React/TS and Tailwind. Configure the filesystem plugin permissions.
	Step 2: Build out the Rust backend unified multi-provider API layer, constructing the extensible provider traits, structural HTTP streaming clients with granular timeout types (connection, first-token, idle-stream, total), archetype-based persona prompt wrapping templates, and secure keyring storage modules including clear/delete routines.
	Step 3: Implement the Top-Bar Telemetry Tracker state layer for real-time input/output token mapping and approximate USD cost calculations.
	Step 4: Implement native document and multimodal image ingestion pipelines, setting up text/vision extractors with OCR fallback, malformed file handling, context-overflow detection, and structuring contextual prompt bundles.
	Step 5: Code the complete backend agent state machine with cancellation support (tokio::CancellationToken), checkpoint persistence at state-machine boundaries, and crash recovery on launch. Build out the pre-flight ambiguity analyzer, interactive clarification loops, and user gating systems.
	Step 6: Build out the frontend "Roundtable" UI layout, connecting streaming Tauri IPC hooks, interactive Zustand matrices, model/persona selection dropdowns, quorum enforcement, and radar graph views.
	Step 7: Implement session state export routines (saving/loading models and assigned personas) using Rust's typst crate for high-fidelity PDFs and Markdown, alongside the frontmatter markdown reader for session imports.
	Step 8: Testing & Release Hardening:
		Provider Contract Tests: Validate each provider trait implementation against API contracts (request format, streaming protocol, error shapes) using recorded HTTP fixtures.
		Retry / Timeout / Failure Tests: Simulate all four timeout types, transient failures, permanent failures, partial-stream failures, and verify correct retry behavior, backoff timing, and graceful degradation.
		State-Machine Tests: Verify every state transition in the debate lifecycle, including cancellation from every phase, checkpoint write/restore correctness, and crash recovery scenarios.
		Import/Export Round-Trip Tests: Assert that exporting a session to Markdown and re-importing it produces identical application state (agent assignments, aspects, transcripts, scores).
		PDF Visual Regression Tests: Render Typst-compiled PDFs from reference session data and compare pixel-level output against baseline snapshots to catch layout regressions.
		Cross-Platform Testing: Full test suite execution on both Windows and macOS, verifying OS-native credential manager integration, file system paths, and Tauri plugin behavior on each platform.
	Step 9: Code signing, notarization (macOS), installer packaging (NSIS for Windows, DMG for macOS), and auto-update strategy (Tauri's built-in updater with signed update manifests). Compile, verify optimization flags, and package zero-dependency production builds (.exe, .app, and .dmg).

14. Necessary Production-Hardening Additions
	These controls are required because the product accepts secrets, arbitrary documents, custom network endpoints, and restorable local state. They are treated as part of the implementation plan rather than optional polish.
	Network Egress & SSRF Controls: Validate custom endpoints before every request, allow plaintext HTTP only for explicit loopback development, reject metadata/link-local destinations, revalidate redirects, and pin the resolved destination for the lifetime of a request to prevent DNS rebinding.
	Versioned State Migrations: Every checkpoint and exported session carries a schema version. Import must reject unsupported future schemas, migrate supported older schemas deterministically, and never partially apply corrupt state.
	Resource Budgets: Enforce per-file, aggregate-upload, decompression, extracted-text, image-dimension, context, and report-generation limits. Surface truncation and unsupported-content warnings before any paid provider call.
	Webview Boundary: Keep a restrictive CSP and least-privilege Tauri capabilities. The webview must never receive API-key values, raw keyring handles, unrestricted filesystem access, or generic network primitives.
	Deterministic Offline Mode: Ship a credential-free local demo backend so the complete gated lifecycle and UI can be exercised without network access or API charges.
	Supply Chain & Release Trust: Pin the Rust toolchain, lock JavaScript/Rust dependencies, run Windows and macOS CI, generate signed update manifests, and require platform signing/notarization before public distribution.
	External-Dependency Gates: Enterprise cloud authentication, production OCR models, live provider contract fixtures, and signing keys must each have explicit readiness checks. The application must describe the unavailable capability accurately instead of silently degrading or claiming success.
