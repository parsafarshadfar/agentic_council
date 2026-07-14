use crate::{
    catalog, checkpoint, engine, ingestion,
    models::*,
    providers::{discover_models, make_provider},
    report,
    security::{sanitize_prompt_for_log, validate_endpoint},
    state::{AppPaths, AppState},
};
use std::{
    collections::{HashMap, HashSet},
    fs,
    future::Future,
    path::PathBuf,
    time::Duration,
};
use tauri::{AppHandle, Manager, State};
use tokio_util::sync::CancellationToken;

type CommandResult<T> = Result<T, String>;

/// Tauri's synchronous command handlers run on WebView2's main COM callback on
/// Windows, outside an entered Tokio context. Spawning with `tokio::spawn`
/// there panics and the panic cannot unwind through the COM boundary, which
/// Windows reports as STATUS_STACK_BUFFER_OVERRUN (0xc0000409). Always spawn
/// long-lived command work through Tauri's runtime handle instead.
fn spawn_on_tauri_runtime<F>(future: F) -> tokio::task::JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    tauri::async_runtime::handle().inner().spawn(future)
}

#[tauri::command]
pub fn bootstrap(state: State<'_, AppState>) -> BootstrapPayload {
    state.bootstrap(env!("CARGO_PKG_VERSION"))
}

#[tauri::command]
pub fn round_poll(state: State<'_, AppState>) -> RoundPollPayload {
    state.round_poll()
}

#[tauri::command]
pub async fn start_preflight(
    app: AppHandle,
    state: State<'_, AppState>,
    input: PreflightInput,
) -> CommandResult<SessionState> {
    validate_roster(&state, &input)?;
    let objective = input.objective.trim().to_string();
    if objective.chars().count() < 12 {
        return Err("The objective must contain at least 12 characters.".into());
    }
    if objective.chars().count() > 100_000 {
        return Err("The objective exceeds the 100,000-character safety limit.".into());
    }
    tracing::info!(prompt = %sanitize_prompt_for_log(&objective), "starting council pre-flight");
    let temp_dir = state.paths().temp_dir.clone();
    let attachments =
        tokio::task::spawn_blocking(move || ingestion::ingest(input.attachment_paths, &temp_dir))
            .await
            .map_err(|error| error.to_string())??;
    validate_attachment_capabilities(&state, &input.agents, &attachments)?;
    let analysis = engine::orchestrate_preflight(
        &state,
        &objective,
        &input.agents,
        &attachments,
        CancellationToken::new(),
    )
    .await?;
    let phase = if analysis.needs_clarification {
        LifecyclePhase::Clarification
    } else {
        LifecyclePhase::AspectGate
    };
    let (_, session) = state.mutate_session(|session| {
        session.phase = phase;
        session.objective = objective;
        session.main_phrase = analysis.main_phrase;
        session.agents = input.agents;
        session.attachments = attachments;
        session.ambiguity_score = Some(analysis.clarity_score);
        session.clarification_questions = analysis.clarification_questions;
        session.clarification_answers.clear();
        session.aspects = analysis.aspects;
        session.rounds.clear();
        session.compacted_history.clear();
        session.final_synthesis = None;
        Ok(())
    })?;
    let smallest_context = session
        .agents
        .iter()
        .filter_map(|agent| {
            state
                .model(&agent.provider_id, &agent.model_id)
                .map(|model| model.context_window)
        })
        .min()
        .unwrap_or(32_768);
    if let Ok((_, _, Some(warning))) = ingestion::build_context_bundle(&session, smallest_context) {
        state.add_notice(
            &app,
            NoticeSeverity::Warning,
            "Context budget warning",
            warning,
            None,
        );
    }
    state.checkpoint(&session)?;
    state.emit_session(&app, &session);
    Ok(session)
}

