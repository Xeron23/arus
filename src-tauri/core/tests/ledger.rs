use arus_core::{db::Database, imap, models::*, parser};
use tempfile::TempDir;
fn setup() -> (TempDir, Database, Account) {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path().join("arus.db")).unwrap();
    let id = db
        .save_account(
            "demo@gmail.com",
            "Gmail",
            "imap.gmail.com",
            993,
            "bca",
            true,
        )
        .unwrap();
    let a = db
        .accounts()
        .unwrap()
        .into_iter()
        .find(|a| a.id == id)
        .unwrap();
    (dir, db, a)
}
fn entry(kind: &str, amount: i64) -> EntryInput {
    EntryInput {
        wallet_id: "bca".into(),
        to_wallet_id: if kind == "transfer" {
            Some("gopay".into())
        } else {
            None
        },
        kind: kind.into(),
        amount,
        merchant: "Indomaret".into(),
        category: "shop".into(),
        occurred_at: "2026-10-27T10:42:00+07:00".into(),
        note: String::new(),
    }
}
fn mail(a: &Account, uid: u32, msg: &str, reference: &str, merchant: &str) -> MailInput {
    MailInput{account_id:a.id.clone(),mailbox:"[Gmail]/All Mail".into(),uid,uid_validity:1,gm_id:None,oversized:false,received_at:Some("2026-10-27T10:43:00+07:00".into()),raw:format!("From: Notifikasi <notifikasi@bca.co.id>\r\nTo: demo@gmail.com\r\nSubject: Pembayaran QRIS\r\nMessage-ID: <{msg}>\r\nDate: Tue, 27 Oct 2026 10:43:00 +0700\r\nContent-Type: text/plain; charset=utf-8\r\n\r\nBERHASIL\nPEMBAYARAN QRIS: {merchant}\nNominal: Rp45.000,00\nTanggal: 2026-10-27 10:42 WIB\nNo. Referensi: {reference}").into_bytes()}
}
fn total(db: &Database) -> i64 {
    db.snapshot()
        .unwrap()
        .wallets
        .iter()
        .map(|w| w.balance)
        .sum()
}
#[test]
fn integers_not_float() {
    assert_eq!(parser::rupiah("Rp45.000,00"), Some(45000));
    assert_eq!(parser::rupiah("Rp45.000,25"), None);
    assert_eq!(parser::rupiah("1.2.3"), None);
    assert_eq!(parser::rupiah("100000000000001"), None);
    assert_eq!(parser::rupiah("Rp1.250.000"), Some(1250000));
}
#[test]
fn transfer_moves_balance_without_creating_money() {
    let (_d, db, _) = setup();
    db.add_manual(&entry("income", 1000000)).unwrap();
    let before = total(&db);
    db.add_manual(&entry("transfer", 150000)).unwrap();
    let s = db.snapshot().unwrap();
    assert_eq!(total(&db), before);
    assert_eq!(
        s.wallets.iter().find(|w| w.id == "gopay").unwrap().balance,
        150000
    );
    assert_eq!(
        s.wallets.iter().find(|w| w.id == "bca").unwrap().balance,
        850000
    );
}
#[test]
fn validation_and_delete_undo() {
    let (_d, db, _) = setup();
    assert!(db.add_manual(&entry("expense", -1)).is_err());
    let mut bad = entry("transfer", 10);
    bad.to_wallet_id = Some("bca".into());
    assert!(db.add_manual(&bad).is_err());
    let id = db.add_manual(&entry("expense", 45000)).unwrap();
    assert_eq!(total(&db), -45000);
    db.delete_entry(&id, false).unwrap();
    assert_eq!(total(&db), 0);
    db.delete_entry(&id, true).unwrap();
    assert_eq!(total(&db), -45000);
}
#[test]
fn two_layers_and_uidvalidity_are_idempotent() {
    let (_d, db, a) = setup();
    let m = mail(&a, 1, "msg1", "BCA-001", "Indomaret");
    assert_eq!(db.ingest(&m, &a).unwrap(), "posted");
    let mut same = mail(&a, 2, "msg1", "BCA-001", "Indomaret");
    assert_eq!(db.ingest(&same, &a).unwrap(), "email_duplicate");
    let summary = mail(&a, 3, "msg2", "BCA-001", "Indomaret");
    assert_eq!(db.ingest(&summary, &a).unwrap(), "duplicate");
    assert_eq!(total(&db), -45000);
    assert_eq!(db.cursor(&a.id, &m.mailbox, 1).unwrap(), 3);
    assert_eq!(db.cursor(&a.id, &m.mailbox, 2).unwrap(), 0);
    same.uid = 10;
    same.uid_validity = 2;
    assert_eq!(db.ingest(&same, &a).unwrap(), "email_duplicate");
    assert_eq!(total(&db), -45000);
    assert_eq!(db.cursor(&a.id, &m.mailbox, 2).unwrap(), 10);
}
#[test]
fn review_balance_waits_for_approval_then_undo() {
    let (_d, db, mut a) = setup();
    a.auto_post = false;
    assert_eq!(
        db.ingest(&mail(&a, 1, "review1", "BCA-REVIEW", "Indomaret"), &a)
            .unwrap(),
        "review"
    );
    assert_eq!(total(&db), 0);
    let id = db.snapshot().unwrap().emails[0].id.clone();
    let input = ReviewInput {
        email_id: id.clone(),
        action: "post".into(),
        entry: Some(entry("expense", 45000)),
    };
    db.resolve_review(&input).unwrap();
    assert_eq!(total(&db), -45000);
    db.resolve_review(&input).unwrap(); // A retried confirmation is idempotent.
    db.undo_review(&id).unwrap();
    assert_eq!(total(&db), 0);
    db.resolve_review(&input).unwrap();
    assert_eq!(total(&db), -45000);
}
#[test]
fn invalid_extracted_wallet_is_saved_for_manual_review() {
    let (_d, db, mut a) = setup();
    a.wallet_id = "missing".into();
    let m = mail(&a, 7, "broken", "BCA-BROKEN-001", "Indomaret");
    db.cursor(&a.id, &m.mailbox, 1).unwrap();
    assert_eq!(db.ingest(&m, &a).unwrap(), "error");
    assert_eq!(db.cursor(&a.id, &m.mailbox, 1).unwrap(), 7);
    assert_eq!(db.snapshot().unwrap().emails.len(), 1);
    assert_eq!(total(&db), 0);
}
#[test]
fn observed_balance_does_not_silently_modify_ledger() {
    let (_d, db, _) = setup();
    db.observed("bca", 100000, false).unwrap();
    assert_eq!(total(&db), 0);
    db.observed("bca", 100000, true).unwrap();
    assert_eq!(total(&db), 100000);
    assert_eq!(db.snapshot().unwrap().transactions.len(), 1);
}
#[test]
fn malformed_mime_is_audited_without_execution() {
    let (_d, db, a) = setup();
    let mut m = mail(&a, 1, "html1", "BCA-HTML", "Indomaret");
    m.raw=b"From: notifications@bca.co.id\r\nMessage-ID: <html1>\r\nSubject: Mutasi berhasil\r\nContent-Type: text/html\r\n\r\n<script>throw 'never executed'</script><img src=\"https://tracker.invalid/pixel\"><p>Transaksi berhasil; nominal pada lampiran.</p>".to_vec();
    db.ingest(&m, &a).unwrap();
    assert_eq!(total(&db), 0);
    assert_eq!(db.snapshot().unwrap().emails[0].status, "error");
}
#[test]
fn google_password_validated_and_custom_requires_tls_host() {
    let invalid = ConnectionInput {
        email: "a@gmail.com".into(),
        password: "short".into(),
        provider: "Auto".into(),
        host: None,
        port: None,
    };
    assert!(imap::endpoint(invalid).is_err());
    let valid = ConnectionInput {
        email: "a@gmail.com".into(),
        password: "abcd efgh ijkl mnop".into(),
        provider: "Auto".into(),
        host: None,
        port: None,
    };
    let ep = imap::endpoint(valid).unwrap();
    assert_eq!(ep.host, "imap.gmail.com");
    assert_eq!(ep.port, 993);
    assert_eq!(ep.password.len(), 16);
}
#[test]
fn no_password_column_or_secret_in_database() {
    let (_d, db, _) = setup();
    let c = db.connection().unwrap();
    let mut q = c.prepare("PRAGMA table_info(accounts)").unwrap();
    let names = q
        .query_map([], |r| r.get::<_, String>(1))
        .unwrap()
        .collect::<std::result::Result<Vec<_>, _>>()
        .unwrap();
    assert!(!names
        .iter()
        .any(|n| n.contains("password") || n.contains("secret")));
}
