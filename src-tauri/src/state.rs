use crate::{catalog, checkpoint, models::*, security::CredentialLedger};
use chrono::Utc;
use parking_lot::{Mutex, RwLock};
use std::{fs, path::PathBuf, sync::Arc};
use tauri::{AppHandle, Emitter, Runtime};
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub data_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub temp_dir: PathBuf,
    pub checkpoint: PathBuf,
    pub providers: PathBuf,
    pub models: PathBuf,
    pub personas: PathBuf,
    pub logs_dir: PathBuf,
}

impl AppPaths {
    pub fn new(data_dir: PathBuf, cache_dir: PathBuf) -> Result<Self, String> {
        let temp_dir = cache_dir.join("ingestion");
        let logs_dir = data_dir.join("logs");
        for path in [&data_dir, &cache_dir, &temp_dir, &logs_dir] {
            fs::create_dir_all(path)
                .map_err(|error| format!("Could not create {}: {error}", path.display()))?;
        }
        Ok(Self {
            checkpoint: data_dir.join("session.checkpoint.json"),
            providers: data_dir.join("providers.json"),
            models: data_dir.join("models.json"),
            personas: data_dir.join("personas.json"),
            data_dir,
            cache_dir,
            temp_dir,
            logs_dir,
        })
    }
}

#[derive(Clone)]
pub struct AppState {
    inner: Arc<Inner>,
}

struct Inner {
    pub session: RwLock<SessionState>,
    pub telemetry: RwLock<TelemetrySnapshot>,
    pub providers: RwLock<Vec<ProviderSummary>>,
    pub models: RwLock<Vec<ModelInfo>>,
    pub personas: RwLock<Vec<Persona>>,
    pub notices: RwLock<Vec<AppNotice>>,
    pub cancellation: Mutex<CancellationToken>,
    pub active_round: Mutex<Option<tokio::task::JoinHandle<()>>>,
    pub credentials: CredentialLedger,
    pub paths: AppPaths,
}

impl AppState {
    pub fn new(paths: AppPaths) -> Self {
        let session = SessionState::empty();
        let ledger = CredentialLedger;
        let mut providers =
            load_json::<Vec<ProviderSummary>>(&paths.providers).unwrap_or_else(catalog::providers);
        let defaults = catalog::providers();
        providers.retain(|provider| defaults.iter().any(|item| item.id == provider.id));
        for default in &defaults {
            if !providers.iter().any(|provider| provider.id == default.id) {
                providers.push(default.clone());
            }
        }
        for provider in &mut providers {
            if let Some(default) = defaults.iter().find(|item| item.id == provider.id) {
                provider.name = default.name.clone();
                provider.protocol = default.protocol.clone();
                provider.supports_discovery = default.supports_discovery;
                provider.configurable_endpoint = default.configurable_endpoint;
                provider.timeout = default.timeout.clone();
                if !default.configurable_endpoint {
                    provider.base_url = default.base_url.clone();
                }
            }
            let persisted_status = provider.credential_status.clone();
            provider.credential_status = if provider.id == "demo" {
                CredentialStatus::NotRequired
            } else if ledger.exists(&provider.id) {
                match persisted_status {
                    status @ (CredentialStatus::Valid
                    | CredentialStatus::Invalid
                    | CredentialStatus::Configured) => status,
                    CredentialStatus::Untested | CredentialStatus::NotRequired => {
                        CredentialStatus::Configured
                    }
                }
            } else {
                CredentialStatus::Untested
            };
        }
        let mut models = load_json::<Vec<ModelInfo>>(&paths.models).unwrap_or_else(catalog::models);
        models.retain(|model| {
            providers
                .iter()
                .any(|provider| provider.id == model.provider_id)
        });
        for default in catalog::models() {
            let provider_discovers_models = providers
                .iter()
                .find(|provider| provider.id == default.provider_id)
                .is_some_and(|provider| provider.supports_discovery);
            if !models
                .iter()
                .any(|model| model.provider_id == default.provider_id && model.id == default.id)
                && (!provider_discovers_models
                    || !models
                        .iter()
                        .any(|model| model.provider_id == default.provider_id))
            {
                models.push(default);
            }
        }
        let mut personas = catalog::personas();
        if let Some(custom) = load_json::<Vec<Persona>>(&paths.personas) {
            personas.extend(custom.into_iter().filter(|persona| !persona.builtin));
        }
        Self {
            inner: Arc::new(Inner {
                telemetry: RwLock::new(TelemetrySnapshot::empty(session.id.clone())),
                session: RwLock::new(session),
                providers: RwLock::new(providers),
                models: RwLock::new(models),
                personas: RwLock::new(personas),
                notices: RwLock::new(vec![]),
                cancellation: Mutex::new(CancellationToken::new()),
                active_round: Mutex::new(None),
                credentials: ledger,
                paths,
            }),
        }
    }