#[tauri::command]
pub async fn submit_clarification(
    app: AppHandle,
    state: State<'_, AppState>,
    answers: HashMap<String, String>,
) -> CommandResult<SessionState> {
    let snapshot = state.session();
    if snapshot.phase != LifecyclePhase::Clarification {
        return Err("The session is not waiting for clarifications.".into());
    }
    let mut additions = vec![];
    for question in &snapshot.clarification_questions {
        let answer = answers
            .get(&question.id)
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| "Please answer every clarification question.".to_string())?;
        if answer.chars().count() > 20_000 {
            return Err("A clarification answer exceeds the 20,000-character limit.".into());
        }
        additions.push(format!("Question: {}\nAnswer: {answer}", question.prompt));
    }
    let objective = format!(
        "{}\n\nCLARIFICATIONS PROVIDED BY THE USER:\n{}",
        snapshot.objective,
        additions.join("\n\n")
    );
    let analysis = engine::orchestrate_preflight(
        &state,
        &objective,
        &snapshot.agents,
        &snapshot.attachments,
        CancellationToken::new(),
    )
    .await?;
    let next_phase = if analysis.needs_clarification {
        LifecyclePhase::Clarification
    } else {
        LifecyclePhase::AspectGate
    };
    let (_, session) = state.mutate_session(|session| {
        if session.phase != LifecyclePhase::Clarification {
            return Err(
                "The clarification state changed while the Orchestrator was working.".into(),
            );
        }
        session.objective = objective;
        session.main_phrase = analysis.main_phrase;
        session.clarification_answers.extend(answers);
        session.ambiguity_score = Some(analysis.clarity_score);
        session.clarification_questions = analysis.clarification_questions;
        session.aspects = analysis.aspects;
        session.phase = next_phase;
        Ok(())
    })?;
    state.checkpoint(&session)?;
    state.emit_session(&app, &session);
    Ok(session)
}

#[tauri::command]
pub fn approve_aspects(
    app: AppHandle,
    state: State<'_, AppState>,
    aspects: Vec<Aspect>,
) -> CommandResult<SessionState> {
    validate_aspects(&aspects)?;
    let (_, session) = state.mutate_session(|session| {
        if session.phase != LifecyclePhase::AspectGate {
            return Err("The session is not at the aspect approval gate.".into());
        }
        session.aspects = aspects;
        session.phase = LifecyclePhase::PostRound;
        Ok(())
    })?;
    state.checkpoint(&session)?;
    state.emit_session(&app, &session);
    Ok(session)
}

#[tauri::command]
pub fn reject_aspects(app: AppHandle, state: State<'_, AppState>) -> CommandResult<SessionState> {
    let (_, session) = state.mutate_session(|session| {
        if !session.rounds.is_empty() { return Err("A session with completed rounds cannot return to pre-flight. Start a new session instead.".into()); }
        session.phase = LifecyclePhase::PreSession;
        Ok(())
    })?;
    state.emit_session(&app, &session);
    Ok(session)
}

#[tauri::command]
pub async fn start_round(
    app: AppHandle,
    state: State<'_, AppState>,
    user_argument: Option<String>,
) -> CommandResult<SessionState> {
    if state.session().phase != LifecyclePhase::PostRound {
        return Err("The session is not ready to start a round.".into());
    }
    if let Some(argument) = &user_argument
        && argument.chars().count() > 40_000
    {
        return Err("Injected arguments are limited to 40,000 characters.".into());
    }
    if let Some(handle) = state.take_active_round()
        && !handle.is_finished()
    {
        state.set_active_round(handle);
        return Err("A provider round is already active.".into());
    }
    let cancellation = state.reset_cancellation();
    let (_, session) = state.mutate_session(|session| {
        let index = session.rounds.len() as u32 + 1;
        let responses = session
            .agents
            .iter()
            .filter(|agent| agent.role == AgentRole::Member)
            .map(|agent| AgentResponse {
                agent_id: agent.id.clone(),
                content: String::new(),
                status: AgentStatus::Streaming,
                error: None,
                input_tokens: None,
                output_tokens: None,
                latency_ms: 0,
            })
            .collect();
        for agent in &mut session.agents {
            agent.status = if agent.role == AgentRole::Member {
                AgentStatus::Streaming
            } else {
                AgentStatus::Idle
            };
        }
        session.rounds.push(RoundRecord {
            index,
            started_at: chrono::Utc::now(),
            completed_at: None,
            responses,
            friction: vec![],
            scores: vec![],
            user_argument: user_argument
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
            semantic_similarity: None,
            consensus: None,
        });
        session.phase = LifecyclePhase::RoundRunning;
        Ok(())
    })?;
    state.checkpoint(&session)?;
    state.emit_session(&app, &session);
    let round_index = session.rounds.last().map(|round| round.index).unwrap_or(1);
    let task_state = state.inner().clone();
    let handle = spawn_on_tauri_runtime(async move {
        engine::execute_round(
            engine::StateRoundEvents,
            task_state,
            round_index,
            cancellation,
        )
        .await;
    });
    state.set_active_round(handle);
    Ok(session)
}

