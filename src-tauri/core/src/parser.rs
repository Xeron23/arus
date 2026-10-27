//! Local, text-only parser. Explicit amount + direction permit opt-in auto-post.
use crate::{models::*, Result};
use chrono::{DateTime, FixedOffset, NaiveDateTime, TimeZone, Utc};
use mailparse::{MailHeaderMap, ParsedMail};
use regex::Regex;
use sha2::{Digest, Sha256};
use uuid::Uuid;
fn capture(pattern: &str, text: &str) -> Option<String> {
    Regex::new(pattern)
        .ok()?
        .captures(text)?
        .get(1)
        .map(|m| m.as_str().trim().to_owned())
}
/// Strip transport labels only at the beginning; preserve the actual merchant spelling.
pub fn clean_merchant(value: &str) -> String {
    let prefix = Regex::new(r"(?i)^(?:(?:transaksi\s+)?pembayaran\s+qris(?:\s+(?:ke|kepada))?|transfer\s+(?:ke|dari)|(?:nama\s+)?merchant|(?:nama\s+)?penerima|pengirim|tujuan)(?:\s*[:\-–]\s*|\s+|$)").unwrap();
    for line in value.lines() {
        let mut name = line.split_whitespace().collect::<Vec<_>>().join(" ");
        for _ in 0..8 {
            let next = prefix.replace(&name, "").trim().to_owned();
            if next == name {
                break;
            }
            name = next;
        }
        if !name.is_empty() {
            return name;
        }
    }
    String::new()
}

pub fn rupiah(text: &str) -> Option<i64> {
    let v = text
        .trim()
        .trim_start_matches("Rp")
        .trim_start_matches("rp")
        .trim()
        .trim_start_matches('.')
        .trim();
    let parts: Vec<_> = v.split(',').collect();
    if parts.len() > 2 || parts.len() == 2 && !parts[1].chars().all(|c| c == '0') {
        return None;
    }
    if !Regex::new(r"^(?:\d+|\d{1,3}(?:\.\d{3})+)$")
        .ok()?
        .is_match(parts[0])
    {
        return None;
    }
    let n = parts[0].replace('.', "").parse::<i64>().ok()?;
    if n > 0 && n <= 100_000_000_000_000 {
        Some(n)
    } else {
        None
    }
}
pub fn category(name: &str) -> String {
    let n = name.to_lowercase();
    if ["indomaret", "alfamart", "superindo", "tokopedia", "shopee"]
        .iter()
        .any(|s| n.contains(s))
    {
        "shop"
    } else if ["pertamina", "spbu", "grab", "gojek", "mrt"]
        .iter()
        .any(|s| n.contains(s))
    {
        "transport"
    } else if ["warteg", "kopi", "coffee", "bakmi", "resto", "makan"]
        .iter()
        .any(|s| n.contains(s))
    {
        "food"
    } else if ["pln", "sewa", "kost", "internet"]
        .iter()
        .any(|s| n.contains(s))
    {
        "home"
    } else {
        "other"
    }
    .into()
}
fn plain(part: &ParsedMail<'_>) -> Option<String> {
    if part.ctype.mimetype == "text/plain" {
        return part.get_body().ok();
    }
    for sub in &part.subparts {
        if let Some(s) = plain(sub) {
            return Some(s);
        }
    }
    None
}
fn html(part: &ParsedMail<'_>) -> Option<String> {
    if part.ctype.mimetype == "text/html" {
        return part
            .get_body()
            .ok()
            .map(|v| html2text::from_read(v.as_bytes(), 100));
    }
    for sub in &part.subparts {
        if let Some(s) = html(sub) {
            return Some(s);
        }
    }
    None
}
fn sender_bank(sender: &str) -> Option<&'static str> {
    let s = sender.to_lowercase();
    let address = s.rsplit('<').next().unwrap_or(&s).trim_matches(['>', ' ']);
    let domain = address.split('@').nth(1)?;
    [
        ("bca.co.id", "BCA"),
        ("bankmandiri.co.id", "Mandiri"),
        ("bri.co.id", "BRI"),
        ("bni.co.id", "BNI"),
        ("jenius.com", "Jenius"),
        ("jago.com", "Bank Jago"),
        ("seabank.co.id", "Seabank"),
        ("gopay.co.id", "GoPay"),
        ("gojek.com", "GoPay"),
        ("dana.id", "DANA"),
        ("ovo.id", "OVO"),
        ("shopeepay.co.id", "ShopeePay"),
    ]
    .into_iter()
    .find(|(d, _)| domain == *d || domain.ends_with(&format!(".{d}")))
    .map(|(_, b)| b)
}
fn actual_time(text: &str) -> Option<String> {
    let raw = capture(r"(?i)(20\d{2}-\d{2}-\d{2}[ T]\d{2}:\d{2}(?::\d{2})?)", text);
    let alt = capture(r"(\d{2}/\d{2}/20\d{2}\s+\d{2}:\d{2}(?::\d{2})?)", text);
    for (candidate, formats) in [
        (raw, ["%Y-%m-%d %H:%M:%S", "%Y-%m-%d %H:%M"]),
        (alt, ["%d/%m/%Y %H:%M:%S", "%d/%m/%Y %H:%M"]),
    ] {
        if let Some(v) = candidate {
            let v = v.replace('T', " ");
            for f in formats {
                if let Ok(t) = NaiveDateTime::parse_from_str(&v, f) {
                    return FixedOffset::east_opt(7 * 3600)?
                        .from_local_datetime(&t)
                        .single()
                        .map(|t| t.to_rfc3339());
                }
            }
        }
    }
    None
}
fn transaction_amount(text: &str) -> Option<i64> {
    // 1. First priority: Look for labelled nominal patterns (Nominal, Jumlah, Total, Amount)
    let label = Regex::new(r"(?im)\b(?:nominal|jumlah|total(?: pembayaran)?|amount)\s*:?\s*((?:Rp\.?\s*|IDR\s*)?[0-9][0-9.,]*)").unwrap();
    let excluded = Regex::new(r"(?i)\b(?:saldo|balance|limit)\b").unwrap();
    
    for caps in label.captures_iter(text) {
        let raw = caps[1].trim().trim_start_matches("IDR").trim_start_matches("idr").trim();
        if let Some(n) = rupiah(raw.trim_end_matches('.')) {
            return Some(n);
        }
    }

    // 2. Second priority: Any line with Rp / IDR that does NOT mention saldo/balance
    let currency = Regex::new(r"(?i)\b(?:Rp\.?|IDR)\s*([0-9][0-9.,]*)").unwrap();
    let mut candidates = Vec::new();
    for line in text.lines().filter(|line| !excluded.is_match(line)) {
        for caps in currency.captures_iter(line) {
            if let Some(n) = rupiah(caps[1].trim_end_matches('.')) {
                candidates.push(n);
            }
        }
    }
    // Return the first candidate found
    candidates.into_iter().next()
}

