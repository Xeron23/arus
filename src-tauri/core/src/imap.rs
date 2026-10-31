//! Direct device -> provider TLS. No HTTP proxy/backend. Never changes IMAP flags.
use crate::{db::Database, models::*, AppError, Result};
use async_imap::{
    types::{Fetch, NameAttribute},
    Client, Session,
};
use futures_util::TryStreamExt;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{net::TcpStream, time::timeout};
use tokio_rustls::{
    client::TlsStream,
    rustls::{self, pki_types::ServerName, ClientConfig, RootCertStore},
    TlsConnector,
};
use zeroize::Zeroizing;
type ImapSession = Session<TlsStream<TcpStream>>;
pub struct Endpoint {
    pub email: String,
    pub password: Zeroizing<String>,
    pub provider: String,
    pub host: String,
    pub port: u16,
}
pub fn endpoint(input: ConnectionInput) -> Result<Endpoint> {
    let email = input.email.trim().to_owned();
    let raw_password = Zeroizing::new(input.password);
    let password = Zeroizing::new(
        raw_password
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>(),
    );
    if email.len() > 254
        || !email.contains('@')
        || email.chars().any(char::is_control)
        || password.is_empty()
    {
        return Err(AppError::new(
            "VALIDATION",
            "Alamat email dan App Password wajib diisi.",
        ));
    }
    let provider = if input.provider == "Auto" {
        let domain = email
            .split('@')
            .next_back()
            .unwrap_or("")
            .to_ascii_lowercase();
        match domain.as_str() {
            "gmail.com" | "googlemail.com" => "Gmail",
            "yahoo.com" | "yahoo.co.id" => "Yahoo",
            "outlook.com" | "hotmail.com" | "live.com" => "Outlook",
            _ => "Custom",
        }
        .to_owned()
    } else {
        input.provider
    };
    let (host, port) = match provider.as_str() {
        "Gmail" => ("imap.gmail.com".into(), 993),
        "Yahoo" => ("imap.mail.yahoo.com".into(), 993),
        "Outlook" => ("outlook.office365.com".into(), 993),
        "Custom" => (
            input.host.unwrap_or_default().trim().to_owned(),
            input.port.unwrap_or(993),
        ),
        _ => return Err(AppError::new("VALIDATION", "Provider tidak dikenal.")),
    };
    if provider == "Gmail"
        && (password.len() != 16 || !password.bytes().all(|c| c.is_ascii_alphabetic()))
    {
        return Err(AppError::new(
            "VALIDATION",
            "Google App Password harus 16 huruf (spasi pemisah boleh diabaikan).",
        ));
    }
    if host.is_empty()
        || host.len() > 253
        || !host
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'.' || c == b'-')
        || port == 0
    {
        return Err(AppError::new(
            "VALIDATION",
            "Host atau port IMAP tidak valid.",
        ));
    }
    Ok(Endpoint {
        email,
        password,
        provider,
        host,
        port,
    })
}
async fn connect(ep: &Endpoint) -> Result<ImapSession> {
    let tcp = timeout(
        Duration::from_secs(12),
        TcpStream::connect((ep.host.as_str(), ep.port)),
    )
    .await
    .map_err(|_| {
        AppError::new(
            "TCP_TIMEOUT",
            "Koneksi TCP habis waktu. Periksa jaringan dan port IMAP.",
        )
    })?
    .map_err(|_| {
        AppError::new(
            "TCP_CONNECT",
            "Server IMAP tidak terjangkau. Periksa DNS, host, dan jaringan.",
        )
    })?;
    let _ = tcp.set_nodelay(true);
    let roots = RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let config =
        ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions()
            .map_err(|_| AppError::new("TLS_CONFIG", "Konfigurasi TLS gagal."))?
            .with_root_certificates(roots)
            .with_no_client_auth();
    let name = ServerName::try_from(ep.host.clone())
        .map_err(|_| AppError::new("TLS_HOST", "Nama host TLS tidak valid."))?;
    let tls=timeout(Duration::from_secs(12),TlsConnector::from(Arc::new(config)).connect(name,tcp)).await.map_err(|_|AppError::new("TLS_TIMEOUT","Handshake TLS habis waktu."))?.map_err(|_|AppError::new("TLS_VERIFY","Validasi sertifikat TLS gagal. Periksa tanggal HP dan nama host. Sertifikat tidak dilewati."))?;
    let mut client = Client::new(tls);
    let greeting = timeout(Duration::from_secs(12), client.read_response())
        .await
        .map_err(|_| AppError::new("IMAP_TIMEOUT", "Server tidak memberikan greeting IMAP."))?;
    if !matches!(greeting, Ok(Some(_))) {
        return Err(AppError::new(
            "IMAP_GREETING",
            "Greeting IMAP tidak valid atau koneksi ditutup.",
        ));
    }
    timeout(Duration::from_secs(18),client.login(&ep.email,ep.password.as_str())).await.map_err(|_|AppError::new("AUTH_TIMEOUT","Autentikasi IMAP habis waktu."))?.map_err(|_|AppError::new("AUTHENTICATION_FAILED","Login ditolak. Gunakan App Password, bukan sandi utama. Provider tertentu memerlukan OAuth2."))
}
async fn folders(s: &mut ImapSession) -> Result<Vec<String>> {
    let names = timeout(Duration::from_secs(20), async {
        s.list(Some(""), Some("*"))
            .await?
            .try_collect::<Vec<_>>()
            .await
    })
    .await
    .map_err(|_| AppError::new("LIST_TIMEOUT", "Daftar folder IMAP habis waktu."))?
    .map_err(|_| AppError::new("IMAP_LIST", "Daftar folder IMAP tidak dapat dibaca."))?;
    let mut archive = vec![];
    for n in names {
        let attrs = n.attributes();
        if attrs.contains(&NameAttribute::NoSelect) {
            continue;
        }
        if attrs.contains(&NameAttribute::All) {
            return Ok(vec![n.name().to_owned()]);
        }
        let name = n.name().to_owned();
        if attrs.contains(&NameAttribute::Archive)
            || name.to_lowercase().contains("archive")
            || name.to_lowercase().contains("arsip")
        {
            archive.push(name)
        }
    }
    let mut result = vec!["INBOX".into()];
    result.extend(archive);
    result.sort();
    result.dedup();
    Ok(result)
}
pub async fn test(input: ConnectionInput) -> Result<ConnectionReport> {
    let ep = endpoint(input)?;
    test_endpoint(&ep).await
}
pub async fn test_endpoint(ep: &Endpoint) -> Result<ConnectionReport> {
    let start = Instant::now();
    let mut s = connect(ep).await?;
    // Fast examine of INBOX only for rapid TLS connection verification (1-2s)
    timeout(Duration::from_secs(10), s.examine("INBOX"))
        .await
        .map_err(|_| AppError::new("EXAMINE_TIMEOUT", "Membaca metadata folder habis waktu."))?
        .map_err(|_| AppError::new("IMAP_EXAMINE", "Folder tidak dapat dibuka read-only."))?;
    let _ = timeout(Duration::from_secs(3), s.logout()).await;
    Ok(ConnectionReport {
        host: ep.host.clone(),
        port: ep.port,
        provider: ep.provider.clone(),
        folder: "INBOX".into(),
        elapsed_ms: start.elapsed().as_millis() as u64,
        tls: true,
    })
}
async fn fetch(s: &mut ImapSession, uid: u32, query: &str) -> Result<Option<Fetch>> {
    let items = timeout(Duration::from_secs(30), async {
        s.uid_fetch(uid.to_string(), query)
            .await?
            .try_collect::<Vec<_>>()
            .await
    })
    .await
    .map_err(|_| {
        AppError::new(
            "FETCH_TIMEOUT",
            "Pengunduhan email habis waktu. Kursor email ini belum dimajukan.",
        )
    })?
    .map_err(|_| {
        AppError::new(
            "IMAP_FETCH",
            "Email gagal diunduh. Tekan Sync lagi untuk melanjutkan.",
        )
    })?;
    Ok(items.into_iter().find(|f| f.uid == Some(uid)))
}
pub async fn sync<F: Fn(SyncProgress) + Send + Sync>(
    db: &Database,
    account: &Account,
    password: Zeroizing<String>,
    progress: &F,
    range: SyncRange,
    banks: Option<Vec<String>>,
) -> Result<SyncReport> {
    let ep = endpoint(ConnectionInput {
        email: account.email.clone(),
        password: password.to_string(),
        provider: account.provider.clone(),
        host: Some(account.host.clone()),
        port: Some(account.port),
    })?;
    let emit = |phase: &str, msg: String, current: u32, total: u32| {
        progress(SyncProgress {
            phase: phase.into(),
            message: msg,
            account_id: account.id.clone(),
            current,
            total,
        })
    };
    emit(
        "connect",
        "Menghubungkan TCP dan TLS langsung ke provider…".into(),
        0,
        0,
    );
    let mut s = connect(&ep).await?;
    let folders = folders(&mut s).await?;
    let caps = timeout(Duration::from_secs(15), s.capabilities())
        .await
        .map_err(|_| AppError::new("IMAP_TIMEOUT", "Pemeriksaan kapabilitas habis waktu."))?
        .map_err(|_| AppError::new("IMAP_CAPABILITY", "Kapabilitas server tidak dapat dibaca."))?;
    let gmail = caps.has_str("X-GM-EXT-1");
    let mut report = SyncReport::default();
    let search_query = range.search_query(chrono::Utc::now());
    let mut in_flight_remaining = 0u32;
    let processing: Result<()> = async {
    for folder in &folders {
        emit(
            "search",
            format!("Memeriksa email pada periode pilihan di {folder}"),
            0,
            0,
        );
        let mailbox = timeout(Duration::from_secs(20), s.examine(folder))
            .await
            .map_err(|_| AppError::new("EXAMINE_TIMEOUT", "Folder IMAP habis waktu."))?
            .map_err(|_| AppError::new("IMAP_EXAMINE", "Folder tidak dapat dibaca read-only."))?;
        let validity = mailbox.uid_validity.ok_or_else(|| {
            AppError::new(
                "UIDVALIDITY_MISSING",
                "Server tidak mengembalikan UIDVALIDITY; sync dihentikan untuk melindungi kursor.",
            )
        })?;
        db.cursor(&account.id, folder, validity)?;
        // Do not apply a global high-water UID: widening the period needs backfill.
        let found = timeout(
            Duration::from_secs(30),
            s.uid_search(&search_query),
        )
        .await
        .map_err(|_| AppError::new("SEARCH_TIMEOUT", "Pencarian UID habis waktu."))?
        .map_err(|_| AppError::new("IMAP_SEARCH", "Pencarian UID gagal."))?;
        let mut uids = db.pending_uids(&account.id, folder, validity, found.into_iter().collect())?;
        // Sort descending: process NEWEST emails first so recent transactions appear immediately
        uids.sort_unstable_by(|a, b| b.cmp(a));
        let total = uids.len() as u32;
        let available = 200u32.saturating_sub(report.fetched) as usize;
        report.remaining += uids.len().saturating_sub(available) as u32;
        uids.truncate(available);
        in_flight_remaining = uids.len() as u32;
        for (i, uid) in uids.into_iter().enumerate() {
            emit(
                "fetch",
                format!("Membaca email {} dari {total}", i + 1),
                i as u32,
                total,
            );
            let meta = fetch(&mut s, uid, "(UID RFC822.SIZE)").await?;
            if meta.is_none() {
                // Expunged between SEARCH and FETCH: no email evidence to ingest.
                let c = db.connection()?;
                c.execute("INSERT OR IGNORE INTO sync_receipts(account_id,mailbox,uid_validity,uid) VALUES(?1,?2,?3,?4)", rusqlite::params![account.id,folder,validity,uid])?;
                c.execute("UPDATE cursors SET last_uid=MAX(last_uid,?1) WHERE account_id=?2 AND mailbox=?3",rusqlite::params![uid,account.id,folder])?;
                in_flight_remaining = in_flight_remaining.saturating_sub(1);
                continue;
            }
            let size = meta.as_ref().and_then(|m| m.size).unwrap_or(u32::MAX);
            let oversized = size > 2 * 1024 * 1024;
            let body = if oversized {
                "BODY.PEEK[]<0.16384>"
            } else {
                "BODY.PEEK[]"
            };
            let query = if gmail {
                format!("(UID INTERNALDATE X-GM-MSGID {body})")
            } else {
                format!("(UID INTERNALDATE {body})")
            };
            let f = fetch(&mut s, uid, &query).await?.ok_or_else(|| {
                AppError::new("FETCH_GONE", "Email berubah saat dibaca. Ulangi sync.")
            })?;
            let raw = f
                .body()
                .ok_or_else(|| {
                    AppError::new(
                        "EMPTY_BODY",
                        "Server tidak mengembalikan body; kursor belum dimajukan.",
                    )
                })?
                .to_vec();
            let mail = MailInput {
                account_id: account.id.clone(),
                mailbox: folder.clone(),
                uid,
                uid_validity: validity,
                gm_id: f.gmail_msg_id().map(|v| v.to_string()),
                raw,
                oversized,
                received_at: f.internal_date().map(|t| t.to_rfc3339()),
            };
            let disposition = db.ingest_with_banks(&mail, account, banks.as_deref())?;
            in_flight_remaining = in_flight_remaining.saturating_sub(1);
            report.fetched += 1;
            match disposition.as_str() {
                "posted" => report.posted += 1,
                "review" => report.review += 1,
                "ignored" => report.ignored += 1,
                "error" => report.errors += 1,
                "duplicate" => report.ledger_duplicates += 1,
                "email_duplicate" => report.email_duplicates += 1,
                _ => {}
            }
            emit(
                "ingest",
                format!("Email {uid} tersimpan · {disposition}"),
                i as u32 + 1,
                total,
            );
        }
        db.mark_synced(&account.id, folder)?;
    }
    Ok(())
    }.await;
    let _ = timeout(Duration::from_secs(3), s.logout()).await;
    if let Err(e) = processing {
        // Network/storage failures stop this connection, but uncommitted UIDs
        // remain retryable. Domain validation errors are persisted by ingest.
        report.remaining += in_flight_remaining;
        report.warnings.push(format!("{}: {}", e.code, e.message));
    }
    emit(
        "done",
        format!(
            "{} tercatat · {} perlu review",
            report.posted, report.review
        ),
        report.fetched,
        report.fetched,
    );
    Ok(report)
}