#[tauri::command]
pub async fn stop_round(app: AppHandle, state: State<'_, AppState>) -> CommandResult<SessionState> {
    state.cancel();
    if let Some(mut handle) = state.take_active_round()
        && tokio::time::timeout(Duration::from_secs(5), &mut handle)
            .await
            .is_err()
    {
        handle.abort();
        let _ = handle.await;
    }
    let (_, session) = state.mutate_session(|session| {
        for agent in &mut session.agents {
            if agent.status == AgentStatus::Streaming {
                agent.status = AgentStatus::Cancelled;
            }
        }
        for round in &mut session.rounds {
            for response in &mut round.responses {
                if response.status == AgentStatus::Streaming {
                    response.status = AgentStatus::Cancelled;
                    response.error = Some("Cancelled by user".into());
                }
            }
        }
        session.phase = if session.rounds.is_empty() {
            LifecyclePhase::PreSession
        } else {
            LifecyclePhase::PostRound
        };
        Ok(())
    })?;
    state.checkpoint(&session)?;
    state.emit_session(&app, &session);
    state.add_notice(&app, NoticeSeverity::Warning, "Round stopped", "Partial responses were retained and excluded from scoring. All provider tasks were cancelled.", None);
    Ok(session)
}

#[tauri::command]
pub async fn retry_agent(
    app: AppHandle,
    state: State<'_, AppState>,
    agent_id: String,
) -> CommandResult<SessionState> {
    if state.session().phase != LifecyclePhase::PostRound {
        return Err("Failed agents can only be retried from the post-round command center.".into());
    }
    let cancellation = state.reset_cancellation();
    let (_, session) = state.mutate_session(|session| {
        let round = session
            .rounds
            .last_mut()
            .ok_or_else(|| "No round is available to retry.".to_string())?;
        let response = round
            .responses
            .iter_mut()
            .find(|response| response.agent_id == agent_id)
            .ok_or_else(|| "Agent response not found.".to_string())?;
        if !matches!(
            response.status,
            AgentStatus::Failed | AgentStatus::Cancelled
        ) {
            return Err("Only failed or cancelled responses can be retried.".into());
        }
        response.content.clear();
        response.error = None;
        response.status = AgentStatus::Streaming;
        response.input_tokens = None;
        response.output_tokens = None;
        response.latency_ms = 0;
        round.friction.clear();
        round.scores.clear();
        round.completed_at = None;
        if let Some(agent) = session.agents.iter_mut().find(|agent| agent.id == agent_id) {
            agent.status = AgentStatus::Streaming;
        }
        session.phase = LifecyclePhase::RoundRunning;
        Ok(())
    })?;
    state.checkpoint(&session)?;
    state.emit_session(&app, &session);
    let round_index = session.rounds.last().map(|round| round.index).unwrap_or(1);
    let task_state = state.inner().clone();
    let task_agent = agent_id.clone();
    let handle = spawn_on_tauri_runtime(async move {
        engine::retry_agent(
            engine::StateRoundEvents,
            task_state,
            round_index,
            task_agent,
            cancellation,
        )
        .await;
    });
    state.set_active_round(handle);
    Ok(session)
}

#[tauri::command]
pub async fn finalize_session(
    app: AppHandle,
    state: State<'_, AppState>,
) -> CommandResult<SessionState> {
    if state.session().phase != LifecyclePhase::PostRound {
        return Err("The council can only be finalized after a round completes.".into());
    }
    let synthesis = engine::synthesize(&state, CancellationToken::new()).await;
    let (_, session) = state.mutate_session(|session| {
        session.final_synthesis = Some(synthesis);
        session.phase = LifecyclePhase::Finalized;
        Ok(())
    })?;
    state.checkpoint(&session)?;
    state.emit_session(&app, &session);
    Ok(session)
}

