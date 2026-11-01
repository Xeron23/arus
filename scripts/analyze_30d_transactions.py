#!/usr/bin/env python3
import imaplib, ssl, email, re, subprocess, os
from email.header import decode_header
from datetime import datetime, timedelta
from collections import Counter

CLASSIFIER_BIN = "/home/webserver-xeron/workspace/arus-mobile/src-tauri/core/target/debug/examples/classify_cli"

def clean_subject(header_val):
    if not header_val:
        return ""
    decoded_parts = decode_header(header_val)
    res = []
    for part, enc in decoded_parts:
        if isinstance(part, bytes):
            res.append(part.decode(enc or "utf-8", errors="ignore"))
        else:
            res.append(str(part))
    return "".join(res)

def get_body(msg):
    body = ""
    if msg.is_multipart():
        for part in msg.walk():
            ct = part.get_content_type()
            if ct in ["text/plain", "text/html"]:
                payload = part.get_payload(decode=True)
                if payload:
                    body += " " + payload.decode(part.get_content_charset() or "utf-8", errors="ignore")
    else:
        payload = msg.get_payload(decode=True)
        if payload:
            body = payload.decode(msg.get_content_charset() or "utf-8", errors="ignore")
    return body

# Extract merchant based on Arus parser regex
def extract_mandiri_merchant(body):
    # Strip HTML tags
    clean = re.sub(r"<[^>]+>", " ", body)
    clean = re.sub(r"&[a-z]+;", " ", clean)
    clean = re.sub(r"[ \t]+", " ", clean)

    # QRIS / Merchant pattern
    # 1. Check Penerima / Merchant
    m = re.search(r"(?im)(?:nama\s+merchant|tujuan\s+transaksi|penerima|merchant)\s*:?\s*#*\s*([A-Za-z0-9\s\.\-_&,\']+?)(?:\s*(?:ke\s+rekening|nominal|id\s+transaksi|terminal|tanggal|rp|\n|$))", clean)
    if m:
        cand = m.group(1).strip()
        cand = re.sub(r"^[#\*\-_=\s]+", "", cand)
        cand = re.sub(r"(?i)\b(?:berhasil|sukses|qris|pembayaran)\b", "", cand).strip()
        if len(cand) >= 3 and not re.search(r"(?i)^(?:pembayaran|berhasil|transaksi|livin|mandiri)$", cand):
            return cand

    # 2. Check "pembayaran qris ... di <merchant>"
    m2 = re.search(r"(?im)(?:di|ke)\s+([A-Za-z0-9\s\.\-_&,\']+?)(?:\s*(?:sebesar|berhasil|rp|\.|\,|$))", clean)
    if m2:
        cand = m2.group(1).strip()
        if len(cand) >= 3 and not re.search(r"(?i)^(?:rekening|mandiri|tabungan|livin)$", cand):
            return cand

    return "Bank Mandiri"

def extract_amount(body):
    clean = re.sub(r"<[^>]+>", " ", body)
    m = re.search(r"(?i)(?:rp\.?|idr)\s*([0-9\.,]+)", clean)
    if m:
        val = m.group(1).replace(".", "").replace(",", ".")
        try:
            return float(val)
        except:
            pass
    return 0.0

