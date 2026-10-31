use arus_core::{db::Database, models::*, parser};
use rusqlite::params;

fn setup(auto: bool) -> (tempfile::TempDir, Database, Account) {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path().join("arus.db")).unwrap();
    db.save_account(
        "u@example.com",
        "Custom",
        "imap.example.com",
        993,
        "bca",
        auto,
    )
    .unwrap();
    let account = db.accounts().unwrap().remove(0);
    (dir, db, account)
}
fn mail(a: &Account, uid: u32, body: &str) -> MailInput {
    MailInput{account_id:a.id.clone(),mailbox:"INBOX".into(),uid,uid_validity:1,gm_id:None,oversized:false,
        received_at:Some("2026-10-29T10:00:00+07:00".into()),
        raw:format!("From: bank@bca.co.id\r\nMessage-ID: <fix-{uid}>\r\nSubject: Mutasi rekening\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n{body}").into_bytes()}
}
fn entry() -> EntryInput {
    EntryInput {
        wallet_id: "bca".into(),
        to_wallet_id: None,
        kind: "expense".into(),
        amount: 45000,
        merchant: "Toko".into(),
        category: "shop".into(),
        occurred_at: "2026-10-29T10:00:00+07:00".into(),
        note: "".into(),
    }
}
fn notification(id: &str) -> NotificationInput {
    NotificationInput {
        id: id.into(),
        package_name: "id.dana".into(),
        title: "Pembayaran berhasil".into(),
        text: "Kamu berhasil bayar Rp45.000 di Toko".into(),
        timestamp: 1793242800000,
    }
}

#[test]
fn review_conflict_keeps_reference_and_retry_is_idempotent() {
    let (_dir, db, a) = setup(false);
    for uid in [1, 2] {
        db.ingest(
            &mail(&a, uid, "Debit\nNominal: Rp45.000\nNo. Referensi: ABC12345"),
            &a,
        )
        .unwrap();
    }
    let s = db.snapshot().unwrap();
    let first = ReviewInput {
        email_id: s.emails[0].id.clone(),
        action: "post".into(),
        entry: Some(entry()),
    };
    db.resolve_review(&first).unwrap();
    db.resolve_review(&first).unwrap();
    let second = ReviewInput {
        email_id: s.emails[1].id.clone(),
        ..first.clone()
    };
    assert_eq!(
        db.resolve_review(&second).unwrap_err().code,
        "DUPLICATE_REFERENCE"
    );
    let s = db.snapshot().unwrap();
    assert_eq!(s.transactions.len(), 1);
    assert_eq!(s.transactions[0].reference.as_deref(), Some("ABC12345"));
    db.undo_review(&first.email_id).unwrap();
    db.resolve_review(&first).unwrap();
    assert_eq!(db.snapshot().unwrap().transactions.len(), 1);
}

#[test]
fn ambiguous_amounts_and_item_counts_cannot_auto_post() {
    let (_dir, _db, a) = setup(true);
    for body in [
        "Transfer ke Toko\nRp50.000 dan Rp75.000",
        "Transfer ke Toko\nJumlah: 2\nPembayaran Rp100.000",
    ] {
        let e = parser::parse_message(&mail(&a, 1, body), &a).unwrap();
        assert_eq!(e.status, "review");
        assert!(!e.parsed.unwrap().confident);
    }
}

#[test]
fn admin_fee_does_not_replace_the_payment_amount() {
    let (_dir, _db, a) = setup(true);
    let e = parser::parse_message(
        &mail(
            &a,
            1,
            "Transfer ke Toko\nBiaya admin Rp2.500\nDana terkirim Rp100.000",
        ),
        &a,
    )
    .unwrap();
    assert_eq!(e.status, "posted");
    assert_eq!(e.parsed.unwrap().amount, 100000);
}

#[test]
fn reference_labels_are_not_values_and_conflicts_need_review() {
    let (_dir, _db, a) = setup(true);
    for label in [
        "Nomor referensi transaksi",
        "Nomor referensi bank",
        "No. Referensi",
    ] {
        let e = parser::parse_message(
            &mail(
                &a,
                1,
                &format!("Debit\nNominal: Rp45.000\n{label}: ABC12345"),
            ),
            &a,
        )
        .unwrap();
        assert_eq!(e.parsed.unwrap().reference.as_deref(), Some("ABC12345"));
    }
    for refs in [
        "No. Referensi: transaksi",
        "No. Referensi: ABC12345\nReference: DEF67890",
    ] {
        let e = parser::parse_message(
            &mail(&a, 1, &format!("Debit\nNominal: Rp45.000\n{refs}")),
            &a,
        )
        .unwrap();
        let p = e.parsed.unwrap();
        assert!(p.reference.is_none());
        assert!(!p.confident);
        assert_eq!(e.status, "review");
    }
}