#[tauri::command]
pub fn new_session(
    state: State<'_, AppState>,
    agents: Option<Vec<AgentAssignment>>,
) -> CommandResult<BootstrapPayload> {
    state.cancel();
    checkpoint::discard(&state.paths().checkpoint)?;
    clear_directory(&state.paths().temp_dir)?;
    let mut preserved_agents = agents.unwrap_or_else(|| state.session().agents);
    for agent in &mut preserved_agents {
        agent.status = AgentStatus::Idle;
    }
    let mut payload = state.reset();
    if !preserved_agents.is_empty() {
        payload.session.agents = preserved_agents;
        state.replace_session(payload.session.clone());
    }
    Ok(payload)
}

#[tauri::command]
pub async fn save_credential(
    state: State<'_, AppState>,
    provider_id: String,
    secret: String,
) -> CommandResult<String> {
    let provider = state
        .provider(&provider_id)
        .ok_or_else(|| "Unknown provider.".to_string())?;
    let ledger = state.credentials().clone();
    let id = provider_id.clone();
    tokio::task::spawn_blocking(move || ledger.set(&id, secret))
        .await
        .map_err(|error| error.to_string())??;
    state.update_provider_status(&provider_id, CredentialStatus::Configured);
    if !provider.supports_discovery {
        return Ok("Saved to the OS credential manager. This provider does not expose automatic model discovery.".into());
    }
    let key = match load_credential(&state, &provider_id).await {
        Ok(key) => key,
        Err(_) => {
            return Ok("Saved securely, but the credential could not be reopened for model discovery. Connection is not verified yet.".into());
        }
    };
    match refresh_provider_models(&state, &provider, &key).await {
        Ok(count) => Ok(format!(
            "Saved securely and imported {count} current generation models. Connection is not verified yet."
        )),
        Err(error) => Ok(format!(
            "Saved securely, but the model catalog could not be refreshed: {error}"
        )),
    }
}

#[tauri::command]
pub async fn delete_credential(
    state: State<'_, AppState>,
    provider_id: String,
) -> CommandResult<()> {
    let ledger = state.credentials().clone();
    let id = provider_id.clone();
    tokio::task::spawn_blocking(move || ledger.delete(&id))
        .await
        .map_err(|error| error.to_string())??;
    state.update_provider_status(&provider_id, CredentialStatus::Untested);
    Ok(())
}

#[tauri::command]
pub async fn test_connection(
    state: State<'_, AppState>,
    provider_id: String,
) -> CommandResult<String> {
    let provider = state
        .provider(&provider_id)
        .ok_or_else(|| "Unknown provider.".to_string())?;
    if provider.protocol == WireProtocol::Demo {
        return Ok("Local demo provider is ready.".into());
    }
    let key = load_credential(&state, &provider_id)
        .await
        .map_err(|_| "No stored credential is available to test.".to_string())?;
    let discovery = if provider.supports_discovery {
        refresh_provider_models(&state, &provider, &key).await.ok()
    } else {
        None
    };
    let model = validation_model(&state, &provider_id).ok_or_else(|| {
        "No compatible validation model is available for this provider.".to_string()
    })?;
    let client = build_provider(provider.clone(), key).await?;
    let request = CompletionRequest {
        system: "Connection check. Reply only with OK.".into(),
        prompt: "OK".into(),
        model: model.id,
        // Thinking models can spend a small probe's entire output budget before
        // producing final text. Disable thinking where the provider supports the
        // OpenAI-compatible switch and leave enough room for a normal reply.
        max_tokens: 256,
        temperature: 0.0,
        thinking_enabled: Some(false),
        images: vec![],
    };
    match client.complete(request, CancellationToken::new()).await {
        Ok(_) => {
            state.update_provider_status(&provider_id, CredentialStatus::Valid);
            Ok(match discovery {
                Some(count) => format!(
                    "Connection verified and {count} current generation models were retrieved."
                ),
                None if provider.supports_discovery => "Connection verified. The provider accepted a minimal completion request, but its model catalog could not be refreshed.".into(),
                None => "Connection verified. The provider accepted a minimal completion request.".into(),
            })
        }
        Err(error) => {
            state.update_provider_status(&provider_id, CredentialStatus::Invalid);
            Err(error.user_message())
        }
    }
}

