use arus_core::{
    db::{BatchReviewInput, BatchReviewReport, Database},
    imap,
    models::*,
    AppError, Result,
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tauri::{ipc::Channel, Manager, State};
use zeroize::Zeroizing;
struct AppState {
    db: Database,
    secrets: Mutex<HashMap<String, Zeroizing<String>>>,
    sync_gate: Arc<tokio::sync::Mutex<()>>,
}
fn secret_error() -> AppError {
    AppError::new(
        "SESSION",
        "Penyimpanan sesi tidak tersedia. Buka ulang aplikasi.",
    )
}
async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Result<T> + Send + 'static) -> Result<T> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|_| AppError::new("TASK", "Tugas lokal terhenti. Coba ulangi."))?
}
#[tauri::command]
async fn init_database(state: State<'_, AppState>) -> Result<Snapshot> {
    get_snapshot(state).await
}
#[tauri::command]
async fn get_snapshot(state: State<'_, AppState>) -> Result<Snapshot> {
    let db = state.db.clone();
    let mut snapshot = blocking(move || db.snapshot()).await?;
    let secrets = state.secrets.lock().map_err(|_| secret_error())?;
    for a in &mut snapshot.accounts {
        a.unlocked = secrets.contains_key(&a.id)
    }
    Ok(snapshot)
}
#[tauri::command]
async fn test_imap_connection(input: ConnectionInput) -> Result<ConnectionReport> {
    imap::test(input).await
}
#[tauri::command]
async fn save_email_account(state: State<'_, AppState>, input: SaveAccountInput) -> Result<String> {
    let ep = imap::endpoint(input.connection)?;
    imap::test_endpoint(&ep).await?;
    let id = state.db.save_account(
        &ep.email,
        &ep.provider,
        &ep.host,
        ep.port,
        &input.wallet_id,
        input.auto_post,
    )?;
    state
        .secrets
        .lock()
        .map_err(|_| secret_error())?
        .insert(id.clone(), ep.password);
    Ok(id)
}
#[tauri::command]
async fn unlock_email_account(
    state: State<'_, AppState>,
    account_id: String,
    password: String,
) -> Result<ConnectionReport> {
    let a = state
        .db
        .accounts()?
        .into_iter()
        .find(|a| a.id == account_id)
        .ok_or_else(|| AppError::new("NOT_FOUND", "Akun tidak ditemukan."))?;
    let ep = imap::endpoint(ConnectionInput {
        email: a.email,
        provider: a.provider,
        host: Some(a.host),
        port: Some(a.port),
        password,
    })?;
    let report = imap::test_endpoint(&ep).await?;
    state
        .secrets
        .lock()
        .map_err(|_| secret_error())?
        .insert(account_id, ep.password);
    Ok(report)
}
#[tauri::command]
fn lock_email_account(state: State<'_, AppState>, account_id: String) -> Result<()> {
    state
        .secrets
        .lock()
        .map_err(|_| secret_error())?
        .remove(&account_id);
    Ok(())
}
#[tauri::command]
async fn sync_email_transactions(
    state: State<'_, AppState>,
    account_id: Option<String>,
    range: Option<SyncRange>,
    on_progress: Channel<SyncProgress>,
) -> Result<SyncReport> {
    let gate = state.sync_gate.clone();
    let _guard = gate
        .try_lock()
        .map_err(|_| AppError::new("SYNC_BUSY", "Sinkronisasi sedang berjalan."))?;
    let accounts = state
        .db
        .accounts()?
        .into_iter()
        .filter(|a| a.enabled && account_id.as_ref().is_none_or(|id| id == &a.id))
        .collect::<Vec<_>>();
    if accounts.is_empty() {
        return Err(AppError::new(
            "NO_ACCOUNT",
            "Hubungkan dan aktifkan akun email terlebih dahulu.",
        ));
    }
    let mut result = SyncReport::default();
    for a in accounts {
        let password = {
            state
                .secrets
                .lock()
                .map_err(|_| secret_error())?
                .get(&a.id)
                .cloned()
        };
        let Some(password) = password else {
            result.warnings.push(format!(
                "{}: masukkan App Password untuk membuka sesi.",
                a.email
            ));
            continue;
        };
        match imap::sync(&state.db, &a, password, &|p| {
            let _ = on_progress.send(p);
        }, range.unwrap_or_default())
        .await
        {
            Ok(r) => {
                result.fetched += r.fetched;
                result.posted += r.posted;
                result.review += r.review;
                result.ignored += r.ignored;
                result.errors += r.errors;
                result.email_duplicates += r.email_duplicates;
                result.ledger_duplicates += r.ledger_duplicates;
                result.remaining += r.remaining;
                result.warnings.extend(r.warnings)
            }
            Err(e) => {
                if e.code == "AUTHENTICATION_FAILED" {
                    state
                        .secrets
                        .lock()
                        .map_err(|_| secret_error())?
                        .remove(&a.id);
                }
                result
                    .warnings
                    .push(format!("{} · {}: {}", a.email, e.code, e.message));
            }
        }
    }
    Ok(result)
}
#[tauri::command]
async fn save_manual_transaction(state: State<'_, AppState>, input: EntryInput) -> Result<String> {
    let db = state.db.clone();
    blocking(move || db.add_manual(&input)).await
}
#[tauri::command]
async fn delete_transaction(state: State<'_, AppState>, id: String, undo: bool) -> Result<()> {
    let db = state.db.clone();
    blocking(move || db.delete_entry(&id, undo)).await
}
#[tauri::command]
async fn resolve_review(state: State<'_, AppState>, input: ReviewInput) -> Result<()> {
    let db = state.db.clone();
    blocking(move || db.resolve_review(&input)).await
}
#[tauri::command]
async fn batch_resolve_review(
    state: State<'_, AppState>,
    input: BatchReviewInput,
) -> Result<BatchReviewReport> {
    let db = state.db.clone();
    blocking(move || db.batch_resolve_review(&input)).await
}
#[tauri::command]
async fn undo_review(state: State<'_, AppState>, id: String) -> Result<()> {
    let db = state.db.clone();
    blocking(move || db.undo_review(&id)).await
}
#[tauri::command]
async fn upsert_wallet(state: State<'_, AppState>, input: WalletInput) -> Result<String> {
    let db = state.db.clone();
    blocking(move || db.upsert_wallet(&input)).await
}
#[tauri::command]
async fn reconcile_wallet(
    state: State<'_, AppState>,
    id: String,
    observed: i64,
    adjust: bool,
) -> Result<()> {
    let db = state.db.clone();
    blocking(move || db.observed(&id, observed, adjust)).await
}
#[tauri::command]
async fn update_email_account(state: State<'_, AppState>, input: AccountUpdate) -> Result<()> {
    let db = state.db.clone();
    blocking(move || db.update_account(&input)).await
}
#[tauri::command]
async fn set_theme(state: State<'_, AppState>, theme: String) -> Result<()> {
    let db = state.db.clone();
    blocking(move || db.set_theme(&theme)).await
}
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let directory = app.path().app_data_dir()?;
            let db = Database::open(directory.join("arus.sqlite3"))?;
            app.manage(AppState {
                db,
                secrets: Mutex::new(HashMap::new()),
                sync_gate: Arc::new(tokio::sync::Mutex::new(())),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            init_database,
            get_snapshot,
            test_imap_connection,
            save_email_account,
            unlock_email_account,
            lock_email_account,
            sync_email_transactions,
            save_manual_transaction,
            delete_transaction,
            resolve_review,
            batch_resolve_review,
            undo_review,
            upsert_wallet,
            reconcile_wallet,
            update_email_account,
            set_theme
        ])
        .run(tauri::generate_context!())
        .expect("Tidak dapat memulai aplikasi Arus");
}