    pub fn paths(&self) -> &AppPaths {
        &self.inner.paths
    }
    pub fn credentials(&self) -> &CredentialLedger {
        &self.inner.credentials
    }
    pub fn session(&self) -> SessionState {
        self.inner.session.read().clone()
    }
    pub fn telemetry(&self) -> TelemetrySnapshot {
        self.inner.telemetry.read().clone()
    }
    pub fn providers(&self) -> Vec<ProviderSummary> {
        self.inner.providers.read().clone()
    }
    pub fn models(&self) -> Vec<ModelInfo> {
        self.inner.models.read().clone()
    }
    pub fn personas(&self) -> Vec<Persona> {
        self.inner.personas.read().clone()
    }
    pub fn notices(&self) -> Vec<AppNotice> {
        self.inner.notices.read().clone()
    }

    pub fn bootstrap(&self, app_version: &str) -> BootstrapPayload {
        BootstrapPayload {
            session: self.session(),
            telemetry: self.telemetry(),
            providers: self.providers(),
            models: self.models(),
            personas: self.personas(),
            notices: self.notices(),
            recoverable_checkpoint: checkpoint::is_recoverable(&self.inner.paths.checkpoint),
            app_version: app_version.to_string(),
        }
    }

    pub fn round_poll(&self) -> RoundPollPayload {
        RoundPollPayload {
            session: self.session(),
            telemetry: self.telemetry(),
            notices: self.notices(),
        }
    }

    pub fn replace_session(&self, session: SessionState) {
        *self.inner.session.write() = session;
    }

    pub fn mutate_session<T>(
        &self,
        operation: impl FnOnce(&mut SessionState) -> Result<T, String>,
    ) -> Result<(T, SessionState), String> {
        let mut session = self.inner.session.write();
        let result = operation(&mut session)?;
        session.updated_at = Utc::now();
        Ok((result, session.clone()))
    }

    pub fn mutate_session_in_place<T>(
        &self,
        operation: impl FnOnce(&mut SessionState) -> Result<T, String>,
    ) -> Result<T, String> {
        let mut session = self.inner.session.write();
        let result = operation(&mut session)?;
        session.updated_at = Utc::now();
        Ok(result)
    }

    pub fn emit_session<R: Runtime>(&self, app: &AppHandle<R>, session: &SessionState) {
        emit_on_main(app, "session://snapshot", session.clone());
    }

    pub fn checkpoint(&self, session: &SessionState) -> Result<(), String> {
        checkpoint::write_atomic(&self.inner.paths.checkpoint, session)
    }

    pub fn emit_telemetry<R: Runtime>(&self, app: &AppHandle<R>) {
        let telemetry = self.telemetry();
        emit_on_main(app, "telemetry://updated", telemetry);
    }

    pub fn add_notice<R: Runtime>(
        &self,
        app: &AppHandle<R>,
        severity: NoticeSeverity,
        title: impl Into<String>,
        message: impl Into<String>,
        details: Option<String>,
    ) {
        let notice = self.store_notice(severity, title, message, details);
        emit_on_main(app, "app://notice", notice);
    }

