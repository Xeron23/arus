use arus_core::{db::Database, models::*, parser};

fn setup() -> (tempfile::TempDir, Database, Account) {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path().join("arus.db")).unwrap();
    let id = db.save_account("user@example.com", "Custom", "imap.example.com", 993, "cash", true).unwrap();
    let account = db.accounts().unwrap().into_iter().find(|a| a.id == id).unwrap();
    (dir, db, account)
}
fn mail(account: &Account, uid: u32, body: &str) -> MailInput {
    MailInput {
        account_id: account.id.clone(), mailbox: "INBOX".into(), uid, uid_validity: 1,
        gm_id: None, oversized: false, received_at: Some("2026-10-27T08:00:00+07:00".into()),
        raw: format!("From: alerts@new-fintech.example\r\nMessage-ID: <patch-{uid}>\r\nSubject: Pemberitahuan transaksi\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n{body}").into_bytes(),
    }
}

#[test]
fn unknown_sender_missing_date_reference_and_category_still_posts() {
    let (_dir, db, account) = setup();
    let m = mail(&account, 1, "Nominal: Rp75.000\nRekening Anda didebet.");
    assert_eq!(db.ingest(&m, &account).unwrap(), "posted");
    let s = db.snapshot().unwrap();
    assert_eq!(s.transactions.len(), 1);
    assert_eq!(s.transactions[0].amount, 75000);
    assert_eq!(s.transactions[0].kind, "expense");
    assert_eq!(s.transactions[0].wallet_id, "cash");
    assert_eq!(s.transactions[0].occurred_at, "2026-10-27T08:00:00+07:00");
    assert_eq!(db.ingest(&m, &account).unwrap(), "email_duplicate");
    assert_eq!(db.snapshot().unwrap().transactions.len(), 1);
}

#[test]
fn credit_currency_without_nominal_label_posts_income() {
    let (_dir, _db, account) = setup();
    let e = parser::parse_message(&mail(&account, 1, "Dana masuk Rp120.000\nKredit berhasil."), &account).unwrap();
    assert_eq!(e.status, "posted");
    assert_eq!(e.parsed.unwrap().kind, "income");
}

#[test]
fn balance_is_not_mistaken_for_transaction_amount() {
    let (_dir, _db, account) = setup();
    let e = parser::parse_message(&mail(&account, 1, "Debit Rp50.000\nSaldo akhir Rp900.000"), &account).unwrap();
    assert_eq!(e.status, "posted");
    assert_eq!(e.parsed.unwrap().amount, 50000);
}

#[test]
fn unclear_direction_pending_promo_and_ambiguous_amount_do_not_post() {
    let (_dir, _db, account) = setup();
    for body in [
        "Nominal Rp10.000", "Debit dan kredit\nNominal Rp10.000",
        "Transfer ke toko pending\nNominal Rp10.000",
        "Promo pembayaran QRIS hingga Rp10.000",
        "Debit Rp10.000 dan Rp20.000",
        "Transfer ke toko gagal\nNominal Rp10.000",
        "Kredit\nSaldo akhir Rp100.000",
    ] {
        let e = parser::parse_message(&mail(&account, 1, body), &account).unwrap();
        assert_ne!(e.status, "posted", "{body}");
    }
}

#[test]
fn auto_post_and_account_activation_remain_explicit() {
    let (_dir, _db, mut account) = setup();
    account.auto_post = false;
    assert_eq!(parser::parse_message(&mail(&account, 1, "Debit Rp10.000"), &account).unwrap().status, "review");
    account.auto_post = true;
    account.enabled = false;
    assert_eq!(parser::parse_message(&mail(&account, 1, "Debit Rp10.000"), &account).unwrap().status, "review");
}

#[test]
fn period_expansion_backfills_lower_uids_and_receipts_include_replays() {
    let (_dir, db, account) = setup();
    let m = mail(&account, 15000, "Debit Rp10.000");
    db.ingest(&m, &account).unwrap();
    assert_eq!(db.cursor(&account.id, "INBOX", 1).unwrap(), 15000);
    let pending = db.pending_uids(&account.id, "INBOX", 1, (1..=15000).collect()).unwrap();
    assert_eq!(pending.len(), 14999);
    assert!(pending.contains(&1));
    assert!(!pending.contains(&15000));
    let mut replay = m.clone();
    replay.uid = 15001;
    assert_eq!(db.ingest(&replay, &account).unwrap(), "email_duplicate");
    assert!(db.pending_uids(&account.id, "INBOX", 1, vec![15000, 15001]).unwrap().is_empty());
    assert_eq!(db.pending_uids(&account.id, "INBOX", 2, vec![15000]).unwrap(), vec![15000]);
}

