export type Kind = "expense" | "income" | "transfer";
export type Theme = "light" | "dark";
export interface Wallet {
  id: string;
  name: string;
  kind: string;
  bank: string;
  digits: string;
  opening: number;
  balance: number;
  observed: number | null;
}
export interface EntryInput {
  walletId: string;
  toWalletId: string | null;
  kind: Kind;
  amount: number;
  merchant: string;
  category: string;
  occurredAt: string;
  note: string;
}
export interface LedgerEntry extends EntryInput {
  id: string;
  source: "Email" | "Manual";
  reference: string | null;
  bank: string;
  emailId: string | null;
}
export interface Mapping {
  bank: string;
  walletId: string;
}
export interface Account {
  id: string;
  email: string;
  provider: string;
  host: string;
  port: number;
  walletId: string;
  enabled: boolean;
  autoPost: boolean;
  folder: string | null;
  lastSync: string | null;
  unlocked: boolean;
  mappings: Mapping[];
}
export interface AccountUpdate {
  id: string;
  walletId: string;
  enabled: boolean;
  autoPost: boolean;
  mappings: Mapping[];
}
export interface ConnectionInput {
  email: string;
  password: string;
  provider: string;
  host?: string;
  port?: number;
  simulation?: "success" | "auth" | "timeout";
}
export interface ConnectionReport {
  host: string;
  port: number;
  provider: string;
  folder: string;
  elapsedMs: number;
  tls: boolean;
}
export interface Parsed extends Omit<EntryInput, "note"> {
  bank: string;
  reference: string | null;
  observed: number | null;
  confident: boolean;
}
export interface EmailRecord {
  id: string;
  accountId: string;
  mailbox: string;
  uid: number;
  uidValidity: number;
  messageId: string;
  gmId: string | null;
  sender: string;
  subject: string;
  receivedAt: string;
  rawSnippet: string;
  status: "review" | "posted" | "ignored" | "error" | "duplicate";
  reason: string;
  parsed: Parsed | null;
}
export interface Snapshot {
  wallets: Wallet[];
  transactions: LedgerEntry[];
  emails: EmailRecord[];
  accounts: Account[];
  theme: Theme;
}
export interface SyncProgress {
  phase: string;
  message: string;
  accountId: string;
  current: number;
  total: number;
}
export interface SyncReport {
  fetched: number;
  posted: number;
  review: number;
  ignored: number;
  errors: number;
  emailDuplicates: number;
  ledgerDuplicates: number;
  remaining: number;
  warnings: string[];
}
export interface WalletInput {
  id: string | null;
  name: string;
  kind: string;
  bank: string;
  digits: string;
  opening: number;
}
export interface ReviewInput {
  emailId: string;
  action: "post" | "ignore";
  entry: EntryInput | null;
}
export interface NotificationStatus {
  granted: boolean;
  enabled: boolean;
  connected: boolean;
  lastError: string;
}

export interface UpdateInfo {
  available: boolean;
  currentVersion: string;
  currentVersionCode: number;
  latestVersion: string;
  latestVersionCode: number;
  notes: string;
  apkUrl: string;
  apkSize: number;
}

export interface FinanceAPI {
  readonly native: boolean;
  snapshot(): Promise<Snapshot>;
  test(input: ConnectionInput): Promise<ConnectionReport>;
  connect(
    input: ConnectionInput,
    walletId: string,
    autoPost: boolean,
  ): Promise<string>;
  unlock(accountId: string, password: string): Promise<ConnectionReport>;
  lock(accountId: string): Promise<void>;
  updateAccount(input: AccountUpdate): Promise<void>;
  sync(
    accountId: string | null,
    onProgress: (progress: SyncProgress) => void,
  ): Promise<SyncReport>;
  manual(input: EntryInput): Promise<string>;
  deleteEntry(id: string, undo: boolean): Promise<void>;
  resolve(input: ReviewInput): Promise<void>;
  undoReview(id: string): Promise<void>;
  wallet(input: WalletInput): Promise<string>;
  reconcile(id: string, observed: number, adjust: boolean): Promise<void>;
  theme(theme: Theme): Promise<void>;
  notificationStatus(): Promise<NotificationStatus>;
  notificationOpenSettings(): Promise<void>;
  notificationSetEnabled(enabled: boolean): Promise<void>;
  notificationImport(): Promise<number>;
  checkUpdate(): Promise<UpdateInfo>;
  downloadUpdate(url: string): Promise<boolean>;
  installUpdate(): Promise<boolean>;
}

