import { appVersion } from "../version";
import { syncRangeOptions } from "../sync-options";
import type { Snapshot, LedgerEntry, Wallet, EmailRecord } from "../models";
import { icon, logo } from "./icons";
export const esc = (v: unknown) =>
  String(v ?? "").replace(
    /[&<>"']/g,
    (c) =>
      ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[
        c
      ]!,
  );
export const money = (n: number) => "Rp" + n.toLocaleString("id-ID");
export const cats: Record<
  string,
  { name: string; icon: string; color: string }
> = {
  food: { name: "Makanan & Minuman", icon: "food", color: "#a1bb7c" },
  shop: { name: "Belanja", icon: "shop", color: "#c9d8a4" },
  transport: { name: "Transportasi", icon: "car", color: "#d7a77d" },
  home: { name: "Rumah & Tagihan", icon: "house", color: "#244e40" },
  fun: { name: "Hiburan", icon: "spark", color: "#a4b8c8" },
  other: { name: "Lainnya", icon: "receipt", color: "#d1c9b1" },
  income: { name: "Pemasukan", icon: "down", color: "#7b9d6b" },
  transfer: { name: "Transfer internal", icon: "transfer", color: "#89a9b8" },
};
export type Page = "dashboard" | "transactions" | "review" | "settings";
export interface View {
  page: Page;
  month: string;
  wallet: string;
  search: string;
  category: string;
  kind: string;
  status: string;
  hidden: boolean;
}
export const today = () =>
  new Intl.DateTimeFormat("en-CA", {
    timeZone: "Asia/Jakarta",
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  }).format(new Date());
export const dateText = (s: string) =>
  new Date(s).toLocaleString("id-ID", {
    timeZone: "Asia/Jakarta",
    day: "numeric",
    month: "short",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
export const localTime = (s: string) =>
  new Date(s)
    .toLocaleString("sv-SE", { timeZone: "Asia/Jakarta" })
    .replace(" ", "T")
    .slice(0, 16);
export function walletOptions(s: Snapshot, selected: string, all = false) {
  return (
    (all ? '<option value="all">Semua dompet</option>' : "") +
    s.wallets
      .map(
        (w) =>
          `<option value="${esc(w.id)}" ${selected === w.id ? "selected" : ""}>${esc(w.name)}</option>`,
      )
      .join("")
  );
}
export function categoryOptions(
  selected: string,
  kind = "expense",
  all = false,
) {
  return (
    (all ? '<option value="all">Semua kategori</option>' : "") +
    Object.entries(cats)
      .filter(([k]) =>
        kind === "expense"
          ? !["income", "transfer"].includes(k)
          : kind === "all"
            ? true
            : k === kind,
      )
      .map(
        ([k, c]) =>
          `<option value="${k}" ${selected === k ? "selected" : ""}>${c.name}</option>`,
      )
      .join("")
  );
}
export const btn = (act: string, label: string, cls = "primary", id = "") =>
  `<button class="${cls}" data-act="${act}" ${act === "close" ? 'aria-label="Tutup"' : act === "filters" ? 'aria-label="Filter transaksi"' : ""} ${id ? `data-id="${esc(id)}"` : ""}>${label}</button>`;
export const empty = (title: string, text: string) =>
  `<div class="empty"><div class="empty-icon">${icon("receipt")}</div><h3>${title}</h3><p>${text}</p></div>`;
export const walletName = (s: Snapshot, id: string | null) =>
  s.wallets.find((w) => w.id === id)?.name || "—";
export function row(s: Snapshot, t: LedgerEntry) {
  const c = cats[t.category] || cats.other;
  return `<button class="tx" data-act="transaction" data-id="${esc(t.id)}"><span class="tx-icon ${t.kind === "transfer" ? "blue" : t.category === "food" ? "orange" : ""}">${icon(c.icon)}</span><div class="grow"><h3>${esc(t.merchant)}</h3><p>${esc(walletName(s, t.walletId))} · ${c.name}</p></div><div class="tx-amount ${t.kind === "income" ? "income" : ""}">${t.kind === "income" ? "+" : t.kind === "expense" ? "−" : ""}${money(t.amount)}<small>${t.kind === "transfer" ? "Antar-dompet" : t.source} · ${new Date(t.occurredAt).toLocaleTimeString("id-ID", { hour: "2-digit", minute: "2-digit", timeZone: "Asia/Jakarta" })}</small></div></button>`;
}
export function filtered(s: Snapshot, v: View) {
  return s.transactions
    .filter(
      (t) =>
        t.occurredAt.startsWith(v.month) &&
        (v.wallet === "all" ||
          t.walletId === v.wallet ||
          t.toWalletId === v.wallet) &&
        (v.category === "all" || t.category === v.category) &&
        (v.kind === "all" || t.kind === v.kind) &&
        (!v.search ||
          [t.merchant, t.note, t.reference || ""]
            .join(" ")
            .toLowerCase()
            .includes(v.search.toLowerCase())),
    )
    .sort((a, b) => Date.parse(b.occurredAt) - Date.parse(a.occurredAt));
}
export function monthSelect(s: Snapshot, v: View) {
  const months = new Set([
    today().slice(0, 7),
    v.month,
    ...s.transactions.map((t) => t.occurredAt.slice(0, 7)),
  ]);
  for (let i = 1; i < 6; i++) {
    const d = new Date(today().slice(0, 7) + "-01T12:00:00");
    d.setMonth(d.getMonth() - i);
    months.add(d.toISOString().slice(0, 7));
  }
  return `<select class="date-select" id="monthSelect" aria-label="Pilih bulan">${[
    ...months,
  ]
    .sort()
    .reverse()
    .map(
      (m) =>
        `<option value="${m}" ${v.month === m ? "selected" : ""}>${new Date(m + "-01T12:00:00").toLocaleDateString("id-ID", { month: "long", year: "numeric" })}</option>`,
    )
    .join("")}</select>`;
}
function monthEntries(s: Snapshot, v: View) {
  return s.transactions.filter(
    (t) =>
      t.occurredAt.startsWith(v.month) &&
      (v.wallet === "all" ||
        t.walletId === v.wallet ||
        t.toWalletId === v.wallet),
  );
}
export function chart(s: Snapshot, v: View) {
  const end =
      v.month === today().slice(0, 7)
        ? Number(today().slice(-2))
        : new Date(
            Number(v.month.slice(0, 4)),
            Number(v.month.slice(5)),
            0,
          ).getDate(),
    days = Array.from({ length: 7 }, (_, i) => end - 6 + i),
    tx = monthEntries(s, v),
    vals = days.map((d) => {
      const ts = tx.filter(
        (t) => Number(t.occurredAt.slice(8, 10)) === d && d > 0,
      );
      return [
        ts.filter((t) => t.kind === "income").reduce((n, t) => n + t.amount, 0),
        ts
          .filter((t) => t.kind === "expense")
          .reduce((n, t) => n + t.amount, 0),
      ];
    }),
    max = Math.max(1, ...vals.flat());
  return `<div class="row between"><div class="legend"><span><i></i>Masuk</span><span><i class="lime"></i>Keluar</span></div><span class="tiny muted">7 hari terakhir</span></div><div class="chart" role="img" aria-label="Grafik arus kas tujuh hari, transfer internal dikecualikan">${vals.map((n, i) => `<button class="bar-group" data-act="chart-day" data-id="${days[i]}" aria-label="Tanggal ${days[i]}: masuk ${money(n[0])}, keluar ${money(n[1])}"><span class="bar" style="height:${Math.max(2, (n[0] / max) * 95)}%"></span><span class="bar out" style="height:${Math.max(2, (n[1] / max) * 95)}%"></span></button>`).join("")}</div><div class="bar-labels">${days.map((d) => `<span>${d > 0 ? d : "—"}</span>`).join("")}</div>`;
}
function walletCard(w: Wallet) {
  return `<button class="wallet-card" data-act="wallet-detail" data-id="${esc(w.id)}"><div class="row"><span class="wallet-brand bank-color">${esc(w.bank.slice(0, 3).toUpperCase())}</span><span class="grow"><b class="small">${esc(w.name)}</b><span class="tiny muted" style="display:block">${esc(w.kind)}${w.digits ? " · •••• " + esc(w.digits) : ""}</span></span>${icon("chevron")}</div><div class="wallet-amount">${money(w.balance)}</div><div class="wallet-bottom"><span>Buku kas lokal</span><span>${w.observed === null ? "Atur saldo awal" : w.observed === w.balance ? "✓ Saldo cocok" : "Selisih " + money(Math.abs(w.observed - w.balance))}</span></div></button>`;
}
export function dashboard(s: Snapshot, v: View, native: boolean) {
  const list = monthEntries(s, v),
    income = list
      .filter((t) => t.kind === "income")
      .reduce((n, t) => n + t.amount, 0),
    expense = list
      .filter((t) => t.kind === "expense")
      .reduce((n, t) => n + t.amount, 0),
    balance = s.wallets
      .filter((w) => v.wallet === "all" || w.id === v.wallet)
      .reduce((n, w) => n + w.balance, 0),
    pending = s.emails.filter((e) => e.status === "review").length;
  return `<div class="row between"><p class="greeting">Halo, selamat datang kembali 👋</p>${monthSelect(s, v)}</div><h1 class="page-title">Uangmu, dalam kendali.</h1><section class="hero"><div class="row between"><label>Total saldo ${btn("hide-balance", icon("eye"), "eye")}</label><select id="walletSelect" class="wallet-select" aria-label="Dompet dashboard">${walletOptions(s, v.wallet, true)}</select></div><div class="balance">${v.hidden ? "••••••••" : `<small>Rp</small>${balance.toLocaleString("id-ID")}`}</div><div class="hero-bottom"><i class="live"></i>${native ? "Tersimpan di ponselmu" : "Mode browser · data demo"}<b>${s.wallets.length} dompet</b></div></section><div class="flow-grid"><div class="flow-card"><div class="row" style="gap:0"><span class="flow-icon">${icon("down")}</span><label>Pemasukan bulan ini</label></div><strong>${v.hidden ? "••••••" : money(income)}</strong></div><div class="flow-card"><div class="row" style="gap:0"><span class="flow-icon out">${icon("up")}</span><label>Pengeluaran bulan ini</label></div><strong>${v.hidden ? "••••••" : money(expense)}</strong></div></div><div class="quick-actions"><button class="quick-action" data-act="manual"><i>${icon("plus")}</i>Catat uang</button><button class="quick-action" data-act="transfer"><i>${icon("transfer")}</i>Transfer</button><button class="quick-action" data-act="sync"><i>${icon("sync")}</i>Tarik email</button><button class="quick-action" data-act="analytics"><i>${icon("chart")}</i>Analitik</button></div><div id="syncInline"></div>${pending ? `<button class="review-banner" data-page="review"><span class="mini-symbol">${icon("mail")}</span><span class="grow"><strong>${pending} transaksi menunggu review</strong><small>Kenali merchant, lalu masukkan ke buku kas.</small></span>${icon("chevron")}</button>` : ""}${native && !s.accounts.length ? '<div class="notice">Mulai dengan mengatur saldo awal dompet dan menghubungkan email di Pengaturan.</div>' : ""}<div class="section-title"><h2>Dompetmu</h2><button class="text-btn" data-act="wallet-new">${icon("plus")}Tambah</button></div><div class="wallet-carousel" aria-label="Geser kartu dompet">${s.wallets.map(walletCard).join("")}</div><div class="section-title"><h2>Arus kas</h2><button class="text-btn" data-act="analytics">Lihat detail ${icon("chevron")}</button></div><div class="card chart-card">${chart(s, v)}</div><div class="section-title"><h2>Transaksi terbaru</h2><button class="text-btn" data-page="transactions">Lihat semua ${icon("chevron")}</button></div>${
    [...list]
      .sort((a, b) => Date.parse(b.occurredAt) - Date.parse(a.occurredAt))
      .slice(0, 5)
      .map((t) => row(s, t))
      .join("") ||
    empty(
      "Belum ada transaksi",
      "Catat manual atau sinkronkan notifikasi email.",
    )
  }`;
}
export function transactionRows(s: Snapshot, v: View) {
  let last = "";
  return (
    filtered(s, v)
      .map((t) => {
        const day = t.occurredAt.slice(0, 10),
          heading =
            last !== day
              ? `<p class="group-date">${new Date(day + "T12:00:00").toLocaleDateString("id-ID", { day: "numeric", month: "long", year: "numeric" })}</p>`
              : "";
        last = day;
        return heading + row(s, t);
      })
      .join("") ||
    empty("Tidak ada transaksi", "Ubah pencarian, kategori, atau dompet.")
  );
}
export function transactions(s: Snapshot, v: View) {
  return `<div class="row between"><h1 class="page-title">Transaksi</h1>${monthSelect(s, v)}</div><div class="toolbar"><div class="search">${icon("search")}<input id="searchInput" type="search" value="${esc(v.search)}" placeholder="Cari merchant atau catatan…" aria-label="Cari transaksi"></div>${btn("filters", icon("filter"), "filter-btn")}</div><div class="chips">${[
    ["all", "Semua"],
    ["expense", "Keluar"],
    ["income", "Masuk"],
    ["transfer", "Transfer"],
  ]
    .map(
      ([k, n]) =>
        `<button class="chip ${v.kind === k ? "active" : ""}" data-act="kind-filter" data-id="${k}">${n}</button>`,
    )
    .join(
      "",
    )}</div><div class="mini-total"><span id="resultCount">${filtered(s, v).length} transaksi</span><strong>${v.category === "all" ? "Semua kategori" : cats[v.category].name}</strong></div><div id="transactionRows">${transactionRows(s, v)}</div>`;
}
export const statusName: Record<string, string> = {
  review: "Perlu review",
  posted: "Tercatat",
  ignored: "Diabaikan",
  error: "Error",
  duplicate: "Duplikat",
};
export function review(s: Snapshot, v: View) {
  const emails = s.emails.filter((e) => e.status === v.status);
  return `<h1 class="page-title" style="margin-top:7px">Review & audit</h1><div class="notice orange">Transaksi perlu review belum mengubah saldo. Periksa nominal, tanggal, dan rekening dari bukti email.</div><div class="chips">${Object.entries(
    statusName,
  )
    .map(
      ([k, n]) =>
        `<button class="chip ${k === v.status ? "active" : ""}" data-act="status-filter" data-id="${k}">${n} ${s.emails.filter((e) => e.status === k).length}</button>`,
    )
    .join(
      "",
    )}</div>${emails.map((e) => reviewCard(e)).join("") || empty("Kotak ini sudah rapi", "Tidak ada email pada status yang dipilih.")}`;
}
function reviewCard(e: EmailRecord) {
  return `<button class="card w-full text-left block" data-act="audit" data-id="${esc(e.id)}"><div class="row"><span class="tx-icon ${e.status === "review" ? "orange" : ""}">${icon("mail")}</span><div class="grow"><div class="strong small">${esc(e.parsed?.merchant || e.subject)}</div><div class="tiny muted" style="margin-top:4px">${esc(e.sender)}</div></div>${icon("chevron")}</div><div class="row between" style="margin:13px 0 8px"><span class="tag ${e.status === "error" ? "red" : e.status === "review" ? "orange" : "gray"}">${statusName[e.status]}</span><strong class="small">${e.parsed?.amount ? money(e.parsed.amount) : "—"}</strong></div><div class="tiny muted">${esc(e.reason)}</div></button>`;
}
function settingsItem(act: string, ico: string, title: string, desc: string) {
  return `<button class="settings-item" data-act="${act}"><span class="settings-icon">${icon(ico)}</span><span class="grow"><h3>${title}</h3><p>${desc}</p></span>${icon("chevron")}</button>`;
}
export function settings(s: Snapshot, native: boolean) {
  return `<h1 class="page-title" style="margin-top:7px">Ruang kendalimu</h1><div class="profile"><div class="avatar">a.</div><div><h3>Teman Arus</h3><p>${native ? "Catatan pribadi di ponselmu" : "Pratinjau dengan data contoh"}</p></div><span class="tag ml-auto">${native ? "PRIBADI" : "DEMO"}</span></div><div class="section-title"><h2>Koneksi email</h2><button class="text-btn" data-act="connect">${icon("plus")}Tambah</button></div>${s.accounts.map((a) => `<div class="email-card"><div class="row"><span class="tx-icon">${icon("mail")}</span><div class="grow"><h3>${esc(a.email)}</h3><p>${esc(a.provider)} · ${a.enabled ? (a.unlocked ? "Sesi aktif" : "Perlu App Password") : "Dijeda"}</p></div><button class="icon-btn" data-act="account" data-id="${esc(a.id)}" aria-label="Pengaturan ${esc(a.email)}">${icon("edit")}</button></div><div class="email-meta"><code>${esc(a.folder || "Folder otomatis")}</code><span>Email tetap belum dibaca</span></div><div class="tiny muted">Dompet utama: ${esc(walletName(s, a.walletId))}</div><div class="email-actions"><button data-act="${a.unlocked ? "lock-account" : "unlock-account"}" data-id="${esc(a.id)}">${a.unlocked ? "Tutup sesi" : "Buka sesi"}</button><button data-act="account" data-id="${esc(a.id)}">Mapping & opsi</button></div></div>`).join("") || empty("Hubungkan email pertamamu", "Email dibaca langsung dari perangkat.")}<label class="field" for="syncRange">Rentang email yang disinkronkan</label><select class="field" id="syncRange">${syncRangeOptions()}</select><p class="tiny muted">Mulai dari 30 hari agar lebih cepat. Semua email diproses bertahap.</p><div class="button-row">${btn("connect", icon("mail") + " Hubungkan email", "primary")}${btn("sync", icon("sync") + " Sync", "secondary")}</div><div class="section-title"><h2>Preferensi & data</h2></div><div class="card" style="padding:2px 15px"><div class="toggle-row"><span class="strong">Tema gelap<small>Tersimpan lokal di perangkat ini.</small></span><button class="switch ${s.theme === "dark" ? "on" : ""}" role="switch" aria-checked="${s.theme === "dark"}" data-act="theme" aria-label="Tema gelap"></button></div>${!native ? settingsItem("export", "download", "Ekspor buku kas CSV", "Laporan transaksi tercatat bulan ini") : ""}${settingsItem("help", "info", "Tentang penyimpanan lokal", "Cara Arus menjaga catatan Anda di ponsel")}${settingsItem("check-update", "sync", "Periksa Pembaruan", `Versi ${appVersion} · Cek rilis terbaru`)}</div><div class="section-title"><h2>Notifikasi HP (QRIS & E-Wallet)</h2></div><div class="card" style="padding:15px"><div class="notification-access"><div><h3 class="small strong">Akses Notifikasi Android</h3><p class="tiny muted" style="margin-top:3px">Catat mutasi GoPay, OVO, DANA & QRIS instan dari notifikasi HP.</p></div><button class="btn small primary notification-access-button" data-act="notif-settings">${icon("shield")} Akses</button></div><div class="button-row" style="margin-top:12px"><button class="btn secondary small w-full" data-act="notif-sync">${icon("sync")} Tarik Notifikasi Sekarang</button></div></div><div class="privacy-card">${icon("shield")}<h3>Uangmu tetap milikmu.</h3><p>${native ? "Privasi Anda adalah prioritas utama. Catatan keuangan Anda tersimpan di ponsel dan tidak dikirim ke server pengembang. Anda tetap memegang kendali atas data Anda." : "Mode ini menggunakan data contoh dan tidak menghubungi IMAP. Jangan masukkan kredensial asli."}</p></div><div class="footer-text">Arus · ${esc(appVersion)}</div>`;
}
export function shell(v: View, s: Snapshot, native: boolean) {
  const pending = s.emails.filter((e) => e.status === "review").length;
  const pages = { dashboard, transactions, review, settings };
  return `<div class="phone"><header class="app-header"><div class="wordmark"><span class="brand-icon">${logo()}</span>arus<span style="color:#9bae76">.</span></div><span class="sub-brand">uangmu, utuh.</span><div class="header-actions"><button class="icon-btn" data-act="help" aria-label="Panduan Arus">${icon("info")}</button><button class="icon-btn" data-page="review" aria-label="Buka review">${icon("bell")}${pending ? '<i class="dot"></i>' : ""}</button></div></header><main class="content" id="content"><div class="page-enter">${v.page === "settings" ? settings(s, native) : v.page === "dashboard" ? dashboard(s, v, native) : pages[v.page](s, v)}</div></main><button class="fab" data-act="manual" aria-label="Tambah transaksi tunai">${icon("plus")}</button><nav class="bottom-nav" aria-label="Navigasi utama">${[
    ["dashboard", "home", "Beranda"],
    ["transactions", "activity", "Transaksi"],
    ["review", "mail", "Review"],
    ["settings", "more", "Pengaturan"],
  ]
    .map(
      ([p, i, n]) =>
        `<button class="nav-item ${p === v.page ? "active" : ""}" data-page="${p}" ${p === v.page ? 'aria-current="page"' : ""}>${icon(i)}<span>${n}</span>${p === "review" && pending ? `<b class="nav-count">${pending}</b>` : ""}</button>`,
    )
    .join(
      "",
    )}</nav><div class="overlay hidden" id="overlay"><section class="sheet" id="sheet" role="dialog" aria-modal="true" aria-labelledby="sheetTitle" tabindex="-1"><div class="sheet-handle"></div><header class="sheet-header" id="sheetHeader"></header><div class="sheet-body" id="sheetBody"></div></section></div><div id="toast" class="toast hidden" role="status" aria-live="polite"></div></div>`;
}