#[tauri::command]
pub async fn refresh_models(
    state: State<'_, AppState>,
    provider_id: String,
) -> CommandResult<String> {
    let provider = state
        .provider(&provider_id)
        .ok_or_else(|| "Unknown provider.".to_string())?;
    if !provider.supports_discovery {
        return Err("This provider does not expose automatic model discovery.".into());
    }
    let key = load_credential(&state, &provider_id)
        .await
        .map_err(|_| "Save a credential before refreshing this provider's models.".to_string())?;
    let count = refresh_provider_models(&state, &provider, &key).await?;
    Ok(format!(
        "Retrieved {count} current generation models from {}.",
        provider.name
    ))
}

async fn refresh_provider_models(
    state: &AppState,
    provider: &ProviderSummary,
    key: &str,
) -> CommandResult<usize> {
    let models = discover_models(provider, key)
        .await
        .map_err(|error| error.user_message())?;
    state.replace_provider_models(&provider.id, models)
}

fn validation_model(state: &AppState, provider_id: &str) -> Option<ModelInfo> {
    let available = state
        .models()
        .into_iter()
        .filter(|model| model.provider_id == provider_id)
        .collect::<Vec<_>>();
    let preferred = catalog::models()
        .into_iter()
        .filter(|model| model.provider_id == provider_id)
        .map(|model| model.id)
        .collect::<Vec<_>>();
    preferred
        .iter()
        .find_map(|id| available.iter().find(|model| &model.id == id).cloned())
        .or_else(|| available.into_iter().next())
}

async fn load_credential(state: &AppState, provider_id: &str) -> CommandResult<String> {
    let ledger = state.credentials().clone();
    let id = provider_id.to_string();
    tracing::info!(provider_id = %id, "reading provider credential on blocking thread");
    tokio::task::spawn_blocking(move || ledger.get(&id).map(|value| value.as_str().to_owned()))
        .await
        .map_err(|error| format!("Credential task failed: {error}"))?
}

async fn build_provider(
    provider: ProviderSummary,
    key: String,
) -> CommandResult<Box<dyn crate::providers::ModelProvider>> {
    tokio::task::spawn_blocking(move || {
        make_provider(&provider, Some(key)).map_err(|error| error.user_message())
    })
    .await
    .map_err(|error| format!("Provider construction task failed: {error}"))?
}

#[tauri::command]
pub fn update_provider(
    state: State<'_, AppState>,
    provider_id: String,
    base_url: String,
    timeout: TimeoutPolicy,
) -> CommandResult<BootstrapPayload> {
    validate_endpoint(&base_url)?;
    validate_timeout(&timeout)?;
    state.update_provider(&provider_id, base_url, timeout)?;
    Ok(state.bootstrap(env!("CARGO_PKG_VERSION")))
}

#[tauri::command]
pub fn save_persona(
    state: State<'_, AppState>,
    mut persona: Persona,
) -> CommandResult<Vec<Persona>> {
    if persona.builtin {
        return Err(
            "Built-in archetypes cannot be overwritten; save a custom copy instead.".into(),
        );
    }
    persona.name = persona.name.trim().to_string();
    persona.description = persona.description.trim().to_string();
    persona.system_prompt = persona.system_prompt.trim().to_string();
    persona.directives = persona
        .directives
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect();
    if persona.name.is_empty() || persona.name.chars().count() > 80 {
        return Err("Persona names must contain 1–80 characters.".into());
    }
    if persona.system_prompt.is_empty() || persona.system_prompt.chars().count() > 20_000 {
        return Err("Persona system prompts must contain 1–20,000 characters.".into());
    }
    let mut personas = state.personas();
    personas.retain(|item| item.id != persona.id || item.builtin);
    personas.push(persona);
    state.save_personas(personas)
}

