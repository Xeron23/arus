use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub enum SyncRange {
    #[serde(rename = "7")] Week,
    #[default]
    #[serde(rename = "30")] Month,
    #[serde(rename = "90")] Quarter,
    #[serde(rename = "all")] All,
}
impl SyncRange {
    pub fn search_query(self, now: chrono::DateTime<chrono::Utc>) -> String {
        let days = match self { Self::Week => 7, Self::Month => 30, Self::Quarter => 90, Self::All => return "ALL".into() };
        // IMAP SINCE is inclusive and has calendar-day precision, not rolling hours.
        let date = (now.with_timezone(&chrono::FixedOffset::east_opt(7 * 3600).unwrap()) - chrono::Duration::days(days)).date_naive();
        format!("SINCE {}", date.format("%d-%b-%Y"))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Wallet {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub bank: String,
    pub digits: String,
    pub opening: i64,
    pub balance: i64,
    pub observed: Option<i64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletInput {
    pub id: Option<String>,
    pub name: String,
    pub kind: String,
    pub bank: String,
    pub digits: String,
    pub opening: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerEntry {
    pub id: String,
    pub wallet_id: String,
    pub to_wallet_id: Option<String>,
    pub kind: String,
    pub amount: i64,
    pub merchant: String,
    pub category: String,
    pub occurred_at: String,
    pub source: String,
    pub reference: Option<String>,
    pub bank: String,
    pub email_id: Option<String>,
    pub note: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryInput {
    pub wallet_id: String,
    pub to_wallet_id: Option<String>,
    pub kind: String,
    pub amount: i64,
    pub merchant: String,
    pub category: String,
    pub occurred_at: String,
    pub note: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Mapping {
    pub bank: String,
    pub wallet_id: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: String,
    pub email: String,
    pub provider: String,
    pub host: String,
    pub port: u16,
    pub wallet_id: String,
    pub enabled: bool,
    pub auto_post: bool,
    pub folder: Option<String>,
    pub last_sync: Option<String>,
    pub unlocked: bool,
    pub mappings: Vec<Mapping>,
}
#[derive(Clone, Serialize, Deserialize)] // Deliberately no Debug: includes a credential.
#[serde(rename_all = "camelCase")]
pub struct ConnectionInput {
    pub email: String,
    pub password: String,
    pub provider: String,
    pub host: Option<String>,
    pub port: Option<u16>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveAccountInput {
    pub connection: ConnectionInput,
    pub wallet_id: String,
    pub auto_post: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountUpdate {
    pub id: String,
    pub wallet_id: String,
    pub enabled: bool,
    pub auto_post: bool,
    pub mappings: Vec<Mapping>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionReport {
    pub host: String,
    pub port: u16,
    pub provider: String,
    pub folder: String,
    pub elapsed_ms: u64,
    pub tls: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Parsed {
    pub bank: String,
    pub amount: i64,
    pub kind: String,
    pub merchant: String,
    pub category: String,
    pub occurred_at: String,
    pub reference: Option<String>,
    pub observed: Option<i64>,
    pub wallet_id: String,
    pub to_wallet_id: Option<String>,
    pub confident: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmailRecord {
    pub id: String,
    pub account_id: String,
    pub mailbox: String,
    pub uid: u32,
    pub uid_validity: u32,
    pub message_id: String,
    pub gm_id: Option<String>,
    pub sender: String,
    pub subject: String,
    pub received_at: String,
    pub raw_snippet: String,
    pub status: String,
    pub reason: String,
    pub parsed: Option<Parsed>,
}
#[derive(Clone, Debug)]
pub struct MailInput {
    pub account_id: String,
    pub mailbox: String,
    pub uid: u32,
    pub uid_validity: u32,
    pub gm_id: Option<String>,
    pub raw: Vec<u8>,
    pub oversized: bool,
    pub received_at: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub wallets: Vec<Wallet>,
    pub transactions: Vec<LedgerEntry>,
    pub emails: Vec<EmailRecord>,
    pub accounts: Vec<Account>,
    pub theme: String,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncReport {
    pub fetched: u32,
    pub posted: u32,
    pub review: u32,
    pub ignored: u32,
    pub errors: u32,
    pub email_duplicates: u32,
    pub ledger_duplicates: u32,
    pub remaining: u32,
    pub warnings: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncProgress {
    pub phase: String,
    pub message: String,
    pub account_id: String,
    pub current: u32,
    pub total: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewInput {
    pub email_id: String,
    pub action: String,
    pub entry: Option<EntryInput>,
}