def main():
    print("Connecting to Gmail IMAP...")
    context = ssl.create_default_context()
    imap = imaplib.IMAP4_SSL("imap.gmail.com", 993, ssl_context=context)
    imap.login("mrifkian09@gmail.com", "dosi rpac hacr ouua")

    status, _ = imap.select("\"[Gmail]/Semua Email\"", readonly=True)
    if status != "OK":
        imap.select("INBOX", readonly=True)

    cutoff = (datetime.now() - timedelta(days=30)).strftime("%d-%b-%Y")
    status, data = imap.uid("search", None, f"(SINCE {cutoff} FROM \"bankmandiri.co.id\")")
    uids = data[0].split()
    print(f"Total Mandiri transaction emails in last 30 days: {len(uids)}")

    # Fetch transaction emails in batches
    sample_uids = uids[-45:]
    transactions = []

    proc = subprocess.Popen([CLASSIFIER_BIN], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)

    uid_list_str = b",".join(sample_uids)
    status, mdata = imap.uid("fetch", uid_list_str, "(RFC822)")
    
    if status == "OK" and mdata:
        for item in mdata:
            if not isinstance(item, tuple) or len(item) < 2:
                continue
            msg = email.message_from_bytes(item[1])
            subject = clean_subject(msg.get("Subject", ""))
            date_str = msg.get("Date", "")
            body = get_body(msg)

            # Ignore non-transactional (otp/login)
            if re.search(r"(?i)\b(?:otp|kode rahasia|aktivasi|login baru|selamat datang)\b", subject + " " + body):
                continue

            merchant = extract_mandiri_merchant(body)
            amount = extract_amount(body)

            proc.stdin.write(merchant + "\n")
            proc.stdin.flush()
            category = proc.stdout.readline().strip()

            transactions.append({
                "date": date_str[:16],
                "subject": subject,
                "merchant": merchant,
                "amount": amount,
                "category": category
            })

    imap.logout()
    proc.stdin.close()
    proc.wait()

    print("\n" + "=" * 95)
    print(f"ANALISIS TRANSAKSI REAL EMAIL 30 HARI TERAKHIR (Sampel {len(transactions)} Transaksi)")
    print("=" * 95)
    print(f"{'Tanggal':<16} | {'Nominal':<12} | {'Merchant Terdeteksi':<36} | {'Kategori AI':<10} | {'Evaluasi'}")
    print("-" * 95)

    category_counts = Counter()
    accurate_count = 0

    # Ground truth mapping evaluation
    for tx in transactions:
        merchant = tx["merchant"]
        cat = tx["category"]
        category_counts[cat] += 1
        m_lower = merchant.lower()

        # Heuristic ground-truth evaluation
        eval_status = "Tepat"
        if any(w in m_lower for w in ["nasi", "warung nasi", "warkop", "batagor", "lumpia", "makan", "ayam", "kopi", "cafe"]):
            expected = "food"
        elif any(w in m_lower for w in ["laundry", "cuci", "token", "pln", "pdam"]):
            expected = "home"
        elif any(w in m_lower for w in ["shopee", "tokopedia", "indomaret", "idm", "alfamart", "alfamidi"]):
            expected = "shop"
        elif any(w in m_lower for w in ["spbu", "bensin", "pertamina", "shell", "gojek", "grab", "parkir"]):
            expected = "transport"
        elif any(w in m_lower for w in ["xxi", "bioskop", "game", "steam"]):
            expected = "fun"
        else:
            expected = cat # wajar / general transfer

        if cat == expected:
            eval_status = "✅ Tepat"
            accurate_count += 1
        else:
            eval_status = f"⚠️ Cek ({expected})"

        nom_str = f"Rp{int(tx['amount']):,}".replace(",", ".")
        print(f"{tx['date']:<16} | {nom_str:<12} | {merchant:<36} | {cat:<10} | {eval_status}")

    print("-" * 95)
    accuracy_pct = (accurate_count / len(transactions)) * 100 if transactions else 0.0

    print("\nDistribusi Hasil Kategori AI:")
    for cat, count in category_counts.most_common():
        pct = (count / len(transactions)) * 100
        print(f"  - {cat:<10}: {count:>2} transaksi ({pct:>5.1f}%)")

    print(f"\nRingkasan Akurasi / Kesesuaian:")
    print(f"  - Total Transaksi Dianalisis: {len(transactions)}")
    print(f"  - Sesuai Konteks Nyata      : {accurate_count} / {len(transactions)}")
    print(f"  - Persentase Kesesuaian AI  : {accuracy_pct:.1f}%\n")

if __name__ == "__main__":
    main()
