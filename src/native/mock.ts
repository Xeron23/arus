import { appVersion } from "../version";
import type {
  FinanceAPI,
  Snapshot,
  EntryInput,
  LedgerEntry,
  EmailRecord,
  ConnectionInput,
  ConnectionReport,
  Account,
  SyncReport,
} from "../models";
export const validAmount = (n: number) =>
  Number.isSafeInteger(n) && n > 0 && n <= 100_000_000_000_000;
export function validateEntry(e: EntryInput, s: Snapshot) {
  if (
    !validAmount(e.amount) ||
    !e.merchant.trim() ||
    Number.isNaN(Date.parse(e.occurredAt))
  )
    throw Error("Nominal Rupiah bulat, merchant, dan waktu harus valid.");
  if (!s.wallets.some((w) => w.id === e.walletId))
    throw Error("Dompet tidak ditemukan.");
  if (
    e.kind === "transfer" &&
    (!s.wallets.some((w) => w.id === e.toWalletId) ||
      e.walletId === e.toWalletId)
  )
    throw Error("Pilih dompet tujuan yang berbeda.");
  if (e.kind === "expense" && ["income", "transfer"].includes(e.category))
    throw Error("Kategori pengeluaran tidak valid.");
}
export function computeWallets(s: Snapshot, deleted: string[] = []) {
  return s.wallets.map((w) => ({
    ...w,
    balance:
      w.opening +
      s.transactions
        .filter((t) => !deleted.includes(t.id))
        .reduce(
          (n, t) =>
            n +
            (t.walletId === w.id
              ? t.kind === "income"
                ? t.amount
                : -t.amount
              : 0) +
            (t.kind === "transfer" && t.toWalletId === w.id ? t.amount : 0),
          0,
        ),
  }));
}
export function classifyMerchant(name: string) {
  return /indomaret|alfamart|superindo|tokopedia/i.test(name)
    ? "shop"
    : /pertamina|spbu|grab|gojek/i.test(name)
      ? "transport"
      : /warteg|kopi|coffee|bakmi|resto|makan/i.test(name)
        ? "food"
        : /pln|sewa|internet/i.test(name)
          ? "home"
          : "other";
}
const id = () => {
  if (typeof crypto.randomUUID === "function") return crypto.randomUUID();
  const b = crypto.getRandomValues(new Uint8Array(16));
  b[6] = (b[6] & 15) | 64;
  b[8] = (b[8] & 63) | 128;
  const h = [...b].map((n) => n.toString(16).padStart(2, "0")).join("");
  return `${h.slice(0, 8)}-${h.slice(8, 12)}-${h.slice(12, 16)}-${h.slice(16, 20)}-${h.slice(20)}`;
};
const day = () =>
  new Intl.DateTimeFormat("en-CA", {
    timeZone: "Asia/Jakarta",
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  }).format(new Date());
export function seedSnapshot(): Snapshot {
  const month = day().slice(0, 7);
  const date = (d: number) =>
    `${month}-${String(Math.min(d, Number(day().slice(-2)))).padStart(2, "0")}T12:30:00+07:00`;
  const wallets = [
    {
      id: "bca",
      name: "BCA Utama",
      kind: "Bank",
      bank: "BCA",
      digits: "4821",
      opening: 1250000,
      balance: 0,
      observed: null,
    },
    {
      id: "mandiri",
      name: "Mandiri Tabungan",
      kind: "Bank",
      bank: "Mandiri",
      digits: "9012",
      opening: 1500000,
      balance: 0,
      observed: null,
    },
    {
      id: "gopay",
      name: "GoPay",
      kind: "E-Wallet",
      bank: "GoPay",
      digits: "8721",
      opening: 250000,
      balance: 0,
      observed: null,
    },
    {
      id: "cash",
      name: "Uang Tunai",
      kind: "Tunai",
      bank: "Cash",
      digits: "",
      opening: 300000,
      balance: 0,
      observed: null,
    },
  ];
  const rows: [string, number, "income" | "expense", string, string, number][] =
    [
      ["Gaji bulanan", 10000000, "income", "income", "bca", 1],
      ["Sewa apartemen", 2600000, "expense", "home", "bca", 2],
      ["PLN Pascabayar", 425000, "expense", "home", "bca", 3],
      ["Indomaret", 187500, "expense", "shop", "bca", 4],
      ["Pertamina", 250000, "expense", "transport", "bca", 5],
      ["Freelance desain", 2500000, "income", "income", "mandiri", 7],
      ["Superindo", 365000, "expense", "shop", "mandiri", 9],
      ["Internet rumah", 350000, "expense", "home", "bca", 10],
      ["Bakmi GM", 165000, "expense", "food", "bca", 12],
      ["Kopi Kenangan", 42000, "expense", "food", "gopay", 15],
      ["Netflix", 186000, "expense", "fun", "bca", 16],
      ["Alfamart", 89500, "expense", "shop", "bca", 18],
      ["Makan bersama", 325000, "expense", "food", "bca", 20],
      ["GrabCar", 78000, "expense", "transport", "gopay", 22],
      ["Warteg Bahari", 28000, "expense", "food", "cash", 23],
      ["Tokopedia", 235000, "expense", "shop", "mandiri", 24],
      ["Kopi Tuku", 46000, "expense", "food", "gopay", 25],
      ["Indomaret", 67500, "expense", "shop", "bca", 26],
      ["Nasi goreng Pak Budi", 32000, "expense", "food", "cash", 26],
    ];
  const transactions: LedgerEntry[] = rows.map((r, i) => ({
    id: "seed-" + i,
    merchant: r[0],
    amount: r[1],
    kind: r[2],
    category: r[3],
    walletId: r[4],
    occurredAt: date(r[5]),
    toWalletId: null,
    note: "Data contoh browser",
    source: r[4] === "cash" ? "Manual" : "Email",
    reference: "SEED-" + i,
    bank: r[4],
    emailId: null,
  }));
  transactions.push({
    id: "seed-transfer",
    merchant: "Top up GoPay",
    amount: 300000,
    kind: "transfer",
    category: "transfer",
    walletId: "bca",
    toWalletId: "gopay",
    occurredAt: date(26),
    source: "Manual",
    reference: null,
    bank: "BCA",
    emailId: null,
    note: "Pindah saldo sendiri",
  });
  const accounts: Account[] = [
    {
      id: "a1",
      email: "utama.demo@gmail.com",
      provider: "Gmail",
      host: "imap.gmail.com",
      port: 993,
      walletId: "bca",
      enabled: true,
      autoPost: true,
      folder: "All Mail",
      lastSync: null,
      unlocked: true,
      mappings: [
        { bank: "BCA", walletId: "bca" },
        { bank: "GoPay", walletId: "gopay" },
      ],
    },
  ];
  const s: Snapshot = {
    wallets,
    transactions,
    emails: [],
    accounts,
    theme: "light",
  };
  s.wallets = computeWallets(s);
  s.wallets[0].observed = s.wallets[0].balance - 25000;
  const em = makeMail(
    "seed-review",
    125000,
    "PT XYZ 009182",
    "review",
    accounts[0],
  );
  s.emails.push(em);
  return s;
}
function makeMail(
  msg: string,
  amount: number,
  merchant: string,
  status: EmailRecord["status"],
  a: Account,
  ref = "BCA-" + msg,
): EmailRecord {
  const date = day() + "T10:42:00+07:00";
  return {
    id: msg,
    accountId: a.id,
    mailbox: "[Gmail]/All Mail",
    uid: 1051,
    uidValidity: 20261001,
    messageId: msg,
    gmId: "gm-" + msg,
    sender: "notifikasi@bca.co.id",
    subject: "Notifikasi pembayaran QRIS",
    receivedAt: day() + "T10:43:00+07:00",
    rawSnippet: `Notifikasi BCA\nBERHASIL\nPEMBAYARAN QRIS: ${merchant}\nNominal: Rp${amount.toLocaleString("id-ID")},00\nTanggal: ${day()} 10:42 WIB\nNo. Referensi: ${ref}`,
    status,
    reason:
      status === "review"
        ? "Nama merchant / kategori perlu dikonfirmasi."
        : "Format valid.",
    parsed: {
      bank: "BCA",
      amount,
      kind: "expense",
      merchant,
      category: classifyMerchant(merchant),
      occurredAt: date,
      reference: ref,
      observed: null,
      walletId: a.walletId,
      toWalletId: null,
      confident: true,
    },
  };
}
interface Store {
  snapshot: Snapshot;
  deleted: string[];
  seen: string[];
}
const pause = (ms: number) => new Promise((r) => setTimeout(r, ms));
export class MockAPI implements FinanceAPI {
  readonly native = false;
  private data: Store;
  private ready: Promise<void>;
  private syncing = false;
  constructor(private persist = true) {
    this.data = { snapshot: seedSnapshot(), deleted: [], seen: [] };
    this.ready = persist ? this.read() : Promise.resolve();
  }
  private async read() {
    const db = await this.database();
    this.data = await new Promise<Store>((resolve, reject) => {
      const tx = db.transaction("state", "readonly"),
        r = tx.objectStore("state").get("main");
      r.onsuccess = () => resolve(r.result || this.data);
      r.onerror = () => reject(r.error);
    });
    db.close();
  }
  private database(): Promise<IDBDatabase> {
    return new Promise((resolve, reject) => {
      const r = indexedDB.open("arus-browser-demo-v1", 1);
      r.onupgradeneeded = () => r.result.createObjectStore("state");
      r.onsuccess = () => resolve(r.result);
      r.onerror = () =>
        reject(Error("Penyimpanan IndexedDB tidak tersedia pada browser ini."));
    });
  }
  private async save() {
    if (!this.persist) return;
    const db = await this.database();
    await new Promise<void>((resolve, reject) => {
      const tx = db.transaction("state", "readwrite");
      tx.objectStore("state").put(this.data, "main");
      tx.oncomplete = () => resolve();
      tx.onerror = () => reject(tx.error);
      tx.onabort = () => reject(tx.error);
    });
    db.close();
  }
  private get s() {
    return this.data.snapshot;
  }
  async snapshot() {
    await this.ready;
    return structuredClone({
      ...this.s,
      wallets: computeWallets(this.s, this.data.deleted),
      transactions: this.s.transactions.filter(
        (t) => !this.data.deleted.includes(t.id),
      ),
    });
  }
  async test(input: ConnectionInput): Promise<ConnectionReport> {
    if (
      !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(input.email) ||
      input.password.replace(/\s/g, "").length !== 16
    )
      throw Error("Gunakan email valid dan App Password demo 16 huruf.");
    await pause(this.persist ? 650 : 0);
    if (input.simulation === "auth")
      throw Error("AUTHENTICATION_FAILED · simulasi login ditolak.");
    if (input.simulation === "timeout")
      throw Error("TCP_TIMEOUT · simulasi jaringan habis waktu.");
    const p = input.provider === "Auto" ? "Gmail" : input.provider;
    return {
      host: input.host || "imap.gmail.com",
      port: input.port || 993,
      provider: p,
      folder: "[Gmail]/All Mail",
      elapsedMs: 650,
      tls: true,
    };
  }
  async connect(input: ConnectionInput, walletId: string, autoPost: boolean) {
    await this.ready;
    const report = await this.test(input);
    let a = this.s.accounts.find((a) => a.email === input.email);
    if (!a) {
      a = {
        id: id(),
        email: input.email,
        provider: report.provider,
        host: report.host,
        port: report.port,
        walletId,
        autoPost,
        enabled: true,
        unlocked: true,
        folder: report.folder,
        lastSync: null,
        mappings: [],
      };
      this.s.accounts.push(a);
    } else {
      a.unlocked = true;
      a.enabled = true;
      a.walletId = walletId;
      a.autoPost = autoPost;
    }
    await this.save();
    return a.id;
  }
  async unlock(accountId: string, password: string) {
    await this.ready;
    const a = this.s.accounts.find((a) => a.id === accountId)!;
    const r = await this.test({
      email: a.email,
      password,
      provider: a.provider,
    });
    a.unlocked = true;
    await this.save();
    return r;
  }
  async lock(accountId: string) {
    await this.ready;
    this.s.accounts.find((a) => a.id === accountId)!.unlocked = false;
    await this.save();
  }
  async updateAccount(input: Parameters<FinanceAPI["updateAccount"]>[0]) {
    await this.ready;
    Object.assign(this.s.accounts.find((a) => a.id === input.id)!, input);
    await this.save();
  }
  async sync(
    accountId: string | null,
    onProgress: Parameters<FinanceAPI["sync"]>[1],
    range: import("../models").SyncRange = "30",
  ): Promise<SyncReport> {
    await this.ready;
    if (this.syncing) throw Error("Sinkronisasi sedang berjalan.");
    this.syncing = true;
    try {
      const a = this.s.accounts.find(
        (a) => a.enabled && a.unlocked && (!accountId || a.id === accountId),
      );
      if (!a)
        throw Error("Aktifkan dan buka sesi satu akun email terlebih dahulu.");
      const r: SyncReport = {
        fetched: 8,
        posted: 0,
        review: 0,
        ignored: 0,
        errors: 0,
        emailDuplicates: 0,
        ledgerDuplicates: 0,
        remaining: 0,
        warnings: [],
      };
      onProgress({
        phase: "connect",
        message: `Simulasi email ${range === "all" ? "semua periode" : range + " hari terakhir"}…`,
        accountId: a.id,
        current: 0,
        total: 8,
      });
      await pause(this.persist ? 500 : 0);
      const coffee = makeMail(
          "sync-kopi",
          45000,
          "Kopi Nusa",
          a.autoPost ? "posted" : "review",
          a,
        ),
        unknown = makeMail("sync-unknown", 185000, "PT ABC 99102", a.autoPost ? "posted" : "review", a),
        summary = makeMail(
          "sync-summary",
          45000,
          "Kopi Nusa",
          a.autoPost ? "posted" : "review",
          a,
          coffee.parsed!.reference!,
        ),
        promo = makeMail("sync-promo", 0, "Promo", "ignored", a),
        err = makeMail("sync-error", 0, "Format baru", "error", a),
        transfer = makeMail(
          "sync-transfer",
          150000,
          "Top up GoPay",
          "review",
          a,
        ),
        income = makeMail(
          "sync-income",
          350000,
          "Pengembalian dana",
          a.autoPost ? "posted" : "review",
          a,
        );
      promo.parsed = null;
      err.parsed = null;
      promo.rawSnippet = "Promo diskon 20%";
      err.rawSnippet = "Transaksi berhasil. Nominal ada di lampiran.";
      transfer.parsed!.kind = "transfer";
      transfer.parsed!.category = "transfer";
      transfer.parsed!.toWalletId = "gopay";
      income.parsed!.kind = "income";
      income.parsed!.category = "income";
      const feed = [
        coffee,
        unknown,
        summary,
        { ...coffee },
        promo,
        err,
        transfer,
        income,
      ];
      for (let i = 0; i < feed.length; i++) {
        const e = feed[i];
        e.uid = 1051 + i;
        onProgress({
          phase: "ingest",
          message: "Mencatat email dan memeriksa duplikat…",
          accountId: a.id,
          current: i + 1,
          total: 8,
        });
        if (this.data.seen.includes(e.messageId)) {
          r.emailDuplicates++;
          continue;
        }
        this.data.seen.push(e.messageId);
        if (
          e.parsed?.reference &&
          this.s.transactions.some(
            (t) =>
              t.reference === e.parsed!.reference &&
              t.walletId === e.parsed!.walletId &&
              t.bank === e.parsed!.bank,
          )
        ) {
          e.status = "duplicate";
          e.reason = "Referensi bank sudah ada di buku kas.";
          r.ledgerDuplicates++;
        } else if (e.status === "posted") {
          this.post(e, { ...e.parsed!, note: "Notifikasi email contoh" });
          r.posted++;
        } else if (e.status === "review") r.review++;
        else if (e.status === "ignored") r.ignored++;
        else r.errors++;
        this.s.emails.unshift(e);
        await pause(this.persist ? 80 : 0);
      }
      a.lastSync = new Date().toISOString();
      await this.save();
      return r;
    } finally {
      this.syncing = false;
    }
  }
  private post(e: EmailRecord, p: EntryInput) {
    validateEntry(p, this.s);
    const existing = this.s.transactions.find((t) => t.emailId === e.id);
    if (existing) {
      Object.assign(existing, p);
      this.data.deleted = this.data.deleted.filter((x) => x !== existing.id);
    } else
      this.s.transactions.unshift({
        ...p,
        id: id(),
        category:
          p.kind === "income"
            ? "income"
            : p.kind === "transfer"
              ? "transfer"
              : p.category,
        source: "Email",
        emailId: e.id,
        bank: e.parsed?.bank || "Unknown",
        reference: e.parsed?.reference || null,
      });
    e.status = "posted";
  }
  async manual(input: EntryInput) {
    await this.ready;
    validateEntry(input, this.s);
    const key = id();
    this.s.transactions.unshift({
      ...input,
      id: key,
      source: "Manual",
      reference: null,
      bank: "Manual",
      emailId: null,
    });
    await this.save();
    return key;
  }
  async deleteEntry(key: string, undo: boolean) {
    await this.ready;
    const t = this.s.transactions.find((t) => t.id === key);
    if (!t) throw Error("Transaksi tidak ditemukan.");
    this.data.deleted = undo
      ? this.data.deleted.filter((x) => x !== key)
      : [...new Set([...this.data.deleted, key])];
    const e = this.s.emails.find((e) => e.id === t.emailId);
    if (e) e.status = undo ? "posted" : "ignored";
    await this.save();
  }
  async resolve(input: Parameters<FinanceAPI["resolve"]>[0]) {
    await this.ready;
    const e = this.s.emails.find((e) => e.id === input.emailId);
    if (!e || !["review", "error"].includes(e.status))
      throw Error("Email sudah diproses.");
    if (input.action === "ignore") e.status = "ignored";
    else {
      if (!input.entry) throw Error("Lengkapi data transaksi.");
      const p = input.entry;
      if (
        e.parsed?.reference &&
        this.s.transactions.some(
          (t) =>
            t.reference === e.parsed!.reference &&
            t.walletId === p.walletId &&
            t.bank === e.parsed!.bank &&
            t.emailId !== e.id,
        )
      )
        throw Error("Referensi ini sudah tercatat.");
      this.post(e, p);
    }
    await this.save();
  }
  async undoReview(key: string) {
    await this.ready;
    const e = this.s.emails.find((e) => e.id === key)!;
    const t = this.s.transactions.find((t) => t.emailId === key);
    if (t) this.data.deleted.push(t.id);
    e.status = "review";
    await this.save();
  }
  async wallet(input: Parameters<FinanceAPI["wallet"]>[0]) {
    await this.ready;
    if (
      input.opening < 0 ||
      !Number.isSafeInteger(input.opening) ||
      !input.name.trim()
    )
      throw Error("Isi nama dan saldo awal yang valid.");
    let w = this.s.wallets.find((w) => w.id === input.id);
    const key = w?.id || id();
    if (w) Object.assign(w, input);
    else
      this.s.wallets.push({
        ...input,
        id: key,
        balance: input.opening,
        observed: null,
      });
    await this.save();
    return key;
  }
  async reconcile(key: string, observed: number, adjust: boolean) {
    await this.ready;
    if (!Number.isSafeInteger(observed) || observed < 0)
      throw Error("Saldo pembanding tidak valid.");
    const w = computeWallets(this.s, this.data.deleted).find(
      (w) => w.id === key,
    )!;
    this.s.wallets.find((w) => w.id === key)!.observed = observed;
    if (adjust && observed !== w.balance) {
      const diff = observed - w.balance;
      await this.manual({
        walletId: key,
        toWalletId: null,
        kind: diff > 0 ? "income" : "expense",
        amount: Math.abs(diff),
        merchant: "Penyesuaian saldo",
        category: diff > 0 ? "income" : "other",
        occurredAt: new Date().toISOString(),
        note: "Rekonsiliasi eksplisit",
      });
    }
    await this.save();
  }
  async theme(theme: Parameters<FinanceAPI["theme"]>[0]) {
    await this.ready;
    this.s.theme = theme;
    await this.save();
  }
  async notificationStatus() {
    return {
      granted: true,
      enabled: true,
      connected: true,
      lastError: "",
    };
  }
  async notificationOpenSettings() {}
  async notificationSetEnabled() {}
  async notificationImport() {
    return 0;
  }
  async checkUpdate() {
    return {
      available: false,
      currentVersion: appVersion,
      currentVersionCode: 1,
      latestVersion: appVersion,
      latestVersionCode: 1,
      notes: "Mode browser / mock",
      apkUrl: "",
      apkSize: 0,
    };
  }
  async updateInstallPermission() { return true; }
  async updateInstallSettings() {}
  async downloadUpdate() {
    return true;
  }
  async installUpdate() {
    return true;
  }
}