    pub fn store_notice(
        &self,
        severity: NoticeSeverity,
        title: impl Into<String>,
        message: impl Into<String>,
        details: Option<String>,
    ) -> AppNotice {
        let notice = AppNotice {
            id: uuid::Uuid::new_v4().to_string(),
            severity,
            title: title.into(),
            message: message.into(),
            details,
            timestamp: Utc::now(),
        };
        let mut notices = self.inner.notices.write();
        notices.insert(0, notice.clone());
        notices.truncate(100);
        notice
    }

    pub fn reset_cancellation(&self) -> CancellationToken {
        let token = CancellationToken::new();
        *self.inner.cancellation.lock() = token.clone();
        token
    }
    pub fn cancel(&self) {
        self.inner.cancellation.lock().cancel();
    }
    pub fn set_active_round(&self, handle: tokio::task::JoinHandle<()>) {
        if let Some(previous) = self.inner.active_round.lock().replace(handle)
            && !previous.is_finished()
        {
            previous.abort();
        }
    }
    pub fn take_active_round(&self) -> Option<tokio::task::JoinHandle<()>> {
        self.inner.active_round.lock().take()
    }

    pub fn reset(&self) -> BootstrapPayload {
        self.cancel();
        let session = SessionState::empty();
        *self.inner.telemetry.write() = TelemetrySnapshot::empty(session.id.clone());
        *self.inner.session.write() = session;
        self.inner.notices.write().clear();
        self.bootstrap(env!("CARGO_PKG_VERSION"))
    }

    pub fn update_provider_status(&self, id: &str, status: CredentialStatus) {
        let mut providers = self.inner.providers.write();
        if let Some(provider) = providers.iter_mut().find(|provider| provider.id == id) {
            provider.credential_status = status;
            if let Err(error) = write_json(&self.inner.paths.providers, &*providers) {
                tracing::warn!(%error, provider_id = id, "could not persist provider status");
            }
        }
    }

    pub fn replace_provider_models(
        &self,
        provider_id: &str,
        mut discovered: Vec<ModelInfo>,
    ) -> Result<usize, String> {
        if discovered.is_empty() {
            return Err("The provider returned no compatible generation models.".into());
        }
        discovered.retain(|model| model.provider_id == provider_id && !model.id.trim().is_empty());
        discovered.sort_by_cached_key(|model| model.name.to_ascii_lowercase());
        discovered.dedup_by(|left, right| left.id == right.id);
        if discovered.is_empty() {
            return Err("The provider returned no valid model identifiers.".into());
        }
        let count = discovered.len();
        let mut models = self.inner.models.write();
        models.retain(|model| model.provider_id != provider_id);
        models.extend(discovered);
        write_json(&self.inner.paths.models, &*models)?;
        Ok(count)
    }

    pub fn update_provider(
        &self,
        id: &str,
        base_url: String,
        timeout: TimeoutPolicy,
    ) -> Result<(), String> {
        {
            let mut providers = self.inner.providers.write();
            let provider = providers
                .iter_mut()
                .find(|provider| provider.id == id)
                .ok_or_else(|| "Unknown provider.".to_string())?;
            if !provider.configurable_endpoint {
                return Err("This provider endpoint is fixed by the built-in catalog.".into());
            }
            provider.base_url = base_url;
            provider.timeout = timeout;
            write_json(&self.inner.paths.providers, &*providers)?;
        }
        Ok(())
    }

    pub fn provider(&self, id: &str) -> Option<ProviderSummary> {
        self.inner
            .providers
            .read()
            .iter()
            .find(|provider| provider.id == id)
            .cloned()
    }
    pub fn model(&self, provider_id: &str, model_id: &str) -> Option<ModelInfo> {
        self.inner
            .models
            .read()
            .iter()
            .find(|model| model.provider_id == provider_id && model.id == model_id)
            .cloned()
    }

