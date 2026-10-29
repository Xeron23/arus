use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedNotification {
    pub amount_idr: i64,
    pub direction: String, // "income" or "expense"
    pub merchant: String,
    pub wallet_hint: String, // "gopay", "dana", "ovo", "bca", "mandiri", "cash"
    pub method: String,      // "qris", "transfer", "payment"
}

pub fn is_ignorable(title: &str, text: &str) -> bool {
    static SKIP: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
        r"(?i)\b(?:otp|kode verifikasi|password|pin|promo|diskon|cashback hingga|gagal|failed|dibatalkan|cancelled|tertunda|pending|sedang diproses|menunggu|belum berhasil)\b"
    ).unwrap()
    });
    SKIP.is_match(&format!("{title}\n{text}"))
}

static MERCHANT_TOKO: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:ke|di|merchant)\s+([A-Za-z0-9\s\.\-_&]+?)(?:\s+(?:sebesar|berhasil|via|menggunakan|\.|\,|$))").unwrap()
});

static DARI_PENGIRIM: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)dari\s+([A-Za-z0-9\s\.\-_&]+?)(?:\s+(?:sebesar|berhasil|\.|\,|$))").unwrap()
});

pub fn parse_notification(
    package_name: &str,
    title: &str,
    text: &str,
) -> Option<ParsedNotification> {
    let s = format!("{title}\n{text}").to_lowercase();

    if is_ignorable(title, text) {
        return None;
    }
    let analysis = crate::parser::transaction_amount(&s);
    if analysis.ambiguous {
        return None;
    }
    let amount_idr = analysis.amount?;

    // Determine direction
    let incoming_words = [
        "menerima",
        "transfer masuk",
        "dana masuk",
        "pembayaran diterima",
        "top up berhasil",
        "isi saldo berhasil",
        "uang masuk",
    ];
    let outgoing_words = [
        "pembayaran berhasil",
        "berhasil membayar",
        "kamu berhasil bayar",
        "transfer keluar",
        "transfer ke",
        "berhasil transfer",
        "kirim uang",
    ];

    let is_incoming = incoming_words.iter().any(|w| s.contains(w));
    let is_outgoing = outgoing_words.iter().any(|w| s.contains(w));

    if is_incoming == is_outgoing {
        // Ambiguous direction
        return None;
    }

    let direction = if is_incoming {
        "income".to_string()
    } else {
        "expense".to_string()
    };

    // Wallet hint by package name
    let wallet_hint = match package_name {
        "com.gojek.gopay" | "com.gojek.app" => "gopay",
        "id.dana" => "dana",
        "ovo.id" => "ovo",
        "id.bmri.livin" => "mandiri",
        "com.bca" | "com.bca.mybca.omni.android" => "bca",
        "com.shopee.id" => "shopeepay",
        _ => return None,
    }
    .to_string();

    // Merchant / recipient extraction
    let merchant = if is_incoming {
        if let Some(caps) = DARI_PENGIRIM.captures(&format!("{title} {text}")) {
            caps[1].trim().to_string()
        } else {
            "Dana Masuk".to_string()
        }
    } else {
        if let Some(caps) = MERCHANT_TOKO.captures(&format!("{title} {text}")) {
            let m = caps[1].trim();
            if m.is_empty() || m.eq_ignore_ascii_case("rp") {
                "Pembayaran / Merchant".to_string()
            } else {
                m.to_string()
            }
        } else {
            "Pembayaran".to_string()
        }
    };

    let method = if s.contains("qris") {
        "qris".to_string()
    } else if s.contains("transfer") {
        "transfer".to_string()
    } else {
        "payment".to_string()
    };

    Some(ParsedNotification {
        amount_idr,
        direction,
        merchant,
        wallet_hint,
        method,
    })
}