#[tauri::command]
pub fn delete_persona(
    state: State<'_, AppState>,
    persona_id: String,
) -> CommandResult<Vec<Persona>> {
    if state
        .personas()
        .iter()
        .any(|persona| persona.id == persona_id && persona.builtin)
    {
        return Err("Built-in archetypes cannot be deleted.".into());
    }
    if state
        .session()
        .agents
        .iter()
        .any(|agent| agent.persona_id.as_deref() == Some(&persona_id))
    {
        return Err("This persona is assigned to a current council seat. Reassign that seat before deleting it.".into());
    }
    let personas: Vec<Persona> = state
        .personas()
        .into_iter()
        .filter(|persona| persona.id != persona_id)
        .collect();
    state.save_personas(personas)
}

#[tauri::command]
pub async fn ingest_files(
    app: AppHandle,
    state: State<'_, AppState>,
    paths: Vec<String>,
) -> CommandResult<SessionState> {
    let temp_dir = state.paths().temp_dir.clone();
    let attachments = tokio::task::spawn_blocking(move || ingestion::ingest(paths, &temp_dir))
        .await
        .map_err(|error| error.to_string())??;
    let agents = state.session().agents;
    validate_attachment_capabilities(&state, &agents, &attachments)?;
    let (_, session) = state.mutate_session(|session| {
        session.attachments.extend(attachments);
        Ok(())
    })?;
    state.checkpoint(&session)?;
    state.emit_session(&app, &session);
    Ok(session)
}

