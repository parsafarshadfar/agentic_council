mod catalog;
mod checkpoint;
mod commands;
mod engine;
mod ingestion;
mod models;
mod providers;
mod report;
mod security;
mod state;

use state::{AppPaths, AppState};
use std::{
    backtrace::Backtrace,
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
};
use tauri::Manager;

struct LogGuard {
    _guard: tracing_appender::non_blocking::WorkerGuard,
}

fn install_panic_log(path: PathBuf) {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) {
            let _ = writeln!(
                file,
                "\n=== {:?} ===\nthread: {:?}\npanic: {info}\nbacktrace:\n{}",
                std::time::SystemTime::now(),
                std::thread::current().name(),
                Backtrace::force_capture()
            );
            let _ = file.sync_all();
        }
        previous(info);
    }));
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    if let Err(error) = providers::install_crypto_provider() {
        eprintln!("{error}");
        return;
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let data_dir = app.path().app_local_data_dir()?;
            let cache_dir = app.path().app_cache_dir()?;
            let paths = AppPaths::new(data_dir, cache_dir).map_err(std::io::Error::other)?;
            install_panic_log(paths.logs_dir.join("panic.log"));
            let file_appender =
                tracing_appender::rolling::daily(&paths.logs_dir, "agentic-council.log");
            let (writer, guard) = tracing_appender::non_blocking(file_appender);
            let _ = tracing_subscriber::fmt()
                .with_ansi(false)
                .with_writer(writer)
                .try_init();
            app.manage(LogGuard { _guard: guard });
            app.manage(AppState::new(paths));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::bootstrap,
            commands::round_poll,
            commands::start_preflight,
            commands::submit_clarification,
            commands::approve_aspects,
            commands::reject_aspects,
            commands::start_round,
            commands::stop_round,
            commands::retry_agent,
            commands::finalize_session,
            commands::new_session,
            commands::save_credential,
            commands::delete_credential,
            commands::test_connection,
            commands::refresh_models,
            commands::update_provider,
            commands::save_persona,
            commands::delete_persona,
            commands::ingest_files,
            commands::import_session,
            commands::export_markdown,
            commands::export_pdf,
            commands::restore_checkpoint,
            commands::discard_checkpoint,
            commands::hard_clear,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Agentic Council");
}
