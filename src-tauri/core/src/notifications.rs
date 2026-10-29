//! Durable inbox + ledger receipt. Android ACK is allowed only after this commit.
use crate::{
    db::{audit, insert_entry, Database},
    models::*,
    notification_parser, parser, AppError, Result,
};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};

pub(crate) fn read_events(c: &Connection) -> Result<Vec<NotificationEvent>> {
    let mut q = c.prepare("SELECT id,package_name,title,body,timestamp,status,reason,ledger_id FROM notification_events ORDER BY timestamp DESC")?;
    let events = q
        .query_map([], |r| {
            Ok(NotificationEvent {
                input: NotificationInput {
                    id: r.get(0)?,
                    package_name: r.get(1)?,
                    title: r.get(2)?,
                    text: r.get(3)?,
                    timestamp: r.get(4)?,
                },
                status: r.get(5)?,
                reason: r.get(6)?,
                ledger_id: r.get(7)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(events)
}

fn wallet_for(c: &Connection, hint: &str) -> Result<Option<String>> {
    let bank = match hint {
        "gopay" => "GoPay",
        "dana" => "DANA",
        "ovo" => "OVO",
        "bca" => "BCA",
        "mandiri" => "Mandiri",
        "shopeepay" => "ShopeePay",
        _ => return Ok(None),
    };
    let mut q = c.prepare("SELECT id FROM wallets WHERE lower(bank)=lower(?1) ORDER BY id")?;
    let ids = q
        .query_map([bank], |r| r.get::<_, String>(0))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    match ids.len() {
        1 => Ok(ids.into_iter().next()),
        0 => {
            let occupied: bool = c.query_row(
                "SELECT EXISTS(SELECT 1 FROM wallets WHERE id=?1)",
                [hint],
                |r| r.get(0),
            )?;
            let id = if occupied {
                uuid::Uuid::new_v4().to_string()
            } else {
                hint.into()
            };
            c.execute(
                "INSERT INTO wallets(id,name,kind,bank,digits,opening) VALUES(?1,?2,?3,?2,'',0)",
                params![
                    id,
                    bank,
                    if ["bca", "mandiri"].contains(&hint) {
                        "Bank"
                    } else {
                        "E-Wallet"
                    }
                ],
            )?;
            Ok(Some(id))
        }
        // Multiple wallets at one bank need a user choice, never a silent fallback.
        _ => Ok(None),
    }
}

impl Database {
    pub fn ingest_notification(&self, input: &NotificationInput) -> Result<String> {
        if input.id.is_empty()
            || input.id.len() > 128
            || input.id.chars().any(char::is_control)
            || input.package_name.len() > 200
            || input.title.chars().count() > 1024
            || input.text.chars().count() > 8192
        {
            return Err(AppError::new(
                "VALIDATION",
                "Payload notifikasi tidak valid; antrean tidak dihapus.",
            ));
        }
        let mut c = self.connection()?;
        let tx = c.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing: Option<(String, String, String, i64)> = tx
            .query_row(
                "SELECT package_name,title,body,timestamp FROM notification_events WHERE id=?1",
                [&input.id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .optional()?;
        if let Some(old) = existing {
            if old
                != (
                    input.package_name.clone(),
                    input.title.clone(),
                    input.text.clone(),
                    input.timestamp,
                )
            {
                return Err(AppError::new(
                    "CONFLICT",
                    "ID notifikasi memiliki payload berbeda; antrean dipertahankan.",
                ));
            }
            tx.commit()?;
            return Ok("duplicate".into());
        }
        let mut status = "review";
        let mut reason =
            "Notifikasi tersimpan; format atau arah transaksi belum dapat dipastikan.".to_owned();
        let mut ledger_id = None;
        if notification_parser::is_ignorable(&input.title, &input.text) {
            status = "ignored";
            reason = "Notifikasi nontransaksi atau transaksi belum selesai; bukti tetap tersimpan."
                .into();
        } else if let Some(p) =
            notification_parser::parse_notification(&input.package_name, &input.title, &input.text)
        {
            if let Some(time) = DateTime::<Utc>::from_timestamp_millis(input.timestamp) {
                if let Some(wallet) = wallet_for(&tx, &p.wallet_hint)? {
                    let e = EntryInput {
                        wallet_id: wallet,
                        to_wallet_id: None,
                        kind: p.direction.clone(),
                        amount: p.amount_idr,
                        merchant: p.merchant.clone(),
                        category: if p.direction == "income" {
                            "income".into()
                        } else {
                            parser::category(&p.merchant)
                        },
                        occurred_at: time.to_rfc3339(),
                        note: format!("Notifikasi {} [{}]", input.package_name, input.id),
                    };
                    match insert_entry(&tx, &e, None, None, "Notification", &p.wallet_hint) {
                        Ok(id) => {
                            ledger_id = Some(id);
                            status = "posted";
                            reason = "Tercatat dari notifikasi; bukti asli tersimpan.".into();
                        }
                        Err(e) if e.code == "VALIDATION" => reason = e.message,
                        Err(e) => return Err(e),
                    }
                } else {
                    reason="Ada beberapa dompet penyedia yang sama; pilih dompet melalui pencatatan manual.".into();
                }
            } else {
                reason =
                    "Timestamp notifikasi tidak valid; bukti disimpan untuk pemeriksaan.".into();
            }
        }
        tx.execute("INSERT INTO notification_events(id,package_name,title,body,timestamp,status,reason,ledger_id) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
            params![input.id,input.package_name,input.title,input.text,input.timestamp,status,reason,ledger_id])?;
        audit(&tx, "ingest_notification", &input.id, status)?;
        tx.commit()?;
        Ok(status.into())
    }

    pub fn import_notifications(&self, items: &[NotificationInput]) -> NotificationImportBatch {
        let mut batch = NotificationImportBatch {
            report: NotificationImportReport::default(),
            ack_ids: vec![],
        };
        for input in items {
            match self.ingest_notification(input) {
                Ok(status) => {
                    match status.as_str() {
                        "posted" => batch.report.posted += 1,
                        "review" => batch.report.review += 1,
                        "ignored" => batch.report.ignored += 1,
                        _ => batch.report.duplicates += 1,
                    }
                    batch.ack_ids.push(input.id.clone());
                }
                Err(e) => {
                    batch.report.failed += 1;
                    batch.report.warnings.push(format!(
                        "Notifikasi {} belum tersimpan: {}",
                        input.id, e.code
                    ));
                }
            }
        }
        batch
    }
}