#[tauri::command]
pub fn import_session(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> CommandResult<SessionState> {
    if !state.session().rounds.is_empty() {
        return Err("Import is only available before the first round of a new session.".into());
    }
    let session = report::import_markdown(&PathBuf::from(path))?;
    validate_imported_session(&state, &session)?;
    state.replace_session(session.clone());
    state.replace_telemetry(TelemetrySnapshot::empty(session.id.clone()));
    for round in &session.rounds {
        for response in &round.responses {
            if let Some(agent) = session
                .agents
                .iter()
                .find(|agent| agent.id == response.agent_id)
            {
                state.record_usage(
                    &agent.provider_id,
                    &agent.model_id,
                    response.input_tokens,
                    response.output_tokens,
                );
            }
        }
    }
    state.checkpoint(&session)?;
    state.emit_session(&app, &session);
    state.emit_telemetry(&app);
    Ok(session)
}

#[tauri::command]
pub async fn export_markdown(
    state: State<'_, AppState>,
    path: String,
) -> CommandResult<ExportResult> {
    let session = state.session();
    tokio::task::spawn_blocking(move || report::export_markdown(&session, &PathBuf::from(path)))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn export_pdf(state: State<'_, AppState>, path: String) -> CommandResult<ExportResult> {
    let session = state.session();
    tokio::task::spawn_blocking(move || report::export_pdf(&session, &PathBuf::from(path)))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
pub fn restore_checkpoint(
    app: AppHandle,
    state: State<'_, AppState>,
) -> CommandResult<BootstrapPayload> {
    let session = checkpoint::load(&state.paths().checkpoint)?;
    state.replace_session(session.clone());
    state.replace_telemetry(TelemetrySnapshot::empty(session.id.clone()));
    state.emit_session(&app, &session);
    Ok(state.bootstrap(env!("CARGO_PKG_VERSION")))
}

#[tauri::command]
pub fn discard_checkpoint(state: State<'_, AppState>) -> CommandResult<BootstrapPayload> {
    checkpoint::discard(&state.paths().checkpoint)?;
    Ok(state.reset())
}

#[tauri::command]
pub async fn hard_clear(
    app: AppHandle,
    state: State<'_, AppState>,
) -> CommandResult<BootstrapPayload> {
    state.cancel();
    if let Some(mut handle) = state.take_active_round()
        && tokio::time::timeout(Duration::from_secs(5), &mut handle)
            .await
            .is_err()
    {
        handle.abort();
        let _ = handle.await;
    }
    let ledger = state.credentials().clone();
    let provider_ids: Vec<String> = state
        .providers()
        .into_iter()
        .filter(|provider| provider.id != "demo")
        .map(|provider| provider.id)
        .collect();
    tokio::task::spawn_blocking(move || {
        for id in provider_ids {
            ledger.delete(&id)?;
        }
        Ok::<(), String>(())
    })
    .await
    .map_err(|error| error.to_string())??;
    clear_app_owned_data(state.paths())?;
    for webview in app.webview_windows().into_values() {
        webview.clear_all_browsing_data().map_err(|error| {
            format!("Could not clear the embedded browser's local data: {error}")
        })?;
    }
    state.reset_catalogs();
    Ok(state.reset())
}

fn validate_roster(state: &AppState, input: &PreflightInput) -> CommandResult<()> {
    let orchestrators = input
        .agents
        .iter()
        .filter(|agent| agent.role == AgentRole::Orchestrator)
        .count();
    let members = input
        .agents
        .iter()
        .filter(|agent| agent.role == AgentRole::Member)
        .count();
    if orchestrators != 1 || members < 2 {
        return Err(
            "A valid council requires exactly one Orchestrator and at least two council members."
                .into(),
        );
    }
    if members > 8 {
        return Err(
            "The desktop client supports at most eight council members per session.".into(),
        );
    }
    let mut ids = HashSet::new();
    for agent in &input.agents {
        if !ids.insert(&agent.id) {
            return Err("Every council seat must have a unique ID.".into());
        }
        if state.model(&agent.provider_id, &agent.model_id).is_none() {
            return Err(format!(
                "{} references an unknown provider/model pair.",
                agent.display_name
            ));
        }
        if let Some(persona) = &agent.persona_id
            && !state.personas().iter().any(|item| &item.id == persona)
        {
            return Err(format!(
                "{} references an unknown persona.",
                agent.display_name
            ));
        }
    }
    Ok(())
}

fn validate_attachment_capabilities(
    state: &AppState,
    agents: &[AgentAssignment],
    attachments: &[Attachment],
) -> CommandResult<()> {
    if !attachments
        .iter()
        .any(|attachment| attachment.media_type.starts_with("image/"))
    {
        return Ok(());
    }
    for agent in agents {
        let model = state
            .model(&agent.provider_id, &agent.model_id)
            .ok_or_else(|| "Unknown assigned model.".to_string())?;
        if !model.supports_vision {
            return Err(format!(
                "{} ({}) cannot accept image attachments. Remove the image or assign a vision-capable model.",
                agent.display_name, model.name
            ));
        }
    }
    Ok(())
}

fn validate_aspects(aspects: &[Aspect]) -> CommandResult<()> {
    if !(3..=5).contains(&aspects.len()) {
        return Err("Use between three and five evaluation aspects.".into());
    }
    let mut names = HashSet::new();
    for aspect in aspects {
        let normalized = aspect.name.trim().to_ascii_lowercase();
        if normalized.is_empty() || aspect.name.chars().count() > 100 {
            return Err("Aspect names must contain 1–100 characters.".into());
        }
        if !names.insert(normalized) {
            return Err("Aspect names must be unique.".into());
        }
        if !aspect.weight.is_finite() || !(0.25..=3.0).contains(&aspect.weight) {
            return Err("Aspect weights must be between 0.25 and 3.0.".into());
        }
    }
    Ok(())
}

fn validate_timeout(timeout: &TimeoutPolicy) -> CommandResult<()> {
    if !(1..=120).contains(&timeout.connect_secs)
        || !(1..=600).contains(&timeout.first_token_secs)
        || !(1..=300).contains(&timeout.idle_stream_secs)
        || !(10..=3_600).contains(&timeout.total_secs)
        || !(1..=5).contains(&timeout.max_attempts)
    {
        return Err("Timeout values are outside the supported safety bounds.".into());
    }
    Ok(())
}

fn validate_imported_session(state: &AppState, session: &SessionState) -> CommandResult<()> {
    if session.schema_version != SESSION_SCHEMA_VERSION {
        return Err("Unsupported session schema.".into());
    }
    let input = PreflightInput {
        objective: session.objective.clone(),
        agents: session.agents.clone(),
        attachment_paths: vec![],
    };
    validate_roster(state, &input)?;
    validate_aspects(&session.aspects)?;
    for round in &session.rounds {
        for cell in &round.scores {
            if !cell.median.is_finite() || !(0.0..=10.0).contains(&cell.median) {
                return Err("Imported answer medians must use the 0–10 scale.".into());
            }
            if cell
                .votes
                .iter()
                .any(|vote| !vote.score.is_finite() || !(0.0..=10.0).contains(&vote.score))
            {
                return Err("Imported peer votes must use the 0–10 scale.".into());
            }
        }
    }
    Ok(())
}

fn clear_directory(path: &PathBuf) -> CommandResult<()> {
    if path.exists() {
        fs::remove_dir_all(path)
            .map_err(|error| format!("Could not clear {}: {error}", path.display()))?;
    }
    fs::create_dir_all(path)
        .map_err(|error| format!("Could not recreate {}: {error}", path.display()))
}

/// Clear only the files Agentic Council owns. On Windows, Tauri resolves the
/// app-local-data and app-cache roots to the same directory, which also hosts
/// the live WebView2 profile. Removing that root while the app is running
/// fails with ERROR_SHARING_VIOLATION and risks deleting browser runtime data.
fn clear_app_owned_data(paths: &AppPaths) -> CommandResult<()> {
    for path in [
        &paths.checkpoint,
        &paths.providers,
        &paths.models,
        &paths.personas,
    ] {
        remove_file_if_present(path)?;
    }
    clear_directory(&paths.temp_dir)?;
    clear_log_directory(&paths.logs_dir)
}

fn remove_file_if_present(path: &PathBuf) -> CommandResult<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("Could not clear {}: {error}", path.display())),
    }
}

