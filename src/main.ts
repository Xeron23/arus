import "./styles.css";
import { api, errorText } from "./native/client";
import type { Snapshot, Kind, EntryInput, ConnectionInput, ConnectionReport, SyncProgress, ReviewInput, AccountUpdate } from "./models";
import { getSyncRange, setSyncRange, syncRangeOptions } from "./sync-options";
import { installSheetGesture } from "./sheet-gesture";
import {
  shell,
  esc,
  money,
  cats,
  btn,
  empty,
  row,
  chart,
  walletOptions,
  categoryOptions,
  walletName,
  today,
  dateText,
  localTime,
  transactionRows,
  filtered,
  type View,
  type Page,
} from "./ui/templates";
import { icon } from "./ui/icons";
import { classifyMerchant, cleanMerchant } from "./native/mock";
const $ = <T extends Element = HTMLElement>(s: string) => document.querySelector<T>(s)!;
let data: Snapshot = {
  wallets: [],
  transactions: [],
  emails: [],
  accounts: [],
  theme: "light",
};
const view: View = {
  page: "dashboard",
  month: today().slice(0, 7),
  wallet: "all",
  search: "",
  category: "all",
  kind: "all",
  status: "review",
  hidden: false,
  selectionMode: false,
  selectedIds: new Set<string>(),
  reviewBusy: false,
};
let busy = false,
  sheetId = "",
  formKind: Kind = "expense",
  manualDraft: Record<string, string> = {},
  wizardStep = 0,
  connectionDraft: Record<string, string> = {},
  testedKey = "",
  testedReport: ConnectionReport | null = null,
  undo: (() => Promise<void>) | null = null,
  toastTimer: ReturnType<typeof setTimeout>,
  lastFocused: HTMLElement | null = null;