pub fn parse_message(input: &MailInput, account: &Account) -> Result<EmailRecord> {
    let decoded = mailparse::parse_mail(&input.raw).ok();
    let header = |name: &str| {
        decoded
            .as_ref()
            .and_then(|m| m.headers.get_first_value(name))
            .unwrap_or_default()
    };
    let sender = header("From");
    let subject = header("Subject");
    let date = header("Date");
    let received = mailparse::dateparse(&date)
        .ok()
        .and_then(|t| DateTime::<Utc>::from_timestamp(t, 0))
        .unwrap_or_else(Utc::now)
        .to_rfc3339();
    let message_id = {
        let s = header("Message-ID");
        if s.trim().is_empty() {
            format!("sha256:{:x}", Sha256::digest(&input.raw))
        } else {
            s.trim().to_owned()
        }
    };
    let text = decoded
        .as_ref()
        .and_then(plain)
        .or_else(|| decoded.as_ref().and_then(html))
        .unwrap_or_else(|| {
            String::from_utf8_lossy(&input.raw)
                .split_once("\r\n\r\n")
                .map(|(_, b)| b.to_owned())
                .unwrap_or_default()
        });
    let snippet = text
        .chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .take(64_000)
        .collect::<String>();
    let mut e = EmailRecord {
        id: Uuid::new_v4().to_string(),
        account_id: input.account_id.clone(),
        mailbox: input.mailbox.clone(),
        uid: input.uid,
        uid_validity: input.uid_validity,
        message_id,
        gm_id: input.gm_id.clone(),
        sender,
        subject,
        received_at: input.received_at.clone().unwrap_or(received),
        raw_snippet: snippet.clone(),
        status: "review".into(),
        reason: String::new(),
        parsed: None,
    };
    if input.oversized {
        e.status = "error".into();
        e.reason =
            "Email melebihi batas 2 MiB; lampiran tidak diunduh. Periksa di aplikasi email.".into();
        return Ok(e);
    }
    let bank_name = sender_bank(&e.sender).map(str::to_owned).unwrap_or_else(|| {
        // Keep unknown senders separate for reference deduplication; no whitelist.
        capture(r"<([^>]+)>", &e.sender)
            .unwrap_or_else(|| e.sender.trim().to_lowercase())
    });
    let bank = bank_name.as_str();
    let combined = format!("{}\n{}", e.subject, snippet);
    let incoming = Regex::new(r"(?i)\b(?:kredit|credit|dikreditkan|credited|dana masuk|uang masuk|transfer masuk|transfer dari|menerima transfer|pembayaran diterima)\b").unwrap().is_match(&combined);
    let outgoing = Regex::new(r"(?i)\b(?:debit|debet|didebit|didebet|didebitkan|didebetkan|debited|dana keluar|transfer keluar|transfer ke|pembayaran qris|pembayaran berhasil|berhasil membayar|top\s*up)\b").unwrap().is_match(&combined);
    let blocked = Regex::new(r"(?i)\b(?:gagal|failed|dibatalkan|cancelled|pending|tertunda|menunggu pembayaran|sedang diproses|belum berhasil|belum dibayar|jatuh tempo|promo|diskon|hingga|otp|kode verifikasi)\b").unwrap().is_match(&combined);
    let amount = transaction_amount(&combined);
    let direction = incoming != outgoing;
    let Some(amount) = amount else {
        e.status = if incoming || outgoing || sender_bank(&e.sender).is_some() { "error" } else { "ignored" }.into();
        e.reason = "Nominal transaksi belum jelas. Periksa bukti email.".into();
        return Ok(e);
    };
    // Unknown or conflicting direction must never silently become an expense.
    let mut kind = if incoming && !outgoing { "income" } else { "expense" }.to_owned();
    let merchant = capture(r"(?im)^(?:nama[ \t]+merchant|merchant|nama[ \t]+penerima|penerima|pengirim|tujuan|pembayaran[ \t]+qris|transfer[ \t]+dari|transfer[ \t]+ke)[ \t]*:?[ \t]*(?:\r?\n[ \t]*)?([^\r\n]+)", &snippet)
        .map(|name| clean_merchant(&name))
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| if e.subject.trim().is_empty() { "Transaksi email".into() } else { clean_merchant(&e.subject) });
    let wallet_id = account
        .mappings
        .iter()
        .find(|m| m.bank == bank)
        .map(|m| m.wallet_id.clone())
        .unwrap_or_else(|| account.wallet_id.clone());
    let mut to_wallet_id = None;
    // Only an explicit user mapping may turn a top-up into an own-wallet transfer.
    if Regex::new(r"(?i)top\s*up\s+gopay")
        .unwrap()
        .is_match(&snippet)
    {
        if let Some(m) = account
            .mappings
            .iter()
            .find(|m| m.bank == "GoPay" && m.wallet_id != wallet_id)
        {
            kind = "transfer".into();
            to_wallet_id = Some(m.wallet_id.clone());
        }
    }
    let category = if kind == "income" {
        "income".into()
    } else if kind == "transfer" {
        "transfer".into()
    } else {
        category(&merchant)
    };
    let reference = capture(
        r"(?im)(?:no\.?\s*)?(?:referensi|reference|ref|order id|no\.?\s*mutasi)\s*[:#]?\s*([A-Z0-9][A-Z0-9_/-]{3,100})",
        &snippet,
    );
    let time = actual_time(&snippet);
    let observed = capture(
        r"(?im)saldo akhir\s*:?\s*((?:Rp\.?\s*)?[0-9][0-9.,]*)",
        &snippet,
    )
    .and_then(|s| {
        if s.trim().trim_start_matches("Rp").trim() == "0" {
            Some(0)
        } else {
            rupiah(&s)
        }
    });
    // Missing date/reference/category is no longer an auto-post blocker.
    let confident = direction && !blocked;
    let p = Parsed {
        bank: bank.into(),
        amount,
        kind,
        merchant,
        category,
        occurred_at: time.unwrap_or_else(|| e.received_at.clone()),
        reference,
        observed,
        wallet_id,
        to_wallet_id,
        confident,
    };
    // Sender-domain recognition is NOT authentication. Auto-post is explicit opt-in.
    if account.enabled && account.auto_post && p.confident {
        e.status = "posted".into();
        e.reason = "Tercatat otomatis dari email: nominal dan arah transaksi terbaca.".into()
    } else {
        e.reason = if !p.confident {
            "Arah transaksi belum jelas atau pesan bukan transaksi selesai. Periksa bukti email."
        } else {
            "Siap dikonfirmasi; pencatatan otomatis atau akun belum aktif."
        }
        .into()
    }
    e.parsed = Some(p);
    Ok(e)
}
