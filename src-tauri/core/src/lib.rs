pub mod db;
pub mod imap;
pub mod models;
pub mod notification_parser;
pub mod notifications;
pub mod parser;
use serde::Serialize;
#[derive(Debug, Clone, Serialize, thiserror::Error)]
#[serde(rename_all = "camelCase")]
#[error("{message}")]
pub struct AppError {
    pub code: String,
    pub message: String,
}
impl AppError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}
impl From<rusqlite::Error> for AppError {
    fn from(_: rusqlite::Error) -> Self {
        Self::new(
            "DATABASE",
            "Operasi database lokal gagal. Data sebelumnya tetap tersimpan.",
        )
    }
}
impl From<serde_json::Error> for AppError {
    fn from(_: serde_json::Error) -> Self {
        Self::new("DATA_FORMAT", "Format data lokal tidak valid.")
    }
}
pub type Result<T> = std::result::Result<T, AppError>;

