use arus_core::models::EntryInput;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::{AppHandle, Manager};

#[cfg(target_os = "android")]
use tauri_plugin_notification_listener::Native;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationStatus {
    pub granted: bool,
    pub enabled: bool,
    pub connected: bool,
    pub last_error: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct RawNotification {
    id: String,
    package_name: String,
    title: String,
    text: String,
    timestamp: i64,
}

#[derive(Debug, Deserialize)]
struct Batch {
    items: Vec<RawNotification>,
}

#[tauri::command]
pub async fn notification_status(app: AppHandle) -> Result<NotificationStatus, String> {
    #[cfg(target_os = "android")]
    {
        let res = tauri::async_runtime::spawn_blocking(move || {
            let plugin = app.state::<Native<tauri::Wry>>();
            plugin.0.run_mobile_plugin("status", json!({}))
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

        let status: NotificationStatus = serde_json::from_value(res).map_err(|e| e.to_string())?;
        Ok(status)
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = app;
        Ok(NotificationStatus {
            granted: true,
            enabled: true,
            connected: true,
            last_error: "".to_string(),
        })
    }
}

#[tauri::command]
pub async fn notification_open_settings(app: AppHandle) -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
            let plugin = app.state::<Native<tauri::Wry>>();
            let _ = plugin.0.run_mobile_plugin("openSettings", json!({}))
                .map_err(|e| e.to_string())?;
            Ok(())
        })
        .await
        .map_err(|e| e.to_string())?
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = app;
        Ok(())
    }
}

#[tauri::command]
pub async fn notification_set_enabled(app: AppHandle, enabled: bool) -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
            let plugin = app.state::<Native<tauri::Wry>>();
            let _ = plugin.0.run_mobile_plugin("setEnabled", json!({ "enabled": enabled }))
                .map_err(|e| e.to_string())?;
            Ok(())
        })
        .await
        .map_err(|e| e.to_string())?
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = (app, enabled);
        Ok(())
    }
}

#[tauri::command]
pub async fn notification_import(app: AppHandle) -> Result<usize, String> {
    #[cfg(target_os = "android")]
    {
        use arus_core::notification_parser::parse_notification;
        use crate::AppState;

        let raw_batch: Value = tauri::async_runtime::spawn_blocking({
            let app = app.clone();
            move || {
                let plugin = app.state::<Native<tauri::Wry>>();
                plugin.0.run_mobile_plugin("peek", json!({}))
            }
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

        let batch: Batch = serde_json::from_value(raw_batch).map_err(|e| e.to_string())?;
        if batch.items.is_empty() {
            return Ok(0);
        }

        let state = app.state::<AppState>();
        let mut processed_ids = Vec::new();
        let now = Utc::now().to_rfc3339();

        for item in &batch.items {
            if let Some(parsed) = parse_notification(&item.package_name, &item.title, &item.text) {
                let wallet_id = parsed.wallet_hint.clone();
                let category = if parsed.direction == "income" { "Pendapatan" } else { "Makanan & Minuman" };
                let note = format!("Notifikasi: {} | {}", item.title, item.text);

                let entry = EntryInput {
                    wallet_id,
                    to_wallet_id: None,
                    kind: parsed.direction,
                    amount: parsed.amount_idr,
                    merchant: parsed.merchant,
                    category: category.to_string(),
                    occurred_at: now.clone(),
                    note,
                };

                // Add to ledger via Database
                let _ = state.db.add_manual(&entry);
            }
            processed_ids.push(item.id.clone());
        }

        // Ack processed IDs
        if !processed_ids.is_empty() {
            let _ = tauri::async_runtime::spawn_blocking({
                let app = app.clone();
                move || {
                    let plugin = app.state::<Native<tauri::Wry>>();
                    plugin.0.run_mobile_plugin("ack", json!({ "ids": processed_ids }))
                }
            })
            .await;
        }

        Ok(batch.items.len())
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = app;
        Ok(0)
    }
}
