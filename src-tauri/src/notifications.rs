use arus_core::models::{NotificationImportReport, NotificationInput};
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

#[derive(Debug, Deserialize)]
struct Batch {
    items: Vec<NotificationInput>,
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
            let _ = plugin
                .0
                .run_mobile_plugin::<Value>("openSettings", json!({}))
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
            let _ = plugin
                .0
                .run_mobile_plugin::<Value>("setEnabled", json!({ "enabled": enabled }))
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
pub async fn notification_import(app: AppHandle) -> Result<NotificationImportReport, String> {
    #[cfg(target_os = "android")]
    {
        use crate::AppState;
        let native = app.clone();
        tauri::async_runtime::spawn_blocking(move || -> Result<NotificationImportReport, String> {
            let plugin = native.state::<Native<tauri::Wry>>();
            let raw: Value = plugin.0.run_mobile_plugin("peek", json!({})).map_err(|e|e.to_string())?;
            let batch: Batch = serde_json::from_value(raw).map_err(|e|e.to_string())?;
            let state = native.state::<AppState>();
            let imported = state.db.import_notifications(&batch.items);
            let mut report = imported.report;
            if !imported.ack_ids.is_empty() {
                let count = imported.ack_ids.len();
                match plugin.0.run_mobile_plugin::<Value>("ack", json!({"ids": imported.ack_ids})) {
                    Ok(_) => report.acknowledged = count,
                    Err(_) => report.warnings.push("Penyimpanan berhasil, tetapi ACK antrean gagal. Tarik ulang; receipt mencegah transaksi ganda.".into()),
                }
            }
            Ok(report)
        }).await.map_err(|e|e.to_string())?
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = app;
        Ok(NotificationImportReport::default())
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub available: bool,
    pub current_version: String,
    pub current_version_code: i64,
    pub latest_version: String,
    pub latest_version_code: i64,
    pub notes: String,
    pub apk_url: String,
    pub apk_size: i64,
}

#[tauri::command]
pub async fn check_app_update(app: AppHandle) -> Result<UpdateInfo, String> {
    #[cfg(target_os = "android")]
    {
        let res = tauri::async_runtime::spawn_blocking(move || {
            let plugin = app.state::<Native<tauri::Wry>>();
            plugin
                .0
                .run_mobile_plugin::<Value>("checkUpdate", json!({}))
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

        let info: UpdateInfo = serde_json::from_value(res).map_err(|e| e.to_string())?;
        Ok(info)
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = app;
        Ok(UpdateInfo {
            available: false,
            current_version: app.package_info().version.to_string(),
            current_version_code: 1,
            latest_version: app.package_info().version.to_string(),
            latest_version_code: 1,
            notes: "Mode desktop/browser".to_string(),
            apk_url: "".to_string(),
            apk_size: 0,
        })
    }
}

#[tauri::command]
pub async fn download_app_update(app: AppHandle) -> Result<bool, String> {
    #[cfg(target_os = "android")]
    {
        tauri::async_runtime::spawn_blocking(move || -> Result<bool, String> {
            let plugin = app.state::<Native<tauri::Wry>>();
            let _ = plugin
                .0
                .run_mobile_plugin::<Value>("downloadUpdate", json!({}))
                .map_err(|e| e.to_string())?;
            Ok(true)
        })
        .await
        .map_err(|e| e.to_string())?
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = app;
        Ok(false)
    }
}

#[tauri::command]
pub async fn install_app_update(app: AppHandle) -> Result<bool, String> {
    #[cfg(target_os = "android")]
    {
        tauri::async_runtime::spawn_blocking(move || -> Result<bool, String> {
            let plugin = app.state::<Native<tauri::Wry>>();
            let _ = plugin
                .0
                .run_mobile_plugin::<Value>("installUpdate", json!({}))
                .map_err(|e| e.to_string())?;
            Ok(true)
        })
        .await
        .map_err(|e| e.to_string())?
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = app;
        Ok(false)
    }
}

#[tauri::command]
pub async fn update_install_permission(app: AppHandle) -> Result<bool, String> {
    #[cfg(target_os = "android")]
    {
        tauri::async_runtime::spawn_blocking(move || {
            let value = app
                .state::<Native<tauri::Wry>>()
                .0
                .run_mobile_plugin::<Value>("updateInstallPermission", json!({}))
                .map_err(|e| e.to_string())?;
            Ok(value
                .get("granted")
                .and_then(Value::as_bool)
                .unwrap_or(false))
        })
        .await
        .map_err(|e| e.to_string())?
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = app;
        Ok(false)
    }
}
#[tauri::command]
pub async fn update_install_settings(app: AppHandle) -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        tauri::async_runtime::spawn_blocking(move || {
            app.state::<Native<tauri::Wry>>()
                .0
                .run_mobile_plugin::<Value>("updateInstallSettings", json!({}))
                .map(|_| ())
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = app;
        Err("Pemasangan pembaruan hanya tersedia di Android.".into())
    }
}
