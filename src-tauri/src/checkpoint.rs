use crate::models::{LifecyclePhase, SESSION_SCHEMA_VERSION, SessionState};
use atomicwrites::{AllowOverwrite, AtomicFile};
use std::{fs, io::Write, path::Path};

pub fn write_atomic(path: &Path, session: &SessionState) -> Result<(), String> {
    if session.schema_version != SESSION_SCHEMA_VERSION {
        return Err("Refusing to persist an unsupported session schema.".into());
    }
    let bytes = serde_json::to_vec_pretty(session).map_err(|error| error.to_string())?;
    let parent = path
        .parent()
        .ok_or_else(|| "Checkpoint path has no parent directory.".to_string())?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    AtomicFile::new(path, AllowOverwrite)
        .write(|file| {
            file.write_all(&bytes)?;
            file.sync_all()
        })
        .map_err(|error| error.to_string())
}

pub fn load(path: &Path) -> Result<SessionState, String> {
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    let session: SessionState = serde_json::from_slice(&bytes)
        .map_err(|error| format!("Checkpoint is malformed: {error}"))?;
    migrate(session)
}

fn migrate(session: SessionState) -> Result<SessionState, String> {
    match session.schema_version {
        SESSION_SCHEMA_VERSION => Ok(session),
        version if version > SESSION_SCHEMA_VERSION => Err(format!(
            "This checkpoint uses schema {version}, but this app supports up to {SESSION_SCHEMA_VERSION}. Update Agentic Council to restore it."
        )),
        version => Err(format!(
            "Checkpoint schema {version} has no registered migration."
        )),
    }
}

pub fn is_recoverable(path: &Path) -> bool {
    load(path).is_ok_and(|session| {
        !matches!(
            session.phase,
            LifecyclePhase::PreSession | LifecyclePhase::Finalized
        )
    })
}

pub fn discard(path: &Path) -> Result<(), String> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checkpoint_roundtrip_preserves_session() {
        let dir =
            std::env::temp_dir().join(format!("agentic-council-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("checkpoint.json");
        let mut expected = SessionState::empty();
        expected.objective = "Test objective".into();
        write_atomic(&path, &expected).unwrap();
        let restored = load(&path).unwrap();
        assert_eq!(restored.id, expected.id);
        assert_eq!(restored.objective, expected.objective);
        let _ = fs::remove_dir_all(dir);
    }
}