#[test]
fn mandiri_qris_merchant_extraction_multiline_heading() {
    let (_dir, _db, a) = setup(true);
    let body = "Pembayaran Berhasil\n\nDetail Transaksi:\nPenerima\n\n#### ALGO BANDUNG 2 - QRIS\nBandung (Kota - ID\n\nNominal Transaksi: Rp 3.500,00\nNo. Referensi: 610223552123";
    let e = parser::parse_message(&mail(&a, 1, body), &a).unwrap();
    let p = e.parsed.unwrap();
    assert_eq!(p.merchant, "ALGO BANDUNG 2 - QRIS");
    assert_eq!(p.amount, 3500);
}

#[test]
fn mandiri_livin_qris_merchant_extraction_with_header_and_hashes() {
    let (_dir, _db, a) = setup(true);
    let body = "Pembayaran QRIS\nPembayaran Berhasil!\n________________________________________\nDetail Transaksi:\nPenerima\n\n##### Laundry Box, DYHKLT\n\nBANDUNG - ID\n\nNominal Transaksi: Rp 18.500,00\nNo. Referensi: 987654321";
    let e = parser::parse_message(&mail(&a, 1, body), &a).unwrap();
    let p = e.parsed.unwrap();
    assert_eq!(p.merchant, "Laundry Box, DYHKLT");
    assert_eq!(p.amount, 18500);
}

#[test]
fn bank_unknown_direction_is_retained_and_missing_amount_is_error() {
    let (_dir, _db, a) = setup(true);
    assert_eq!(
        parser::parse_message(&mail(&a, 1, "Transaksi berhasil\nNominal: Rp45.000"), &a)
            .unwrap()
            .status,
        "review"
    );
    assert_eq!(
        parser::parse_message(&mail(&a, 2, "Transaksi berhasil tanpa nominal"), &a)
            .unwrap()
            .status,
        "error"
    );
}

#[test]
fn invalid_auto_post_is_quarantined_then_next_uid_succeeds() {
    let (_dir, db, a) = setup(true);
    let poison = mail(
        &a,
        20,
        &format!("Debit\nNominal: Rp45.000\nMerchant: {}", "A".repeat(201)),
    );
    assert_eq!(db.ingest(&poison, &a).unwrap(), "error");
    assert_eq!(
        db.ingest(
            &mail(&a, 19, "Debit\nNominal: Rp10.000\nMerchant: Toko"),
            &a
        )
        .unwrap(),
        "posted"
    );
    assert!(db
        .pending_uids(&a.id, "INBOX", 1, vec![19, 20])
        .unwrap()
        .is_empty());
    let s = db.snapshot().unwrap();
    assert_eq!(s.emails.len(), 2);
    assert_eq!(s.transactions.len(), 1);
}

#[test]
fn actual_storage_failure_rolls_back_evidence_ledger_and_receipt() {
    let (_dir, db, a) = setup(true);
    db.connection().unwrap().execute_batch("CREATE TRIGGER fail_ledger BEFORE INSERT ON ledger BEGIN SELECT RAISE(ABORT,'test failure'); END;").unwrap();
    assert!(db.ingest(&mail(&a, 1, "Debit Rp45.000"), &a).is_err());
    assert_eq!(
        db.pending_uids(&a.id, "INBOX", 1, vec![1]).unwrap(),
        vec![1]
    );
    let s = db.snapshot().unwrap();
    assert!(s.emails.is_empty());
    assert!(s.transactions.is_empty());
}

#[test]
fn notification_retry_preserves_source_time_wallet_and_money() {
    let (_dir, db, _a) = setup(false);
    let input = notification("same-id");
    assert_eq!(db.ingest_notification(&input).unwrap(), "posted");
    assert_eq!(db.ingest_notification(&input).unwrap(), "duplicate");
    let s = db.snapshot().unwrap();
    assert_eq!(s.transactions.len(), 1);
    assert_eq!(s.notifications.len(), 1);
    let t = &s.transactions[0];
    assert_eq!(t.wallet_id, "dana");
    assert_eq!(t.source, "Notification");
    assert_eq!(t.category, "other");
    assert_eq!(
        chrono::DateTime::parse_from_rfc3339(&t.occurred_at)
            .unwrap()
            .timestamp_millis(),
        input.timestamp
    );
    assert_eq!(
        s.wallets.iter().find(|w| w.id == "gopay").unwrap().balance,
        0
    );
}

#[test]
fn dana_and_ovo_map_to_their_own_wallets() {
    let (_dir, db, _a) = setup(false);
    for (id, package, wallet) in [
        ("dana-event", "id.dana", "dana"),
        ("ovo-event", "ovo.id", "ovo"),
    ] {
        let mut n = notification(id);
        n.package_name = package.into();
        db.ingest_notification(&n).unwrap();
        assert_eq!(
            db.snapshot()
                .unwrap()
                .transactions
                .iter()
                .find(|t| t.note.contains(id))
                .unwrap()
                .wallet_id,
            wallet
        );
    }
}

