use crate::{models::*, parser, AppError, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use uuid::Uuid;
#[derive(Clone)]
pub struct Database {
    path: PathBuf,
}
fn newid() -> String {
    Uuid::new_v4().to_string()
}
fn audit(c: &Connection, action: &str, id: &str, detail: &str) -> Result<()> {
    c.execute(
        "INSERT INTO audit_log(action,entity_id,detail) VALUES(?1,?2,?3)",
        params![action, id, detail],
    )?;
    Ok(())
}
impl Database {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p)
                .map_err(|_| AppError::new("STORAGE", "Folder data aplikasi tidak dapat dibuat."))?
        }
        let db = Self { path };
        let c = db.connection()?;
        c.pragma_update(None, "journal_mode", "WAL")?;
        c.execute_batch(include_str!("schema.sql"))?;
        for (id, name, kind, bank, digits) in [
            ("bca", "BCA Utama", "Bank", "BCA", ""),
            ("mandiri", "Mandiri Tabungan", "Bank", "Mandiri", ""),
            ("gopay", "GoPay", "E-Wallet", "GoPay", ""),
            ("cash", "Uang Tunai", "Tunai", "Cash", ""),
        ] {
            c.execute("INSERT OR IGNORE INTO wallets(id,name,kind,bank,digits,opening) VALUES(?1,?2,?3,?4,?5,0)",params![id,name,kind,bank,digits])?;
        }
        Ok(db)
    }
    pub fn connection(&self) -> Result<Connection> {
        let c = Connection::open(&self.path)?;
        c.busy_timeout(Duration::from_secs(5))?;
        c.pragma_update(None, "foreign_keys", "ON")?;
        Ok(c)
    }
    pub fn snapshot(&self) -> Result<Snapshot> {
        let c = self.connection()?;
        let wallets = read_wallets(&c)?;
        let mut q=c.prepare("SELECT id,wallet_id,to_wallet_id,kind,amount,merchant,category,occurred_at,source,reference,bank,email_id,note FROM ledger WHERE deleted=0 ORDER BY occurred_at DESC")?;
        let transactions = q
            .query_map([], |r| {
                Ok(LedgerEntry {
                    id: r.get(0)?,
                    wallet_id: r.get(1)?,
                    to_wallet_id: r.get(2)?,
                    kind: r.get(3)?,
                    amount: r.get(4)?,
                    merchant: r.get(5)?,
                    category: r.get(6)?,
                    occurred_at: r.get(7)?,
                    source: r.get(8)?,
                    reference: r.get(9)?,
                    bank: r.get(10)?,
                    email_id: r.get(11)?,
                    note: r.get(12)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let mut q=c.prepare("SELECT id,account_id,mailbox,uid,uid_validity,message_id,gm_id,sender,subject,received_at,raw_snippet,status,reason,parsed FROM emails ORDER BY CASE WHEN status IN ('review','error') THEN 0 ELSE 1 END,rowid DESC LIMIT 2000")?;
        let emails = q
            .query_map([], |r| {
                let p: Option<String> = r.get(13)?;
                Ok(EmailRecord {
                    id: r.get(0)?,
                    account_id: r.get(1)?,
                    mailbox: r.get(2)?,
                    uid: r.get(3)?,
                    uid_validity: r.get(4)?,
                    message_id: r.get(5)?,
                    gm_id: r.get(6)?,
                    sender: r.get(7)?,
                    subject: r.get(8)?,
                    received_at: r.get(9)?,
                    raw_snippet: r.get(10)?,
                    status: r.get(11)?,
                    reason: r.get(12)?,
                    parsed: p.and_then(|s| serde_json::from_str(&s).ok()),
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let accounts = self.accounts()?;
        let theme = c
            .query_row("SELECT value FROM preferences WHERE key='theme'", [], |r| {
                r.get(0)
            })
            .optional()?
            .unwrap_or_else(|| "light".into());
        Ok(Snapshot {
            wallets,
            transactions,
            emails,
            accounts,
            theme,
        })
    }
    pub fn accounts(&self) -> Result<Vec<Account>> {
        let c = self.connection()?;
        let mut q=c.prepare("SELECT id,email,provider,host,port,wallet_id,enabled,auto_post,folder,last_sync FROM accounts ORDER BY rowid")?;
        let mut accounts = q
            .query_map([], |r| {
                Ok(Account {
                    id: r.get(0)?,
                    email: r.get(1)?,
                    provider: r.get(2)?,
                    host: r.get(3)?,
                    port: r.get(4)?,
                    wallet_id: r.get(5)?,
                    enabled: r.get(6)?,
                    auto_post: r.get(7)?,
                    folder: r.get(8)?,
                    last_sync: r.get(9)?,
                    unlocked: false,
                    mappings: vec![],
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        for a in &mut accounts {
            let mut q = c.prepare("SELECT bank,wallet_id FROM mappings WHERE account_id=?1")?;
            a.mappings = q
                .query_map([&a.id], |r| {
                    Ok(Mapping {
                        bank: r.get(0)?,
                        wallet_id: r.get(1)?,
                    })
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;
        }
        Ok(accounts)
    }
    pub fn save_account(
        &self,
        email: &str,
        provider: &str,
        host: &str,
        port: u16,
        wallet: &str,
        auto_post: bool,
    ) -> Result<String> {
        let c = self.connection()?;
        require_wallet(&c, wallet)?;
        let existing: Option<String> = c
            .query_row(
                "SELECT id FROM accounts WHERE lower(email)=lower(?1)",
                [email],
                |r| r.get(0),
            )
            .optional()?;
        let id = existing.unwrap_or_else(newid);
        c.execute("INSERT INTO accounts(id,email,provider,host,port,wallet_id,auto_post) VALUES(?1,?2,?3,?4,?5,?6,?7) ON CONFLICT(id) DO UPDATE SET host=excluded.host,port=excluded.port,wallet_id=excluded.wallet_id,auto_post=excluded.auto_post,enabled=1",params![id,email,provider,host,port,wallet,auto_post])?;
        audit(
            &c,
            "connect_account",
            &id,
            "TLS authentication verified; credential session-only",
        )?;
        Ok(id)
    }
    pub fn update_account(&self, a: &AccountUpdate) -> Result<()> {
        let mut c = self.connection()?;
        let tx = c.transaction_with_behavior(TransactionBehavior::Immediate)?;
        require_wallet(&tx, &a.wallet_id)?;
        if tx.execute(
            "UPDATE accounts SET wallet_id=?1,enabled=?2,auto_post=?3 WHERE id=?4",
            params![a.wallet_id, a.enabled, a.auto_post, a.id],
        )? == 0
        {
            return Err(AppError::new("NOT_FOUND", "Akun email tidak ditemukan."));
        }
        tx.execute("DELETE FROM mappings WHERE account_id=?1", [&a.id])?;
        for m in &a.mappings {
            require_wallet(&tx, &m.wallet_id)?;
            tx.execute(
                "INSERT INTO mappings(account_id,bank,wallet_id) VALUES(?1,?2,?3)",
                params![a.id, m.bank, m.wallet_id],
            )?;
        }
        audit(
            &tx,
            "update_account",
            &a.id,
            "Mapping / enabled / auto-post changed",
        )?;
        tx.commit()?;
        Ok(())
    }
    pub fn set_theme(&self, theme: &str) -> Result<()> {
        if !["light", "dark"].contains(&theme) {
            return Err(AppError::new("VALIDATION", "Tema tidak valid."));
        }
        self.connection()?.execute("INSERT INTO preferences(key,value) VALUES('theme',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value",[theme])?;
        Ok(())
    }
    pub fn upsert_wallet(&self, w: &WalletInput) -> Result<String> {
        if w.name.trim().is_empty()
            || w.name.len() > 100
            || w.bank.len() > 60
            || w.opening < 0
            || w.opening > 100_000_000_000_000
            || !["Bank", "E-Wallet", "Tunai"].contains(&w.kind.as_str())
        {
            return Err(AppError::new(
                "VALIDATION",
                "Nama, jenis, atau saldo awal dompet tidak valid.",
            ));
        }
        let id = w.id.clone().unwrap_or_else(newid);
        let c = self.connection()?;
        c.execute("INSERT INTO wallets(id,name,kind,bank,digits,opening) VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(id) DO UPDATE SET name=excluded.name,kind=excluded.kind,bank=excluded.bank,digits=excluded.digits,opening=excluded.opening",params![id,w.name.trim(),w.kind,w.bank,w.digits,w.opening])?;
        audit(&c, "upsert_wallet", &id, "Wallet / opening balance changed")?;
        Ok(id)
    }
    pub fn observed(&self, id: &str, amount: i64, adjust: bool) -> Result<()> {
        if !(0..=100_000_000_000_000).contains(&amount) {
            return Err(AppError::new("VALIDATION", "Saldo pembanding tidak valid."));
        }
        let mut c = self.connection()?;
        let tx = c.transaction_with_behavior(TransactionBehavior::Immediate)?;
        require_wallet(&tx, id)?;
        tx.execute(
            "UPDATE wallets SET observed=?1 WHERE id=?2",
            params![amount, id],
        )?;
        if adjust {
            let w = read_wallets(&tx)?
                .into_iter()
                .find(|w| w.id == id)
                .ok_or_else(|| AppError::new("NOT_FOUND", "Dompet tidak ditemukan."))?;
            let diff = amount - w.balance;
            if diff != 0 {
                let e = EntryInput {
                    wallet_id: id.into(),
                    to_wallet_id: None,
                    kind: if diff > 0 { "income" } else { "expense" }.into(),
                    amount: diff.abs(),
                    merchant: "Penyesuaian saldo".into(),
                    category: if diff > 0 { "income" } else { "other" }.into(),
                    occurred_at: Utc::now().to_rfc3339(),
                    note: format!("Rekonsiliasi eksplisit ke saldo {amount}"),
                };
                insert_entry(&tx, &e, None, None, "Manual", "Manual")?;
            }
        }
        audit(
            &tx,
            "reconcile",
            id,
            if adjust {
                "Explicit ledger adjustment"
            } else {
                "Observed balance only"
            },
        )?;
        tx.commit()?;
        Ok(())
    }
    pub fn add_manual(&self, e: &EntryInput) -> Result<String> {
        let mut c = self.connection()?;
        let tx = c.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let id = insert_entry(&tx, e, None, None, "Manual", "Manual")?;
        audit(&tx, "manual_entry", &id, "Manual ledger entry")?;
        tx.commit()?;
        Ok(id)
    }
    pub fn delete_entry(&self, id: &str, undo: bool) -> Result<()> {
        let mut c = self.connection()?;
        let tx = c.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let deleted = if undo { 0 } else { 1 };
        if tx.execute(
            "UPDATE ledger SET deleted=?1 WHERE id=?2",
            params![deleted, id],
        )? == 0
        {
            return Err(AppError::new("NOT_FOUND", "Transaksi tidak ditemukan."));
        }
        tx.execute(
            "UPDATE emails SET status=?1 WHERE id=(SELECT email_id FROM ledger WHERE id=?2)",
            params![if undo { "posted" } else { "ignored" }, id],
        )?;
        audit(
            &tx,
            if undo { "undo_delete" } else { "delete_entry" },
            id,
            "Soft delete; reference stays reserved",
        )?;
        tx.commit()?;
        Ok(())
    }
    pub fn resolve_review(&self, r: &ReviewInput) -> Result<()> {
        let mut c = self.connection()?;
        let tx = c.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (status, parsed): (String, Option<String>) = tx
            .query_row(
                "SELECT status,parsed FROM emails WHERE id=?1",
                [&r.email_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?
            .ok_or_else(|| AppError::new("NOT_FOUND", "Email tidak ditemukan."))?;
        if !["review", "error"].contains(&status.as_str()) {
            return Err(AppError::new(
                "CONFLICT",
                "Email ini sudah diproses. Muat ulang daftar.",
            ));
        }
        if r.action == "ignore" {
            tx.execute(
                "UPDATE emails SET status='ignored',reason='Diabaikan pengguna' WHERE id=?1",
                [&r.email_id],
            )?;
        } else if r.action == "post" {
            let e = r
                .entry
                .as_ref()
                .ok_or_else(|| AppError::new("VALIDATION", "Data koreksi belum lengkap."))?;
            let p: Option<Parsed> = parsed.and_then(|v| serde_json::from_str(&v).ok());
            let bank = p.as_ref().map(|p| p.bank.as_str()).unwrap_or("Unknown");
            let reference = p.as_ref().and_then(|p| p.reference.as_deref());
            let existing: Option<String> = tx
                .query_row(
                    "SELECT id FROM ledger WHERE email_id=?1",
                    [&r.email_id],
                    |r| r.get(0),
                )
                .optional()?;
            if let Some(id) = existing {
                validate_entry(&tx, e)?;
                tx.execute("UPDATE ledger SET deleted=0,wallet_id=?1,to_wallet_id=?2,kind=?3,amount=?4,merchant=?5,category=?6,occurred_at=?7,note=?8 WHERE id=?9",params![e.wallet_id,e.to_wallet_id,e.kind,e.amount,e.merchant,category_for(e),e.occurred_at,e.note,id])?;
            } else {
                if reference_duplicate(&tx, &e.wallet_id, bank, reference)? {
                    return Err(AppError::new(
                        "DUPLICATE",
                        "Referensi bank sudah tercatat. Tidak dibuat mutasi ganda.",
                    ));
                }
                insert_entry(&tx, e, reference, Some(&r.email_id), "Email", bank)?;
            }
            tx.execute("UPDATE emails SET status='posted',reason='Dikonfirmasi dan dikoreksi pengguna' WHERE id=?1",[&r.email_id])?;
        } else {
            return Err(AppError::new("VALIDATION", "Aksi review tidak dikenal."));
        }
        audit(
            &tx,
            &format!("review_{}", r.action),
            &r.email_id,
            "User confirmed email action",
        )?;
        tx.commit()?;
        Ok(())
    }
    pub fn undo_review(&self, id: &str) -> Result<()> {
        let mut c = self.connection()?;
        let tx = c.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let status: Option<String> = tx
            .query_row("SELECT status FROM emails WHERE id=?1", [id], |r| r.get(0))
            .optional()?;
        if !status.is_some_and(|s| ["posted", "ignored"].contains(&s.as_str())) {
            return Err(AppError::new(
                "CONFLICT",
                "Status email tidak dapat di-undo.",
            ));
        }
        tx.execute("UPDATE ledger SET deleted=1 WHERE email_id=?1", [id])?;
        tx.execute(
            "UPDATE emails SET status='review',reason='Aksi dibatalkan pengguna' WHERE id=?1",
            [id],
        )?;
        audit(
            &tx,
            "undo_review",
            id,
            "Returned to review; ledger entry soft deleted",
        )?;
        tx.commit()?;
        Ok(())
    }
    pub fn cursor(&self, account: &str, mailbox: &str, validity: u32) -> Result<u32> {
        let c = self.connection()?;
        let old: Option<(u32, u32)> = c
            .query_row(
                "SELECT uid_validity,last_uid FROM cursors WHERE account_id=?1 AND mailbox=?2",
                params![account, mailbox],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        if let Some((v, u)) = old {
            if v == validity {
                return Ok(u);
            }
        }
        c.execute("INSERT INTO cursors(account_id,mailbox,uid_validity,last_uid) VALUES(?1,?2,?3,0) ON CONFLICT(account_id,mailbox) DO UPDATE SET uid_validity=excluded.uid_validity,last_uid=0",params![account,mailbox,validity])?;
        audit(
            &c,
            "uidvalidity_reset",
            account,
            "Cursor reset; message/ref dedup retained",
        )?;
        Ok(0)
    }
    /// Filter committed messages, not the global cursor, so broader ranges can backfill.
    pub fn pending_uids(&self, account: &str, mailbox: &str, validity: u32, candidates: Vec<u32>) -> Result<Vec<u32>> {
        let c = self.connection()?;
        let mut q = c.prepare("SELECT uid FROM emails WHERE account_id=?1 AND mailbox=?2 AND uid_validity=?3 UNION SELECT uid FROM sync_receipts WHERE account_id=?1 AND mailbox=?2 AND uid_validity=?3")?;
        let seen = q.query_map(params![account, mailbox, validity], |r| r.get::<_, u32>(0))?
            .collect::<std::result::Result<std::collections::HashSet<_>, _>>()?;
        Ok(candidates.into_iter().filter(|uid| !seen.contains(uid)).collect())
    }
    pub fn mark_synced(&self, id: &str, folder: &str) -> Result<()> {
        self.connection()?.execute(
            "UPDATE accounts SET last_sync=?1,folder=?2 WHERE id=?3",
            params![Utc::now().to_rfc3339(), folder, id],
        )?;
        Ok(())
    }
    /// The email, optional ledger entry, and cursor are committed together.
    pub fn ingest(&self, input: &MailInput, account: &Account) -> Result<String> {
        let mut email = parser::parse_message(input, account)?;
        let mut c = self.connection()?;
        let tx = c.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let duplicate:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM emails WHERE account_id=?1 AND (message_id=?2 OR (?3 IS NOT NULL AND gm_id=?3) OR (mailbox=?4 AND uid_validity=?5 AND uid=?6)))",params![email.account_id,email.message_id,email.gm_id,email.mailbox,email.uid_validity,email.uid],|r|r.get(0))?;
        if duplicate {
            advance(&tx, input)?;
            tx.commit()?;
            return Ok("email_duplicate".into());
        }
        if let Some(p) = &email.parsed {
            if reference_duplicate(&tx, &p.wallet_id, &p.bank, p.reference.as_deref())? {
                email.status = "duplicate".into();
                email.reason = "Referensi bank sudah tercatat (deduplikasi lapis 2)".into()
            }
        }
        tx.execute("INSERT INTO emails(id,account_id,mailbox,uid_validity,uid,message_id,gm_id,sender,subject,received_at,raw_snippet,status,reason,parsed) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",params![email.id,email.account_id,email.mailbox,email.uid_validity,email.uid,email.message_id,email.gm_id,email.sender,email.subject,email.received_at,email.raw_snippet,email.status,email.reason,email.parsed.as_ref().map(serde_json::to_string).transpose()?])?;
        if email.status == "posted" {
            if let Some(p) = &email.parsed {
                let e = EntryInput {
                    wallet_id: p.wallet_id.clone(),
                    to_wallet_id: p.to_wallet_id.clone(),
                    kind: p.kind.clone(),
                    amount: p.amount,
                    merchant: p.merchant.clone(),
                    category: p.category.clone(),
                    occurred_at: p.occurred_at.clone(),
                    note: "Otomatis dari email; bukti asli tersimpan lokal".into(),
                };
                insert_entry(
                    &tx,
                    &e,
                    p.reference.as_deref(),
                    Some(&email.id),
                    "Email",
                    &p.bank,
                )?;
                if let Some(obs) = p.observed {
                    tx.execute(
                        "UPDATE wallets SET observed=?1 WHERE id=?2",
                        params![obs, p.wallet_id],
                    )?;
                }
            }
        }
        advance(&tx, input)?;
        audit(&tx, "ingest_email", &email.id, &email.status)?;
        let result = email.status.clone();
        tx.commit()?;
        Ok(result)
    }
}
fn advance(tx: &Transaction<'_>, m: &MailInput) -> Result<()> {
    tx.execute("INSERT OR IGNORE INTO sync_receipts(account_id,mailbox,uid_validity,uid) VALUES(?1,?2,?3,?4)", params![m.account_id,m.mailbox,m.uid_validity,m.uid])?;
    tx.execute("INSERT INTO cursors(account_id,mailbox,uid_validity,last_uid) VALUES(?1,?2,?3,?4) ON CONFLICT(account_id,mailbox) DO UPDATE SET last_uid=MAX(last_uid,excluded.last_uid),uid_validity=excluded.uid_validity",params![m.account_id,m.mailbox,m.uid_validity,m.uid])?;
    Ok(())
}
fn require_wallet(c: &Connection, id: &str) -> Result<()> {
    let exists: bool = c.query_row(
        "SELECT EXISTS(SELECT 1 FROM wallets WHERE id=?1)",
        [id],
        |r| r.get(0),
    )?;
    if exists {
        Ok(())
    } else {
        Err(AppError::new(
            "VALIDATION",
            "Dompet tujuan tidak ditemukan.",
        ))
    }
}
fn category_for(e: &EntryInput) -> &str {
    match e.kind.as_str() {
        "income" => "income",
        "transfer" => "transfer",
        _ => &e.category,
    }
}
fn validate_entry(c: &Connection, e: &EntryInput) -> Result<()> {
    if e.amount <= 0
        || e.amount > 100_000_000_000_000
        || e.merchant.trim().is_empty()
        || e.merchant.len() > 200
        || e.note.len() > 2000
        || !["expense", "income", "transfer"].contains(&e.kind.as_str())
        || DateTime::parse_from_rfc3339(&e.occurred_at).is_err()
        || ![
            "food",
            "shop",
            "transport",
            "home",
            "fun",
            "other",
            "income",
            "transfer",
        ]
        .contains(&e.category.as_str())
    {
        return Err(AppError::new(
            "VALIDATION",
            "Nominal, kategori, nama, atau waktu transaksi tidak valid.",
        ));
    }
    if e.kind == "expense" && ["income", "transfer"].contains(&e.category.as_str()) {
        return Err(AppError::new("VALIDATION", "Pilih kategori pengeluaran."));
    }
    require_wallet(c, &e.wallet_id)?;
    if e.kind == "transfer" {
        let to = e
            .to_wallet_id
            .as_deref()
            .ok_or_else(|| AppError::new("VALIDATION", "Dompet tujuan wajib diisi."))?;
        if to == e.wallet_id {
            return Err(AppError::new(
                "VALIDATION",
                "Dompet asal dan tujuan harus berbeda.",
            ));
        }
        require_wallet(c, to)?;
    }
    Ok(())
}
fn insert_entry(
    tx: &Connection,
    e: &EntryInput,
    reference: Option<&str>,
    email_id: Option<&str>,
    source: &str,
    bank: &str,
) -> Result<String> {
    validate_entry(tx, e)?;
    let id = newid();
    tx.execute("INSERT INTO ledger(id,wallet_id,to_wallet_id,kind,amount,merchant,category,occurred_at,source,reference,bank,email_id,note) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",params![id,e.wallet_id,if e.kind=="transfer"{e.to_wallet_id.as_deref()}else{None},e.kind,e.amount,e.merchant.trim(),category_for(e),e.occurred_at,source,reference,bank,email_id,e.note])?;
    Ok(id)
}
fn reference_duplicate(
    c: &Connection,
    wallet: &str,
    bank: &str,
    reference: Option<&str>,
) -> Result<bool> {
    if let Some(r) = reference {
        Ok(c.query_row(
            "SELECT EXISTS(SELECT 1 FROM ledger WHERE wallet_id=?1 AND bank=?2 AND reference=?3)",
            params![wallet, bank, r],
            |r| r.get(0),
        )?)
    } else {
        Ok(false)
    }
}
fn read_wallets(c: &Connection) -> Result<Vec<Wallet>> {
    let mut q=c.prepare("SELECT w.id,w.name,w.kind,w.bank,w.digits,w.opening,w.observed,w.opening+COALESCE((SELECT SUM(CASE WHEN l.wallet_id=w.id THEN CASE WHEN l.kind='income' THEN l.amount ELSE -l.amount END ELSE 0 END + CASE WHEN l.kind='transfer' AND l.to_wallet_id=w.id THEN l.amount ELSE 0 END) FROM ledger l WHERE l.deleted=0 AND (l.wallet_id=w.id OR l.to_wallet_id=w.id)),0) FROM wallets w ORDER BY rowid")?;
    let rows = q
        .query_map([], |r| {
            Ok(Wallet {
                id: r.get(0)?,
                name: r.get(1)?,
                kind: r.get(2)?,
                bank: r.get(3)?,
                digits: r.get(4)?,
                opening: r.get(5)?,
                observed: r.get(6)?,
                balance: r.get(7)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(rows)
}