function render(preserveScroll = false) {
  const y = preserveScroll ? $("#content")?.scrollTop || 0 : 0;
  document.documentElement.classList.toggle("dark", data.theme === "dark");
  $("#app").innerHTML = shell(view, data, api.native);
  $("#content").scrollTop = y;
}
async function refresh() {
  data = await api.snapshot();
  const pending = new Set(data.emails.filter((e) => e.status === "review").map((e) => e.id));
  view.selectedIds = new Set([...(view.selectedIds || [])].filter((id) => pending.has(id)));
  if (!pending.size) view.selectionMode = false;
  render();
}
function close() {
  const o = $("#overlay");
  o?.classList.add("hidden");
  sheetId = "";
  if (lastFocused?.isConnected) lastFocused.focus();
  lastFocused = null;
}
function sheet(title: string, html: string, subtitle = "") {
  if ($("#overlay").classList.contains("hidden")) lastFocused = document.activeElement as HTMLElement;
  $("#sheetHeader").innerHTML =
    `<div><h2 id="sheetTitle">${esc(title)}</h2>${subtitle ? `<p>${esc(subtitle)}</p>` : ""}</div>${btn("close", icon("close"), "icon-btn")}`;
  $("#sheetBody").innerHTML = html;
  $("#overlay").classList.remove("hidden");
  $("#sheetBody").scrollTop = 0;
  $("#sheet").focus();
}
function toast(message: string, fn?: () => Promise<void>) {
  clearTimeout(toastTimer);
  undo = fn || null;
  $("#toast").innerHTML =
    `<span class="toast-text">${esc(message)}</span>${fn ? btn("undo", "Undo", "text-btn") : btn("dismiss", "×", "close-small")}`;
  $("#toast").classList.remove("hidden");
  toastTimer = setTimeout(
    () => {
      $("#toast")?.classList.add("hidden");
      undo = null;
    },
    fn ? 12000 : 6000,
  );
}
function fail(e: unknown) {
  const p = $("#formError");
  if (p) p.textContent = errorText(e);
  else toast(errorText(e));
}
function input(name: string, label: string, value: unknown = "", type = "text", attrs = "") {
  return `<label class="field" for="f-${name}">${label}</label><input class="field" type="${type}" id="f-${name}" name="${name}" value="${esc(value)}" ${attrs}>`;
}
function select(name: string, label: string, options: string) {
  return `<label class="field" for="f-${name}">${label}</label><select class="field" name="${name}" id="f-${name}">${options}</select>`;
}
const err = () => '<p class="error-message" id="formError" role="alert"></p>';
const entries = (f: HTMLFormElement) => Object.fromEntries(new FormData(f).entries()) as Record<string, string>;
function entryFrom(d: Record<string, string>): EntryInput {
  return {
    walletId: d.walletId,
    toWalletId: d.kind === "transfer" ? d.toWalletId : null,
    kind: d.kind as Kind,
    amount: Number(d.amount),
    merchant: d.merchant.trim(),
    category: d.kind === "income" ? "income" : d.kind === "transfer" ? "transfer" : d.category,
    occurredAt: d.occurredAt + ":00+07:00",
    note: d.note || "",
  };
}
function entryFields(d: Record<string, string>, kind: Kind) {
  return `${input("amount", "Nominal (Rupiah)", d.amount || "", "number", 'required min="1" max="100000000000000" step="1" inputmode="numeric"')}${input("merchant", "Merchant / nama transaksi", d.merchant || "", "text", 'required maxlength="80" placeholder="Contoh: Warteg Bahari"')}<input type="hidden" name="kind" value="${kind}"><div class="field-grid"><div>${select("walletId", kind === "transfer" ? "Dompet asal" : "Dompet", walletOptions(data, d.walletId || "cash"))}</div><div>${kind === "transfer" ? select("toWalletId", "Dompet tujuan", walletOptions(data, d.toWalletId || "gopay")) : select("category", "Kategori", categoryOptions(d.category || "food", kind))}</div></div>${input("occurredAt", "Tanggal & waktu transaksi (WIB)", d.occurredAt || localTime(new Date().toISOString()), "datetime-local", "required")}${input("note", "Catatan (opsional)", d.note || "", "text", 'maxlength="500"')}`;
}
function manual(kind: Kind = "expense", keep = false) {
  if (!keep) {
    manualDraft = {
      walletId: kind === "transfer" ? "bca" : "cash",
      toWalletId: "gopay",
      category: "food",
    };
    formKind = kind;
  }
  sheetId = "manual";
  sheet(
    "Catat uangmu",
    `<form id="manualForm"><div class="form-tabs">${[
      ["expense", "Keluar"],
      ["income", "Masuk"],
      ["transfer", "Transfer"],
    ]
      .map(
        ([k, n]) => `<button type="button" class="${formKind === k ? "active" : ""}" data-act="manual-kind" data-id="${k}">${n}</button>`,
      )
      .join(
        "",
      )}</div>${entryFields(manualDraft, formKind)}${formKind === "transfer" ? '<div class="notice" style="margin-top:12px">Transfer sendiri hanya memindahkan saldo; tidak masuk arus kas pengeluaran.</div>' : ""}${err()}<button class="primary" type="submit">${icon("check")}Simpan transaksi</button></form>`,
  );
}
function transaction(id: string) {
  const t = data.transactions.find((t) => t.id === id)!;
  sheetId = id;
  sheet(
    "Detail transaksi",
    `<div class="detail-amount ${t.kind === "income" ? "income" : ""}">${t.kind === "income" ? "+" : t.kind === "expense" ? "−" : ""}${money(t.amount)}</div><div class="detail-merchant">${esc(t.source === "Email" ? cleanMerchant(t.merchant) : t.merchant)}</div><div class="card">${[["Jenis", t.kind === "transfer" ? "Transfer internal" : t.kind === "income" ? "Pemasukan" : "Pengeluaran"], ["Kategori", cats[t.category]?.name || "Lainnya"], ["Dompet", walletName(data, t.walletId)], ...(t.toWalletId ? [["Tujuan", walletName(data, t.toWalletId)]] : []), ["Waktu aktual", dateText(t.occurredAt) + " WIB"], ["Sumber", t.source], ["Referensi", t.reference || "Manual"], ["Catatan", t.note || "—"]].map(([k, v]) => `<div class="detail-line"><span>${k}</span><span>${esc(v)}</span></div>`).join("")}</div>${t.emailId ? btn("audit", icon("mail") + " Lihat bukti email", "secondary", t.emailId) : ""}<div style="margin-top:12px">${btn("delete-entry", icon("trash") + " Hapus transaksi", "danger-btn", id)}</div>`,
  );
}
function audit(id: string) {
  const e = data.emails.find((e) => e.id === id)!;
  sheetId = id;
  const p = e.parsed,
    canEdit = ["review", "error"].includes(e.status);
  const d = {
    amount: String(p?.amount || ""),
    merchant: cleanMerchant(p?.merchant || ""),
    walletId: p?.walletId || data.accounts.find((a) => a.id === e.accountId)?.walletId || data.wallets[0].id,
    toWalletId: p?.toWalletId || "gopay",
    category: p?.category || "other",
    occurredAt: localTime(p?.occurredAt || e.receivedAt),
    note: "",
  };
  sheet(
    "Bukti & review email",
    `<div class="tag ${canEdit ? "orange" : "gray"}">${esc(e.status)} · UID ${e.uid}</div><div class="strong small" style="margin-top:15px">${esc(e.subject)}</div>${[
      ["Pengirim", e.sender],
      ["Email diterima", dateText(e.receivedAt) + " WIB"],
      ["Referensi bank", p?.reference || "Tidak ditemukan"],
      ["Message-ID", e.messageId],
      ["UIDVALIDITY", String(e.uidValidity)],
      ["Folder", e.mailbox],
    ]
      .map(([k, v]) => `<div class="detail-line"><span>${k}</span><span>${esc(v)}</span></div>`)
      .join(
        "",
      )}<div class="audit-heading">${icon("shield")}Bukti email · hanya teks</div><pre class="raw-email">${esc(e.rawSnippet)}</pre><p class="tiny muted">${esc(e.reason)}</p>${canEdit ? `<form id="reviewForm"><div class="section-title"><h2>Koreksi sebelum dicatat</h2></div>${select("reviewKind", "Jenis mutasi", ["expense", "income", "transfer"].map((k) => `<option value="${k}" ${(p?.kind || "expense") === k ? "selected" : ""}>${k === "expense" ? "Keluar" : k === "income" ? "Masuk" : "Transfer internal"}</option>`).join(""))}<div id="reviewFields">${entryFields(d, p?.kind || "expense")}</div>${err()}<button class="primary" type="submit">${icon("check")}Konfirmasi & catat</button><div style="margin-top:9px">${btn("ignore-email", "Abaikan email", "secondary", id)}</div></form>` : `<div class="notice">Email ini ${e.status === "posted" ? "sudah tercatat" : "tidak mengubah saldo"}.</div>`}`,
    "Saldo hanya berubah saat transaksi dicatat.",
  );
}
function walletForm(id: string | null = null) {
  const w = data.wallets.find((w) => w.id === id);
  sheetId = id || "";
  sheet(
    w ? "Edit dompet" : "Dompet baru",
    `<form id="walletForm">${input("name", "Nama dompet", w?.name || "", "text", 'required maxlength="40"')}${select("kind", "Jenis", ["Bank", "E-Wallet", "Tunai"].map((k) => `<option ${w?.kind === k ? "selected" : ""}>${k}</option>`).join(""))}${input("bank", "Bank / penyedia", w?.bank || "BCA", "text", 'required maxlength="30"')}${input("digits", "4 digit akhir (opsional)", w?.digits || "", "text", 'maxlength="4" pattern="[0-9]{0,4}" inputmode="numeric"')}${input("opening", "Saldo awal", w?.opening || 0, "number", 'required min="0" step="1"')}<div class="notice" style="margin-top:14px">Saldo awal bukan pemasukan. Saldo saat ini akan dihitung bersama seluruh mutasi.</div>${err()}<button class="primary" type="submit">Simpan dompet</button></form>`,
  );
}
function walletDetail(id: string) {
  const w = data.wallets.find((w) => w.id === id)!;
  sheetId = id;
  sheet(
    w.name,
    `<div class="detail-amount">${money(w.balance)}</div><div class="detail-merchant">Saldo hasil perhitungan buku kas</div><div class="card"><div class="detail-line"><span>Saldo awal</span><span>${money(w.opening)}</span></div><div class="detail-line"><span>Saldo teramati</span><span>${w.observed === null ? "Belum tersedia" : money(w.observed)}</span></div><div class="detail-line"><span>Selisih</span><span>${w.observed === null ? "—" : money(w.observed - w.balance)}</span></div></div><form id="reconcileForm">${input("observed", "Saldo pembanding rekening", w.observed ?? w.balance, "number", 'required min="0" step="1"')}<label class="toggle-row"><input name="adjust" type="checkbox"><span>Catat penyesuaian selisih<small>Tambahkan mutasi eksplisit. Biarkan kosong untuk hanya menyimpan pembanding.</small></span></label>${err()}<button class="secondary" type="submit">Simpan rekonsiliasi</button></form><div class="button-row">${btn("wallet-edit", "Edit dompet", "secondary", id)}${btn("wallet-transactions", "Lihat mutasi", "secondary", id)}</div>`,
  );
}
function connection(fresh = true) {
  if (fresh) {
    connectionDraft = {
      email: api.native ? "" : "baru.demo@gmail.com",
      password: api.native ? "" : "abcdefghijklmnop",
      provider: "Auto",
      host: "",
      port: "993",
      walletId: data.wallets[0].id,
      autoPost: "",
      simulation: "success",
    };
    testedKey = "";
    testedReport = null;
  }
  sheetId = "connection";
  const d = connectionDraft;
  sheet(
    "Hubungkan email",
    `<div class="notice ${api.native ? "" : "orange"}">${api.native ? "TCP + TLS langsung dari Rust. App Password hanya disimpan di memori sesi; perlu dimasukkan lagi setelah aplikasi ditutup." : "Mode browser: gunakan data contoh. Tidak ada koneksi IMAP nyata dan kredensial tidak disimpan."}</div><form id="connectionForm">${input("email", "Alamat email", d.email, "email", 'required autocomplete="off"')}${select("provider", "Provider", ["Auto", "Gmail", "Outlook", "Yahoo", "Custom"].map((p) => `<option ${p === d.provider ? "selected" : ""}>${p}</option>`).join(""))}${input("password", "Google App Password · 16 huruf", d.password, "password", 'required autocomplete="off" maxlength="40"')}<div class="row between" style="margin-top:8px"><button class="text-btn" type="button" data-act="wizard">Cara membuat App Password ${icon("chevron")}</button>${!api.native ? '<button class="text-btn" type="button" data-act="fill-demo">Isi demo</button>' : ""}</div><div id="customFields" class="${d.provider === "Custom" ? "" : "hidden"}">${input("host", "Host IMAP", d.host, "text", 'placeholder="imap.example.com"')}${input("port", "Port TLS", d.port, "number", 'min="1" max="65535"')}</div>${select("walletId", "Dompet default", walletOptions(data, d.walletId))}<label class="toggle-row"><input type="checkbox" name="autoPost" ${d.autoPost ? "checked" : ""}><span>Auto-post format yang dikenali<small>Opsional. Sender email bukan bukti autentikasi bank; gunakan review untuk setiap mutasi jika ragu.</small></span></label>${
      !api.native
        ? select(
            "simulation",
            "Skenario uji",
            [
              ["success", "Berhasil"],
              ["auth", "Autentikasi gagal"],
              ["timeout", "Timeout"],
            ]
              .map(([k, n]) => `<option value="${k}" ${d.simulation === k ? "selected" : ""}>${n}</option>`)
              .join(""),
          )
        : ""
    }${err()}<div id="connectionResult"></div><div class="button-row"><button class="secondary" type="button" data-act="test-connection">${icon("sync")}Test Connection</button><button class="primary" type="submit" id="saveConnection" ${testedReport ? "" : "disabled"}>Simpan akun</button></div></form>`,
  );
}
function connectionInput(d: Record<string, string>): ConnectionInput {
  return {
    email: d.email.trim(),
    password: d.password,
    provider: d.provider,
    host: d.host || undefined,
    port: Number(d.port) || 993,
    simulation: d.simulation as ConnectionInput["simulation"],
  };
}
function connectionKey(d: Record<string, string>) {
  return JSON.stringify(d);
}
function wizard() {
  const steps = [
    [
      "shield",
      "Aktifkan Verifikasi 2 Langkah",
      "Buka Akun Google → Keamanan. Aktifkan Verifikasi 2 Langkah dan selesaikan verifikasi akun.",
    ],
    ["lock", "Buat sandi aplikasi", "Buka myaccount.google.com/apppasswords. Beri nama “Arus Mobile”, lalu buat App Password khusus."],
    [
      "mail",
      "Salin 16 huruf",
      "Salin sandi yang baru ditampilkan. Tempel ke form Arus, bukan sandi utama akun Google. Sandi disimpan hanya selama sesi aplikasi.",
    ],
    [
      "check",
      "Uji koneksi dahulu",
      "Pilih dompet default, tekan Test Connection, lalu Simpan akun. Opsi App Password mungkin tidak tersedia untuk akun organisasi atau Advanced Protection.",
    ],
  ];
  const s = steps[wizardStep];
  sheet(
    "Panduan App Password",
    `<div class="wizard-num">LANGKAH ${wizardStep + 1} DARI 4</div><div class="wizard-illustration">${icon(s[0])}</div><h3 class="wizard-title">${s[1]}</h3><p class="wizard-text">${s[2]}</p><p class="tiny muted">Panduan resmi: support.google.com/accounts/answer/185833</p><div class="wizard-dots">${steps.map((_, i) => `<i class="${i <= wizardStep ? "active" : ""}"></i>`).join("")}</div><div class="button-row">${wizardStep ? btn("wizard-prev", "Sebelumnya", "secondary") : ""}${btn(wizardStep === 3 ? "wizard-done" : "wizard-next", wizardStep === 3 ? "Kembali ke setup" : "Lanjut")}</div>`,
  );
}
function accountForm(id: string) {
  const a = data.accounts.find((a) => a.id === id)!;
  sheetId = id;
  const banks = ["BCA", "Mandiri", "BRI", "BNI", "Jenius", "Bank Jago", "Seabank", "GoPay", "DANA", "OVO", "ShopeePay"];
  sheet(
    "Opsi & mapping email",
    `<form id="accountForm"><p class="small muted">${esc(a.email)} · ${esc(a.host)}:${a.port}</p>${select("walletId", "Dompet default", walletOptions(data, a.walletId))}<label class="toggle-row"><input type="checkbox" name="enabled" ${a.enabled ? "checked" : ""}><span>Aktifkan sinkronisasi</span></label><label class="toggle-row"><input type="checkbox" name="autoPost" ${a.autoPost ? "checked" : ""}><span>Auto-post format dikenali<small>Matikan agar setiap mutasi dikonfirmasi dahulu.</small></span></label><div class="notice">Mapping pengirim bank mengalahkan dompet default. Untuk top-up GoPay, petakan BCA dan GoPay ke dua dompet milik sendiri.</div>${banks.map((bank) => select("map_" + bank, bank, '<option value="">Ikuti dompet default</option>' + walletOptions(data, a.mappings.find((m) => m.bank === bank)?.walletId || ""))).join("")}${err()}<button class="primary" type="submit" style="margin-top:15px">Simpan mapping</button></form>`,
  );
}
function unlockForm(id: string) {
  const a = data.accounts.find((a) => a.id === id)!;
  sheetId = id;
  sheet(
    "Buka sesi email",
    `<form id="unlockForm"><div class="notice">${esc(a.email)}<br>App Password tidak disimpan ke disk. Masukkan lagi untuk mengaktifkan koneksi selama sesi ini.</div>${input("password", "App Password", "", "password", 'required autocomplete="off"')}${err()}<button class="primary" type="submit">Uji & buka sesi</button></form>`,
  );
}
function syncBody(p: SyncProgress) {
  const pct = p.total ? Math.round((p.current / p.total) * 100) : 10;
  return `<div class="sync-icon spin">${icon("sync")}</div><h3 class="sync-status">${p.phase === "connect" ? "Menghubungkan…" : "Menyusun buku kas…"}</h3><p class="sync-sub">${esc(p.message)}</p><div class="progress"><span style="width:${pct}%"></span></div><p class="tiny muted" style="text-align:center">${api.native ? "Koneksi langsung dari perangkat" : "Simulasi browser"} · ${p.current}/${p.total || "—"}</p>`;
}
async function sync() {
  if (busy) return;
  busy = true;
  undo = null;
  try {
    sheet(
      "Sinkronisasi email",
      syncBody({
        phase: "connect",
        message: "Menyiapkan akun aktif…",
        accountId: "",
        current: 0,
        total: 0,
      }),
    );
    const r = await api.sync(
      null,
      (p) => {
        if (!$("#overlay").classList.contains("hidden")) $("#sheetBody").innerHTML = syncBody(p);
        const inline = $("#syncInline");
        if (inline)
          inline.innerHTML = `<div class="sync-inline"><p class="tiny muted">${esc(p.message)}</p><div class="progress"><span style="width:${p.total ? Math.round((p.current / p.total) * 100) : 10}%"></span></div></div>`;
      },
      getSyncRange(),
    );
    await refresh();
    sheet(
      "Sinkronisasi selesai",
      `<div class="sync-icon">${icon("check")}</div><h3 class="sync-status">${r.fetched ? "Uangmu semakin tertata." : r.warnings.length ? "Sinkronisasi perlu perhatian." : "Sudah paling baru."}</h3><p class="sync-sub">${r.fetched} email diproses · ${r.remaining} menunggu batch berikut</p><div class="sync-grid"><div class="sync-stat"><b>${r.posted}</b><small>tercatat</small></div><div class="sync-stat"><b>${r.review}</b><small>perlu review</small></div><div class="sync-stat"><b>${r.emailDuplicates + r.ledgerDuplicates}</b><small>duplikat dicegah</small></div></div><div class="card"><div class="detail-line"><span>Message-ID / GMID</span><span>${r.emailDuplicates}</span></div><div class="detail-line"><span>Referensi bank</span><span>${r.ledgerDuplicates}</span></div><div class="detail-line"><span>Diabaikan / error</span><span>${r.ignored} / ${r.errors}</span></div></div>${r.warnings.map((w) => `<div class="notice orange">${esc(w)}</div>`).join("")}${btn("go-review", "Buka kotak review")}${r.remaining ? `<div style="margin-top:10px">${btn("sync", "Lanjutkan batch berikut", "secondary")}</div>` : ""}`,
    );
  } catch (e) {
    await refresh();
    sheet(
      "Sinkronisasi belum selesai",
      `<div class="notice orange">${esc(errorText(e))}</div><div class="notice">Mutasi yang sudah berhasil diproses tetap tersimpan. Ulangi sync untuk melanjutkan dari kursor terakhir.</div>${btn("go-settings", "Buka pengaturan", "primary")}`,
    );
  } finally {
    busy = false;
  }
}
function analytics() {
  const ts = data.transactions.filter(
      (t) => t.occurredAt.startsWith(view.month) && (view.wallet === "all" || t.walletId === view.wallet || t.toWalletId === view.wallet),
    ),
    ins = ts.filter((t) => t.kind === "income").reduce((n, t) => n + t.amount, 0),
    outs = ts.filter((t) => t.kind === "expense").reduce((n, t) => n + t.amount, 0),
    groups = Object.entries(cats)
      .filter(([k]) => !["income", "transfer"].includes(k))
      .map(([k, c]) => ({
        ...c,
        amount: ts.filter((t) => t.kind === "expense" && t.category === k).reduce((n, t) => n + t.amount, 0),
      }))
      .filter((c) => c.amount);
  let start = 0;
  const colors = groups
    .map((c) => {
      const a = start;
      start += (c.amount / outs) * 100;
      return `${c.color} ${a}% ${start}%`;
    })
    .join(",");
  sheet(
    "Gambaran keuangan",
    `<div class="card"><div class="tiny muted">Arus kas bersih · ${view.month}</div><div class="detail-amount">${money(ins - outs)}</div></div><div class="card chart-card">${chart(data, view)}</div><div class="card">${outs ? `<div class="donut" style="background:conic-gradient(${colors})"><div class="donut-center">Total keluar<b>${money(outs)}</b></div></div>${groups.map((c) => `<div class="category-item"><i class="category-dot" style="background:${c.color}"></i><span class="grow">${c.name}</span><b>${money(c.amount)}</b><small>${Math.round((c.amount / outs) * 100)}%</small></div>`).join("")}` : empty("Belum ada pengeluaran", "Tambahkan transaksi terlebih dahulu.")}</div><div class="notice">Transfer internal dikecualikan dari grafik pemasukan dan pengeluaran.</div>`,
  );
}
function exportCsv() {
  const protect = (v: unknown) => {
    let s = String(v ?? "");
    if (/^[=+\-@\t\r]/.test(s)) s = "'" + s;
    return '"' + s.replace(/"/g, '""') + '"';
  };
  const lines = [
    ["Tanggal", "Merchant", "Jenis", "Nominal Rupiah", "Dompet", "Tujuan", "Kategori", "Sumber", "Referensi", "Catatan"],
    ...data.transactions
      .filter((t) => t.occurredAt.startsWith(view.month))
      .map((t) => [
        t.occurredAt,
        t.merchant,
        t.kind,
        t.amount,
        walletName(data, t.walletId),
        walletName(data, t.toWalletId),
        cats[t.category]?.name,
        t.source,
        t.reference,
        t.note,
      ]),
  ];
  const url = URL.createObjectURL(
    new Blob(["\uFEFF" + lines.map((r) => r.map(protect).join(",")).join("\r\n")], { type: "text/csv;charset=utf-8" }),
  );
  const a = document.createElement("a");
  a.href = url;
  a.download = "Arus_Buku_Kas_" + view.month + ".csv";
  a.click();
  setTimeout(() => URL.revokeObjectURL(url), 2000);
  toast("Laporan CSV disiapkan untuk diunduh.");
}
function syncOptions() {
  sheet(
    "Pilih periode email",
    `<p class="small muted">Pilih rentang waktu email yang ingin diperiksa. Email yang sudah dicatat tidak akan dicatat ganda.</p>${select("syncRange", "Rentang waktu", syncRangeOptions())}<div class="notice">Semua email diproses bertahap, maksimal 200 email per akun dalam satu proses.</div>${btn("sync-start", "Mulai sinkronisasi", "primary")}`
  );
}

function showHelp() {
  sheet(
    "Tentang Arus",
    `<div class="wizard-illustration">${icon("shield")}</div><h3 class="wizard-title">Keuangan tertata, privasi terjaga.</h3><p class="small">Arus membantu Anda mencatat pemasukan, pengeluaran, dan mutasi secara otomatis langsung di perangkat Anda.</p><div class="notice">${api.native ? "Catatan keuangan Anda tersimpan di ponsel dan tidak dikirim ke server pengembang." : "Ini adalah pratinjau dengan data contoh. Email asli tidak dihubungkan."}</div><div class="card"><div class="detail-line"><span>Catatan Anda</span><span>Tersimpan di perangkat</span></div><div class="detail-line"><span>Email Anda</span><span>Tetap aman & belum dibaca</span></div><div class="detail-line"><span>Kata sandi email</span><span>Hanya selama sesi aktif (RAM)</span></div><div class="detail-line"><span>Versi Arus</span><span>v0.3.4</span></div></div><p class="small muted">Koneksi internet hanya digunakan saat mengambil email atau memeriksa pembaruan.</p>`,
  );
}
let press: { id: string; pointer: number; x: number; y: number; timer: ReturnType<typeof setTimeout> } | null = null;
let suppressedCard = "",
  suppressUntil = 0;
function cancelPress() {
  if (press) clearTimeout(press.timer);
  press = null;
}
function clearSelection() {
  view.selectionMode = false;
  view.selectedIds = new Set();
  cancelPress();
}
function toggleReview(id: string, force = false) {
  if (busy || !data.emails.some((e) => e.id === id && e.status === "review")) return;
  view.selectionMode = true;
  const ids = new Set(view.selectedIds);
  if (force || !ids.has(id)) ids.add(id);
  else ids.delete(id);
  view.selectedIds = ids;
  render(true);
}
document.addEventListener("pointerdown", (e) => {
  const card = (e.target as Element).closest<HTMLElement>('[data-selectable="true"]');
  if (
    !card ||
    busy ||
    view.page !== "review" ||
    view.status !== "review" ||
    e.button !== 0 ||
    (e.target as Element).closest('[role="checkbox"]')
  )
    return;
  cancelPress();
  const id = card.dataset.reviewId!;
  press = {
    id,
    pointer: e.pointerId,
    x: e.clientX,
    y: e.clientY,
    timer: setTimeout(() => {
      suppressedCard = id;
      suppressUntil = performance.now() + 800;
      press = null;
      toggleReview(id, true);
    }, 500),
  };
});
document.addEventListener("pointermove", (e) => {
  if (press && press.pointer === e.pointerId && Math.hypot(e.clientX - press.x, e.clientY - press.y) > 10) cancelPress();
});
for (const event of ["pointerup", "pointercancel", "lostpointercapture"]) {
  document.addEventListener(event, () => cancelPress());
}
document.addEventListener("scroll", () => cancelPress(), true);
window.addEventListener("blur", cancelPress);
document.addEventListener("contextmenu", (e) => {
  if ((e.target as Element).closest('[data-selectable="true"]')) e.preventDefault();
});
async function resolveBatch(action: "post" | "ignore", all = false) {
  const ids = data.emails.filter((e) => e.status === "review" && (all || view.selectedIds?.has(e.id))).map((e) => e.id);
  if (!ids.length) {
    toast("Pilih setidaknya satu transaksi.");
    return;
  }
  if (busy) return;
  busy = true;
  view.reviewBusy = true;
  undo = null;
  cancelPress();
  render(true);
  try {
    const r = await api.batchResolve({ items: ids.map((emailId) => ({ emailId, action, entry: null })) });
    clearSelection();
    view.reviewBusy = false;
    await refresh();
    toast(
      `${r.posted + r.updated} dicatat, ${r.ignored} diabaikan, ${r.duplicates} bukti duplikat, ${r.unchanged} sudah diproses.${r.referenceConflicts ? ` ${r.referenceConflicts} referensi berulang tetap disimpan di bukti email.` : ""}`,
    );
  } catch (e) {
    view.reviewBusy = false;
    render(true);
    toast(errorText(e));
  } finally {
    busy = false;
    view.reviewBusy = false;
  }
}
document.addEventListener("click", async (e) => {
  const el = (e.target as Element).closest<HTMLElement>("[data-act],[data-page]");
  if (!el) {
    if (e.target === $("#overlay")) close();
    return;
  }
  if (el instanceof HTMLButtonElement && el.disabled) return;
  const reviewCard = el.closest<HTMLElement>("[data-review-id]");
  if (reviewCard?.dataset.reviewId === suppressedCard && performance.now() < suppressUntil) {
    e.preventDefault();
    return;
  }
  if (busy && el.dataset.page) {
    toast("Tunggu sampai proses selesai.");
    return;
  }
  if (el.dataset.page) {
    clearSelection();
    view.page = el.dataset.page as Page;
    close();
    render();
    return;
  }
  const act = el.dataset.act!,
    id = el.dataset.id || "";
  if (busy && !["close", "dismiss", "go-settings"].includes(act)) {
    toast("Tunggu sampai proses selesai.");
    return;
  }
  try {
    switch (act) {
      case "close":
        close();
        break;
      case "dismiss":
        $("#toast").classList.add("hidden");
        break;
      case "undo": {
        const f = undo;
        undo = null;
        if (f) {
          await f();
          await refresh();
          toast("Perubahan dibatalkan.");
        }
        break;
      }
      case "hide-balance":
        view.hidden = !view.hidden;
        render();
        break;
      case "help":
        showHelp();
        break;
      case "reload":
        location.reload();
        break;
      case "manual":
        manual();
        break;
      case "transfer":
        manual("transfer");
        break;
      case "manual-kind":
        manualDraft = { ...manualDraft, ...entries($("#manualForm")) };
        formKind = id as Kind;
        manual(formKind, true);
        break;
      case "transaction":
        transaction(id);
        break;
      case "delete-entry":
        await api.deleteEntry(id, false);
        close();
        await refresh();
        toast("Transaksi dihapus.", () => api.deleteEntry(id, true));
        break;
      case "audit":
        if (view.selectionMode && view.status === "review" && data.emails.some((e) => e.id === id && e.status === "review"))
          toggleReview(id);
        else audit(id);
        break;
      case "ignore-email":
        await api.resolve({ emailId: id, action: "ignore", entry: null });
        close();
        await refresh();
        toast("Email diabaikan.", () => api.undoReview(id));
        break;
      case "start-selection":
        view.selectionMode = true;
        render(true);
        break;
      case "cancel-selection":
        clearSelection();
        render(true);
        break;
      case "toggle-review":
        toggleReview(id);
        break;
      case "select-all-review":
        view.selectedIds = new Set(data.emails.filter((e) => e.status === "review").map((e) => e.id));
        view.selectionMode = true;
        render(true);
        break;
      case "batch-post-all":
        await resolveBatch("post", true);
        break;
      case "batch-post-selected":
        await resolveBatch("post");
        break;
      case "batch-ignore-selected":
        await resolveBatch("ignore");
        break;
      case "kind-filter":
        view.kind = id;
        render();
        break;
      case "status-filter":
        clearSelection();
        view.status = id;
        render();
        break;
      case "filters":
        sheet(
          "Filter transaksi",
          `<form id="filterForm">${select("wallet", "Dompet", walletOptions(data, view.wallet, true))}${select("category", "Kategori", categoryOptions(view.category, "all", true))}<div class="button-row">${btn("reset-filters", "Reset", "secondary")}<button type="submit" class="primary">Terapkan</button></div></form>`,
        );
        break;
      case "reset-filters":
        view.wallet = "all";
        view.category = "all";
        view.kind = "all";
        view.search = "";
        close();
        render();
        break;
      case "wallet-new":
        walletForm();
        break;
      case "wallet-edit":
        walletForm(id);
        break;
      case "wallet-detail":
        walletDetail(id);
        break;
      case "wallet-transactions":
        view.page = "transactions";
        view.wallet = id;
        view.kind = "all";
        view.category = "all";
        close();
        render();
        break;
      case "analytics":
        analytics();
        break;
      case "chart-day": {
        const ts = data.transactions.filter(
          (t) =>
            t.occurredAt.startsWith(view.month) &&
            Number(t.occurredAt.slice(8, 10)) === Number(id) &&
            (view.wallet === "all" || t.walletId === view.wallet || t.toWalletId === view.wallet),
        );
        sheet(
          "Arus kas · tanggal " + id,
          ts.map((t) => row(data, t)).join("") || empty("Belum ada transaksi", "Tidak ada transaksi pada hari ini."),
        );
        break;
      }
      case "connect":
        connection();
        break;
      case "fill-demo":
        connectionDraft = {
          email: "baru.demo@gmail.com",
          password: "abcdefghijklmnop",
          provider: "Auto",
          host: "",
          port: "993",
          walletId: data.wallets[0].id,
          autoPost: "",
          simulation: "success",
        };
        testedReport = null;
        testedKey = "";
        connection(false);
        break;
      case "wizard":
        connectionDraft = entries($("#connectionForm"));
        wizardStep = 0;
        wizard();
        break;
      case "wizard-prev":
        wizardStep--;
        wizard();
        break;
      case "wizard-next":
        wizardStep++;
        wizard();
        break;
      case "wizard-done":
        connection(false);
        break;
      case "test-connection": {
        const form = $<HTMLFormElement>("#connectionForm");
        if (!form.reportValidity()) return;
        const draft = entries(form);
        connectionDraft = draft;
        const key = connectionKey(draft);
        const b = el as HTMLButtonElement;
        b.disabled = true;
        b.textContent = "Menguji…";
        $("#formError").textContent = "";
        try {
          const report = await api.test(connectionInput(draft));
          if (!$("#connectionForm") || key !== connectionKey(entries($("#connectionForm")))) return;
          testedKey = key;
          testedReport = report;
          $("#connectionResult").innerHTML =
            `<div class="test-output">✓ TLS ${esc(report.host)}:${report.port}<br>✓ Autentikasi berhasil${api.native ? "" : " (simulasi)"}<br>✓ Folder ${esc(report.folder)}<br>${report.elapsedMs} ms · read-only</div>`;
          $<HTMLButtonElement>("#saveConnection").disabled = false;
        } catch (e) {
          testedReport = null;
          fail(e);
        } finally {
          b.disabled = false;
          b.innerHTML = icon("sync") + "Test Connection";
        }
        break;
      }
      case "account":
        accountForm(id);
        break;
      case "unlock-account":
        unlockForm(id);
        break;
      case "lock-account":
        await api.lock(id);
        await refresh();
        toast("Sesi email ditutup; sandi di memori dilepas.");
        break;
      case "theme":
        await api.theme(data.theme === "dark" ? "light" : "dark");
        await refresh();
        break;
      case "sync":
        syncOptions();
        break;
      case "sync-start":
        await sync();
        break;
      case "go-review":
        view.page = "review";
        view.status = "review";
        close();
        render();
        break;
      case "go-settings":
        view.page = "settings";
        close();
        render();
        break;
      case "export":
        exportCsv();
        break;
    }
  } catch (e) {
    fail(e);
  }
});
document.addEventListener("input", (e) => {
  const target = e.target as HTMLInputElement;
  if (target.id === "searchInput") {
    view.search = target.value;
    $("#transactionRows").innerHTML = transactionRows(data, view);
    $("#resultCount").textContent = filtered(data, view).length + " transaksi";
  }
  if (target.closest("#connectionForm")) invalidateConnection();
  if (target.id === "f-merchant" && $("#manualForm") && formKind === "expense") {
    const c = classifyMerchant(target.value);
    if (c !== "other") $<HTMLSelectElement>("#f-category").value = c;
  }
});
function invalidateConnection() {
  testedReport = null;
  testedKey = "";
  const save = $<HTMLButtonElement>("#saveConnection");
  if (save) save.disabled = true;
  const r = $("#connectionResult");
  if (r) r.innerHTML = "";
}
document.addEventListener("change", (e) => {
  const target = e.target as HTMLInputElement | HTMLSelectElement;
  if (target.id === "monthSelect") {
    view.month = target.value;
    render();
  }
  if (target.id === "walletSelect") {
    view.wallet = target.value;
    render();
  }
  if (target.name === "syncRange") {
    setSyncRange(target.value);
  }
  if (target.id === "f-provider") {
    $("#customFields").classList.toggle("hidden", target.value !== "Custom");
    invalidateConnection();
  }
  if (target.closest("#connectionForm")) invalidateConnection();
  if (target.id === "f-reviewKind") {
    const d = entries($("#reviewForm"));
    $("#reviewFields").innerHTML = entryFields(d, target.value as Kind);
  }
});
document.addEventListener("submit", async (e) => {
  e.preventDefault();
  const form = e.target as HTMLFormElement;
  if (busy || !form.reportValidity()) return;
  const d = entries(form),
    submit = form.querySelector<HTMLButtonElement>('button[type="submit"]');
  if (submit) submit.disabled = true;
  try {
    switch (form.id) {
      case "manualForm": {
        const entry = entryFrom(d);
        const id = await api.manual(entry);
        close();
        await refresh();
        toast("Transaksi tercatat. Saldo diperbarui.", () => api.deleteEntry(id, false));
        break;
      }
      case "reviewForm": {
        const id = sheetId;
        const input: ReviewInput = {
          emailId: id,
          action: "post",
          entry: entryFrom(d),
        };
        await api.resolve(input);
        close();
        await refresh();
        toast(
          data.emails.find((e) => e.id === id)?.status === "duplicate"
            ? "Bukti transaksi identik sudah tercatat; saldo tidak digandakan."
            : "Transaksi dikonfirmasi dan tercatat.",
          () => api.undoReview(id),
        );
        break;
      }
      case "walletForm":
        await api.wallet({
          id: sheetId || null,
          name: d.name,
          kind: d.kind,
          bank: d.bank,
          digits: d.digits,
          opening: Number(d.opening),
        });
        close();
        await refresh();
        toast("Dompet disimpan.");
        break;
      case "reconcileForm":
        await api.reconcile(sheetId, Number(d.observed), d.adjust === "on");
        {
          const id = sheetId;
          await refresh();
          walletDetail(id);
          toast("Rekonsiliasi disimpan.");
        }
        break;
      case "filterForm":
        view.wallet = d.wallet;
        view.category = d.category;
        close();
        render();
        break;
      case "connectionForm":
        if (!testedReport || testedKey !== connectionKey(d))
          throw Error("Lakukan Test Connection yang berhasil setelah perubahan terakhir.");
        await api.connect(connectionInput(d), d.walletId, d.autoPost === "on");
        connectionDraft = {};
        testedKey = "";
        testedReport = null;
        close();
        await refresh();
        toast("Email terhubung. App Password hanya di memori sesi.");
        break;
      case "unlockForm":
        await api.unlock(sheetId, d.password);
        close();
        await refresh();
        toast("Sesi aktif. Email siap disinkronkan.");
        break;
      case "accountForm": {
        const update: AccountUpdate = {
          id: sheetId,
          walletId: d.walletId,
          enabled: d.enabled === "on",
          autoPost: d.autoPost === "on",
          mappings: Object.entries(d)
            .filter(([k, v]) => k.startsWith("map_") && v)
            .map(([k, v]) => ({ bank: k.slice(4), walletId: v })),
        };
        await api.updateAccount(update);
        close();
        await refresh();
        toast("Opsi dan mapping email disimpan.");
        break;
      }
    }
  } catch (e) {
    fail(e);
  } finally {
    if (submit) submit.disabled = false;
  }
});
document.addEventListener("keydown", (e) => {
  const card = (e.target as Element).closest<HTMLElement>('[data-selectable="true"]');
  if (card && e.key === " " && view.page === "review" && view.status === "review") {
    e.preventDefault();
    if (!e.repeat) toggleReview(card.dataset.reviewId!);
    return;
  }

  if (!$("#overlay") || $("#overlay").classList.contains("hidden")) return;
  if (e.key === "Escape") {
    close();
    return;
  }
  if (e.key === "Tab") {
    const nodes = [...$("#sheet").querySelectorAll<HTMLElement>("button,input,select,textarea,a[href]")].filter(
      (n) => !(n as HTMLButtonElement).disabled && n.getClientRects().length,
    );
    if (!nodes.length) {
      e.preventDefault();
      return;
    }
    const first = nodes[0],
      last = nodes[nodes.length - 1];
    if (e.shiftKey && (document.activeElement === first || document.activeElement === $("#sheet"))) {
      e.preventDefault();
      last.focus();
    } else if (!e.shiftKey && document.activeElement === last) {
      e.preventDefault();
      first.focus();
    }
  }
});
installSheetGesture(close);
$("#app").innerHTML =
  '<div class="phone p-6"><div class="skeleton mt-20"></div><p class="text-center muted mt-8">Membuka buku kas lokal…</p></div>';
refresh().catch((e) => {
  $("#app").innerHTML =
    `<div class="p-8"><h1>Arus belum bisa dibuka</h1><p>${esc(errorText(e))}</p><button data-act="reload">Muat ulang</button></div>`;
});