    pub fn record_usage(
        &self,
        provider_id: &str,
        model_id: &str,
        input: Option<u64>,
        output: Option<u64>,
    ) {
        let model = self.model(provider_id, model_id);
        let mut telemetry = self.inner.telemetry.write();
        let usage = telemetry
            .by_model
            .iter_mut()
            .find(|item| item.provider_id == provider_id && item.model_id == model_id);
        let input_cost = input
            .zip(model.as_ref().and_then(|value| value.input_per_million))
            .map(|(tokens, rate)| tokens as f64 / 1_000_000.0 * rate);
        let output_cost = output
            .zip(model.as_ref().and_then(|value| value.output_per_million))
            .map(|(tokens, rate)| tokens as f64 / 1_000_000.0 * rate);
        if let Some(item) = usage {
            item.input_tokens = sum_optional(item.input_tokens, input);
            item.output_tokens = sum_optional(item.output_tokens, output);
            item.input_cost_usd = sum_optional_f64(item.input_cost_usd, input_cost);
            item.output_cost_usd = sum_optional_f64(item.output_cost_usd, output_cost);
            item.total_cost_usd = item
                .input_cost_usd
                .zip(item.output_cost_usd)
                .map(|(a, b)| a + b);
        } else {
            telemetry.by_model.push(ModelUsage {
                provider_id: provider_id.into(),
                model_id: model_id.into(),
                input_tokens: input,
                output_tokens: output,
                input_cost_usd: input_cost,
                output_cost_usd: output_cost,
                total_cost_usd: input_cost.zip(output_cost).map(|(a, b)| a + b),
            });
        }
        telemetry.total_input_tokens = telemetry.by_model.iter().try_fold(0_u64, |sum, item| {
            item.input_tokens.map(|value| sum + value)
        });
        telemetry.total_output_tokens = telemetry.by_model.iter().try_fold(0_u64, |sum, item| {
            item.output_tokens.map(|value| sum + value)
        });
        telemetry.total_cost_usd = telemetry.by_model.iter().try_fold(0_f64, |sum, item| {
            item.total_cost_usd.map(|value| sum + value)
        });
    }

    pub fn save_personas(&self, personas: Vec<Persona>) -> Result<Vec<Persona>, String> {
        let builtins = catalog::personas();
        let custom: Vec<Persona> = personas
            .into_iter()
            .filter(|persona| !persona.builtin)
            .collect();
        write_json(&self.inner.paths.personas, &custom)?;
        let mut merged = builtins;
        merged.extend(custom);
        *self.inner.personas.write() = merged.clone();
        Ok(merged)
    }

    pub fn replace_telemetry(&self, telemetry: TelemetrySnapshot) {
        *self.inner.telemetry.write() = telemetry;
    }

    pub fn reset_catalogs(&self) {
        *self.inner.providers.write() = catalog::providers();
        *self.inner.models.write() = catalog::models();
        *self.inner.personas.write() = catalog::personas();
    }
}

pub fn emit_on_main<R, T>(app: &AppHandle<R>, event: &'static str, payload: T)
where
    R: Runtime,
    T: serde::Serialize + Clone + Send + 'static,
{
    let emitter = app.clone();
    if let Err(error) = app.run_on_main_thread(move || {
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            emitter.emit(event, payload)
        })) {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                tracing::warn!(%error, %event, "could not emit application event");
            }
            Err(_) => {
                tracing::error!(%event, "panic contained while emitting application event");
            }
        }
    }) {
        tracing::warn!(%error, %event, "could not schedule application event");
    }
}

fn sum_optional(a: Option<u64>, b: Option<u64>) -> Option<u64> {
    a.zip(b).map(|(x, y)| x + y).or(a).or(b)
}
fn sum_optional_f64(a: Option<f64>, b: Option<f64>) -> Option<f64> {
    a.zip(b).map(|(x, y)| x + y).or(a).or(b)
}

fn load_json<T: serde::de::DeserializeOwned>(path: &PathBuf) -> Option<T> {
    fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
}

pub fn write_json<T: serde::Serialize>(path: &PathBuf, value: &T) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    atomicwrites::AtomicFile::new(path, atomicwrites::AllowOverwrite)
        .write(|file| {
            use std::io::Write;
            file.write_all(&bytes)?;
            file.sync_all()
        })
        .map_err(|error| error.to_string())
}
