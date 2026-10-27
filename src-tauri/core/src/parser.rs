//! Conservative, text-only parser. Unsupported real bank layouts go to review.
use crate::{models::*, Result};
use chrono::{DateTime, FixedOffset, NaiveDateTime, TimeZone, Utc};
use mailparse::{MailHeaderMap, ParsedMail};
use regex::Regex;
use sha2::{Digest, Sha256};
use std::sync::OnceLock;
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
    static PREFIX: OnceLock<Regex> = OnceLock::new();
    let prefix = PREFIX.get_or_init(|| Regex::new(r"(?i)^(?:(?:transaksi\s+)?pembayaran\s+qris(?:\s+(?:ke|kepada))?|transfer\s+(?:ke|dari)|(?:nama\s+)?merchant|(?:nama\s+)?penerima|pengirim|tujuan)(?:\s*[:\-–]\s*|\s+|$)").unwrap());
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
/// Empty/placeholder references are not transaction identities.
pub fn normalize_reference(value: Option<&str>) -> Option<String> {
    let value = value?.trim().to_uppercase();
    if value.is_empty()
        || [
            "-",
            "--",
            "N/A",
            "NA",
            "NULL",
            "NONE",
            "UNKNOWN",
            "TIDAK ADA",
            "NOT AVAILABLE",
        ]
        .contains(&value.as_str())
        || value
            .chars()
            .all(|c| c == '0' || c == '-' || c == '/' || c.is_whitespace())
    {
        None
    } else {
        Some(value)
    }
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
    let bank = sender_bank(&e.sender).unwrap_or("Unknown");
    let amount = capture(
        r"(?im)(?:nominal|jumlah|total(?: pembayaran)?)\s*:?\s*((?:Rp\.?\s*)?[0-9][0-9.,]*)",
        &snippet,
    )
    .and_then(|s| rupiah(&s));
    if amount.is_none() {
        if bank != "Unknown"
            && Regex::new(r"(?i)berhasil|mutasi|transfer|qris|debit|kredit")
                .unwrap()
                .is_match(&format!("{} {}", e.subject, snippet))
        {
            e.status = "error".into();
            e.reason = "Notifikasi finansial belum dapat dibaca: nominal tidak ditemukan.".into()
        } else {
            e.status = "ignored".into();
            e.reason = "Promosi / email tanpa mutasi yang dapat dikenali.".into()
        }
        return Ok(e);
    }
    let merchant = capture(r"(?im)^(?:nama[ \t]+merchant|merchant|nama[ \t]+penerima|penerima|pengirim|pembayaran[ \t]+qris|transfer[ \t]+dari|transfer[ \t]+ke)[ \t]*:?[ \t]*(?:\r?\n[ \t]*)?([^\r\n]+)",&snippet)
        .map(|name| clean_merchant(&name)).filter(|name| !name.is_empty())
        .unwrap_or_else(|| "Merchant belum dikenali".into());
    let mut kind = if Regex::new(r"(?i)transfer dari|dana masuk|(?:^|\n)kredit")
        .unwrap()
        .is_match(&snippet)
    {
        "income"
    } else {
        "expense"
    }
    .to_owned();
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
    let raw_reference = capture(
        r"(?im)^[ \t]*(?:no\.?[ \t]*)?(?:referensi|reference|ref|order[ \t]+id|no\.?[ \t]*mutasi)[ \t]*[:#]?[ \t]*([^\r\n]+)",
        &snippet,
    );
    let reference = normalize_reference(raw_reference.as_deref());
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
    let direction = Regex::new(
        r"(?i)transfer dari|transfer ke|pembayaran qris|top\s*up|(?:^|\n)(?:debit|kredit)",
    )
    .unwrap()
    .is_match(&snippet);
    let confident = bank != "Unknown"
        && reference.is_some()
        && time.is_some()
        && direction
        && merchant != "Merchant belum dikenali"
        && (category != "other" || kind != "expense");
    let p = Parsed {
        bank: bank.into(),
        amount: amount.unwrap(),
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
    if account.auto_post && p.confident {
        e.status = "posted".into();
        e.reason = "Auto-post opt-in; nominal, arah, referensi dan waktu transaksi terbaca.".into()
    } else {
        e.reason = if !p.confident {
            "Format ambigu / belum dikenali. Periksa nominal, tanggal, dan rekening."
        } else {
            "Siap dikonfirmasi; auto-post akun dinonaktifkan."
        }
        .into()
    }
    e.parsed = Some(p);
    Ok(e)
}