#[test]
fn range_queries_are_calendar_days_in_jakarta_and_all_has_no_date_filter() {
    let now = chrono::DateTime::parse_from_rfc3339("2026-10-26T18:00:00Z").unwrap().with_timezone(&chrono::Utc);
    assert_eq!(SyncRange::Week.search_query(now), "SINCE 20-Oct-2026");
    assert_eq!(SyncRange::Month.search_query(now), "SINCE 27-Sep-2026");
    assert_eq!(SyncRange::Quarter.search_query(now), "SINCE 29-Jul-2026");
    assert_eq!(SyncRange::All.search_query(now), "ALL");
    assert!(serde_json::from_str::<SyncRange>("\"365\"").is_err());
}

#[test]
fn bank_filter_ignores_unselected_bank() {
    let (_dir, db, account) = setup();
    let mandiri_mail = MailInput {
        account_id: account.id.clone(),
        mailbox: "INBOX".into(),
        uid: 501,
        uid_validity: 1,
        gm_id: None,
        oversized: false,
        received_at: Some("2026-10-27T08:00:00+07:00".into()),
        raw: "From: no-reply@bankmandiri.co.id\r\nMessage-ID: <mandiri-501>\r\nSubject: Pembayaran Berhasil\r\nContent-Type: text/plain; charset=utf-8\r\n\r\nDebit Rp18.500".as_bytes().to_vec(),
    };
    let bca_mail = MailInput {
        account_id: account.id.clone(),
        mailbox: "INBOX".into(),
        uid: 502,
        uid_validity: 1,
        gm_id: None,
        oversized: false,
        received_at: Some("2026-10-27T08:00:00+07:00".into()),
        raw: "From: alert@bca.co.id\r\nMessage-ID: <bca-502>\r\nSubject: Pembayaran Berhasil\r\nContent-Type: text/plain; charset=utf-8\r\n\r\nDebit Rp25.000".as_bytes().to_vec(),
    };
    let filter = vec!["Mandiri".to_string()];
    assert_eq!(db.ingest_with_banks(&mandiri_mail, &account, Some(&filter)).unwrap(), "posted");
    assert_eq!(db.ingest_with_banks(&bca_mail, &account, Some(&filter)).unwrap(), "ignored");
    let s = db.snapshot().unwrap();
    let bca_rec = s.emails.into_iter().find(|e| e.uid == 502).unwrap();
    assert_eq!(bca_rec.status, "ignored");
    assert!(bca_rec.reason.contains("tidak dipilih untuk disinkronkan"));
}

#[test]
fn test_adaptive_learning_rules_in_db() {
    let (_dir, db, mut account) = setup();
    account.auto_post = false; // Need review status!
    let mail1 = MailInput {
        account_id: account.id.clone(),
        mailbox: "INBOX".into(),
        uid: 991,
        uid_validity: 1,
        gm_id: None,
        oversized: false,
        received_at: Some("2026-10-27T08:00:00+07:00".into()),
        raw: "From: no-reply@bankmandiri.co.id\r\nMessage-ID: <mandiri-991>\r\nSubject: Pembayaran QRIS\r\nContent-Type: text/plain; charset=utf-8\r\n\r\nDebit Rp50.000\nMerchant: PT Antah Berantah".as_bytes().to_vec(),
    };
    db.ingest(&mail1, &account).unwrap();
    let s = db.snapshot().unwrap();
    let email1 = s.emails.into_iter().find(|e| e.uid == 991).unwrap();
    assert_eq!(email1.parsed.as_ref().unwrap().category, "other");

    // User corrects it to "food"
    let review_input = ReviewInput {
        email_id: email1.id.clone(),
        action: "post".into(),
        entry: Some(EntryInput {
            wallet_id: "cash".into(),
            to_wallet_id: None,
            kind: "expense".into(),
            amount: 50000,
            merchant: "PT Antah Berantah".into(),
            category: "food".into(),
            occurred_at: "2026-10-27T08:00:00+07:00".into(),
            note: "".into(),
        }),
    };
    db.resolve_review(&review_input).unwrap();

    // Now a new email from the same merchant comes in
    let mail2 = MailInput {
        account_id: account.id.clone(),
        mailbox: "INBOX".into(),
        uid: 992,
        uid_validity: 1,
        gm_id: None,
        oversized: false,
        received_at: Some("2026-10-27T08:00:00+07:00".into()),
        raw: "From: no-reply@bankmandiri.co.id\r\nMessage-ID: <mandiri-992>\r\nSubject: Pembayaran QRIS\r\nContent-Type: text/plain; charset=utf-8\r\n\r\nDebit Rp75.000\nMerchant: PT Antah Berantah Cabang 2".as_bytes().to_vec(),
    };
    db.ingest(&mail2, &account).unwrap();
    let s2 = db.snapshot().unwrap();
    let email2 = s2.emails.into_iter().find(|e| e.uid == 992).unwrap();
    // It should have learned and automatically classified as "food"!
    assert_eq!(email2.parsed.unwrap().category, "food");
}