fn clear_log_directory(path: &PathBuf) -> CommandResult<()> {
    fs::create_dir_all(path)
        .map_err(|error| format!("Could not access {}: {error}", path.display()))?;
    for entry in fs::read_dir(path)
        .map_err(|error| format!("Could not inspect {}: {error}", path.display()))?
    {
        let entry = entry.map_err(|error| format!("Could not inspect log entry: {error}"))?;
        let entry_path = entry.path();
        if entry_path.is_dir() {
            fs::remove_dir_all(&entry_path)
                .map_err(|error| format!("Could not clear {}: {error}", entry_path.display()))?;
            continue;
        }
        if let Err(remove_error) = fs::remove_file(&entry_path) {
            // The active tracing file is held open on Windows. Its handle
            // permits writes but not deletion, so truncate it in place.
            fs::OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(&entry_path)
                .map_err(|truncate_error| {
                    format!(
                        "Could not clear {}: {remove_error}; truncation also failed: {truncate_error}",
                        entry_path.display()
                    )
                })?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{clear_app_owned_data, spawn_on_tauri_runtime};
    use crate::state::AppPaths;
    use std::fs;

    #[test]
    fn background_work_can_spawn_without_an_entered_tokio_runtime() {
        assert!(tokio::runtime::Handle::try_current().is_err());

        let task = spawn_on_tauri_runtime(async { 42_u8 });
        let output = tauri::async_runtime::block_on(task)
            .expect("task spawned through Tauri's runtime should complete");

        assert_eq!(output, 42);
    }

    #[test]
    fn hard_clear_preserves_the_live_runtime_root() {
        let root = std::env::temp_dir().join(format!(
            "agentic-council-clear-test-{}",
            uuid::Uuid::new_v4()
        ));
        let paths = AppPaths::new(root.clone(), root.clone()).unwrap();
        fs::write(&paths.checkpoint, b"session").unwrap();
        fs::write(&paths.providers, b"providers").unwrap();
        fs::write(paths.temp_dir.join("extract.txt"), b"content").unwrap();
        fs::write(paths.logs_dir.join("old.log"), b"diagnostics").unwrap();
        let webview_dir = root.join("EBWebView");
        fs::create_dir_all(&webview_dir).unwrap();
        fs::write(webview_dir.join("lockfile"), b"runtime").unwrap();

        clear_app_owned_data(&paths).unwrap();

        assert!(root.exists());
        assert!(webview_dir.join("lockfile").exists());
        assert!(!paths.checkpoint.exists());
        assert!(!paths.providers.exists());
        assert!(!paths.temp_dir.join("extract.txt").exists());
        assert!(!paths.logs_dir.join("old.log").exists());
        let _ = fs::remove_dir_all(root);
    }
}
