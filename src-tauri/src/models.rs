use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

pub const SESSION_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LifecyclePhase {
    PreSession,
    Clarification,
    AspectGate,
    RoundRunning,
    PostRound,
    Finalized,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentRole {
    Orchestrator,
    Member,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Idle,
    Streaming,
    Complete,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NoticeSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WireProtocol {
    OpenAi,
    Anthropic,
    Gemini,
    Demo,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CredentialStatus {
    Configured,
    Untested,
    Valid,
    Invalid,
    NotRequired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutPolicy {
    pub connect_secs: u64,
    pub first_token_secs: u64,
    pub idle_stream_secs: u64,
    pub total_secs: u64,
    pub max_attempts: u32,
}

impl Default for TimeoutPolicy {
    fn default() -> Self {
        Self {
            connect_secs: 13,
            first_token_secs: 38,
            idle_stream_secs: 19,
            total_secs: 375,
            max_attempts: 3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSummary {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub protocol: WireProtocol,
    pub credential_status: CredentialStatus,
    pub supports_discovery: bool,
    pub configurable_endpoint: bool,
    pub timeout: TimeoutPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub provider_id: String,
    pub name: String,
    pub context_window: u64,
    pub input_per_million: Option<f64>,
    pub output_per_million: Option<f64>,
    pub supports_vision: bool,
    pub supports_documents: bool,
    pub supports_streaming: bool,
    pub reasoning: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Persona {
    pub id: String,
    pub name: String,
    pub description: String,
    pub system_prompt: String,
    pub directives: Vec<String>,
    pub builtin: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAssignment {
    pub id: String,
    pub role: AgentRole,
    pub display_name: String,
    pub provider_id: String,
    pub model_id: String,
    pub persona_id: Option<String>,
    pub status: AgentStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Aspect {
    pub id: String,
    pub name: String,
    pub description: String,
    pub weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttachmentStatus {
    Ready,
    Warning,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub id: String,
    pub name: String,
    pub path: String,
    pub media_type: String,
    pub bytes: u64,
    pub extracted_chars: usize,
    pub status: AttachmentStatus,
    pub warning: Option<String>,
    #[serde(default)]
    pub extracted_path: Option<String>,
    #[serde(default, skip)]
    pub extracted_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClarificationQuestion {
    pub id: String,
    pub prompt: String,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResponse {
    pub agent_id: String,
    pub content: String,
    pub status: AgentStatus,
    pub error: Option<String>,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub latency_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FrictionKind {
    Contradiction,
    Omission,
    UnsupportedClaim,
    Consensus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrictionItem {
    pub id: String,
    pub kind: FrictionKind,
    pub agent_ids: Vec<String>,
    pub aspect_id: Option<String>,
    pub explanation: String,
    pub challenge: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteDetail {
    pub voter_alias: String,
    pub score: f64,
    pub outlier: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreCell {
    pub agent_id: String,
    pub aspect_id: String,
    pub median: f64,
    pub votes: Vec<VoteDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundRecord {
    pub index: u32,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub responses: Vec<AgentResponse>,
    pub friction: Vec<FrictionItem>,
    pub scores: Vec<ScoreCell>,
    pub user_argument: Option<String>,
    pub semantic_similarity: Option<f64>,
    pub consensus: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub schema_version: u32,
    pub id: String,
    pub phase: LifecyclePhase,
    pub objective: String,
    #[serde(default)]
    pub main_phrase: String,
    pub ambiguity_score: Option<u32>,
    pub clarification_questions: Vec<ClarificationQuestion>,
    pub clarification_answers: HashMap<String, String>,
    pub aspects: Vec<Aspect>,
    pub agents: Vec<AgentAssignment>,
    pub attachments: Vec<Attachment>,
    pub rounds: Vec<RoundRecord>,
    pub compacted_history: Vec<String>,
    #[serde(default)]
    pub final_synthesis: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl SessionState {
    pub fn empty() -> Self {
        let now = Utc::now();
        let demo_agent = |id: &str, role: AgentRole, name: &str, persona: &str| AgentAssignment {
            id: id.to_string(),
            role,
            display_name: name.to_string(),
            provider_id: "demo".to_string(),
            model_id: "council-demo".to_string(),
            persona_id: Some(persona.to_string()),
            status: AgentStatus::Idle,
        };
        Self {
            schema_version: SESSION_SCHEMA_VERSION,
            id: Uuid::new_v4().to_string(),
            phase: LifecyclePhase::PreSession,
            objective: String::new(),
            main_phrase: String::new(),
            ambiguity_score: None,
            clarification_questions: vec![],
            clarification_answers: HashMap::new(),
            aspects: vec![],
            agents: vec![
                demo_agent(
                    "orchestrator",
                    AgentRole::Orchestrator,
                    "Moderator",
                    "strategist",
                ),
                demo_agent(
                    "member-1",
                    AgentRole::Member,
                    "Councilor 1",
                    "devils-advocate",
                ),
                demo_agent("member-2", AgentRole::Member, "Councilor 2", "architect"),
            ],
            attachments: vec![],
            rounds: vec![],
            compacted_history: vec![],
            final_synthesis: None,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelUsage {
    pub provider_id: String,
    pub model_id: String,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub input_cost_usd: Option<f64>,
    pub output_cost_usd: Option<f64>,
    pub total_cost_usd: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetrySnapshot {
    pub session_id: String,
    pub by_model: Vec<ModelUsage>,
    pub total_input_tokens: Option<u64>,
    pub total_output_tokens: Option<u64>,
    pub total_cost_usd: Option<f64>,
}

impl TelemetrySnapshot {
    pub fn empty(session_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            by_model: vec![],
            total_input_tokens: Some(0),
            total_output_tokens: Some(0),
            total_cost_usd: Some(0.0),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppNotice {
    pub id: String,
    pub severity: NoticeSeverity,
    pub title: String,
    pub message: String,
    pub details: Option<String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapPayload {
    pub session: SessionState,
    pub telemetry: TelemetrySnapshot,
    pub providers: Vec<ProviderSummary>,
    pub models: Vec<ModelInfo>,
    pub personas: Vec<Persona>,
    pub notices: Vec<AppNotice>,
    pub recoverable_checkpoint: bool,
    pub app_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundPollPayload {
    pub session: SessionState,
    pub telemetry: TelemetrySnapshot,
    pub notices: Vec<AppNotice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreflightInput {
    pub objective: String,
    pub agents: Vec<AgentAssignment>,
    pub attachment_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    pub session_id: String,
    pub round_index: u32,
    pub agent_id: String,
    pub delta: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportResult {
    pub path: String,
    pub bytes: u64,
}

#[derive(Debug, Clone)]
pub struct ImagePayload {
    pub media_type: String,
    pub data_base64: String,
}

#[derive(Debug, Clone)]
pub struct CompletionRequest {
    pub system: String,
    pub prompt: String,
    pub model: String,
    pub max_tokens: u32,
    pub temperature: f32,
    pub thinking_enabled: Option<bool>,
    pub images: Vec<ImagePayload>,
}

#[derive(Debug, Clone, Default)]
pub struct CompletionResult {
    pub content: String,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
}