#[test]
fn import_ack_list_contains_only_durable_events() {
    let (_dir, db, _a) = setup(false);
    let valid = notification("valid");
    let mut ambiguous = notification("ambiguous");
    ambiguous.text = "Pembayaran berhasil Rp10.000 dan Rp20.000".into();
    let mut ignored = notification("ignored");
    ignored.text = "Promo diskon Rp10.000".into();
    let invalid = notification("");
    let batch = db.import_notifications(&[valid, ambiguous, ignored, invalid]);
    assert_eq!(batch.ack_ids, vec!["valid", "ambiguous", "ignored"]);
    assert_eq!(batch.report.posted, 1);
    assert_eq!(batch.report.review, 1);
    assert_eq!(batch.report.ignored, 1);
    assert_eq!(batch.report.failed, 1);
    assert_eq!(db.snapshot().unwrap().notifications.len(), 3);
}

#[test]
fn notification_receipt_failure_rolls_back_money_and_is_not_acked() {
    let (_dir, db, _a) = setup(false);
    db.connection().unwrap().execute_batch("CREATE TRIGGER fail_receipt BEFORE INSERT ON notification_events BEGIN SELECT RAISE(ABORT,'test failure'); END;").unwrap();
    let batch = db.import_notifications(&[notification("fail")]);
    assert!(batch.ack_ids.is_empty());
    assert_eq!(batch.report.failed, 1);
    let s = db.snapshot().unwrap();
    assert!(s.transactions.is_empty());
    assert!(s.notifications.is_empty());
}

#[test]
fn concurrent_imports_commit_one_ledger_entry() {
    let (_dir, db, _a) = setup(false);
    let threads: Vec<_> = (0..4)
        .map(|_| {
            let db = db.clone();
            std::thread::spawn(move || db.ingest_notification(&notification("concurrent")))
        })
        .collect();
    for t in threads {
        t.join().unwrap().unwrap();
    }
    assert_eq!(db.snapshot().unwrap().transactions.len(), 1);
}

#[test]
fn conflicting_payload_for_same_notification_id_is_not_acked() {
    let (_dir, db, _a) = setup(false);
    let mut n = notification("same");
    db.ingest_notification(&n).unwrap();
    n.text = "Pembayaran berhasil Rp90.000".into();
    let batch = db.import_notifications(&[n]);
    assert!(batch.ack_ids.is_empty());
    assert_eq!(batch.report.failed, 1);
    assert_eq!(db.snapshot().unwrap().transactions[0].amount, 45000);
}

#[test]
fn multiple_wallets_require_review_instead_of_arbitrary_routing() {
    let (_dir, db, _a) = setup(false);
    for id in ["dana-a", "dana-b"] {
        db.connection().unwrap().execute("INSERT INTO wallets(id,name,kind,bank,opening) VALUES(?1,'DANA','E-Wallet','DANA',0)",[id]).unwrap();
    }
    assert_eq!(
        db.ingest_notification(&notification("ambiguous-wallet"))
            .unwrap(),
        "review"
    );
    assert!(db.snapshot().unwrap().transactions.is_empty());
}

#[test]
fn schema_v2_upgrade_preserves_money_reference_and_indexes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("old.db");
    let c = rusqlite::Connection::open(&path).unwrap();
    let schema = include_str!("../src/schema.sql")
        .split("CREATE TABLE IF NOT EXISTS notification_events")
        .next()
        .unwrap()
        .replace("'Email','Manual','Notification'", "'Email','Manual'");
    c.execute_batch(&schema).unwrap();
    c.pragma_update(None, "user_version", 2).unwrap();
    c.execute(
        "INSERT INTO wallets(id,name,kind,bank,opening) VALUES('bca','BCA','Bank','BCA',0)",
        [],
    )
    .unwrap();
    c.execute("INSERT INTO ledger(id,wallet_id,kind,amount,merchant,category,occurred_at,source,reference,bank,note) VALUES('old','bca','income',100000,'Income','income','2026-10-29T10:00:00+07:00','Manual',?1,'BCA','')",params!["OLD12345"]).unwrap();
    drop(c);
    let db = Database::open(&path).unwrap();
    let s = db.snapshot().unwrap();
    assert_eq!(s.transactions.len(), 1);
    assert_eq!(s.transactions[0].id, "old");
    assert_eq!(
        s.wallets.iter().find(|w| w.id == "bca").unwrap().balance,
        100000
    );
    assert_eq!(s.transactions[0].reference.as_deref(), Some("OLD12345"));
    let c = db.connection().unwrap();
    assert_eq!(
        c.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        3
    );
    assert_eq!(
        c.query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |r| r
            .get::<_, u32>(
            0
        ))
        .unwrap(),
        0
    );
    assert!(c
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='index' AND name='ledger_bank_reference'",
            [],
            |r| r.get::<_, u32>(0)
        )
        .is_ok());
    drop(c);
    drop(db);
    assert!(Database::open(&path).is_ok());
}
