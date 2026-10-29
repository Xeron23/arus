import { Channel, invoke, isTauri } from "@tauri-apps/api/core";
import type {
  FinanceAPI,
  Snapshot,
  ConnectionReport,
  SyncProgress,
  SyncReport,
} from "../models";
import { MockAPI } from "./mock";
class NativeAPI implements FinanceAPI {
  readonly native = true;
  snapshot: FinanceAPI["snapshot"] = () => invoke<Snapshot>("get_snapshot");
  test: FinanceAPI["test"] = (input) =>
    invoke<ConnectionReport>("test_imap_connection", { input });
  connect: FinanceAPI["connect"] = (connection, walletId, autoPost) =>
    invoke<string>("save_email_account", {
      input: { connection, walletId, autoPost },
    });
  unlock: FinanceAPI["unlock"] = (accountId, password) =>
    invoke<ConnectionReport>("unlock_email_account", { accountId, password });
  lock: FinanceAPI["lock"] = (accountId) =>
    invoke("lock_email_account", { accountId });
  updateAccount: FinanceAPI["updateAccount"] = (input) =>
    invoke("update_email_account", { input });
  sync: FinanceAPI["sync"] = (accountId, fn, range = "30") => {
    const onProgress = new Channel<SyncProgress>();
    onProgress.onmessage = fn;
    return invoke<SyncReport>("sync_email_transactions", {
      accountId,
      onProgress,
      range,
    });
  };
  manual: FinanceAPI["manual"] = (input) =>
    invoke<string>("save_manual_transaction", { input });
  deleteEntry: FinanceAPI["deleteEntry"] = (id, undo) =>
    invoke("delete_transaction", { id, undo });
  resolve: FinanceAPI["resolve"] = (input) =>
    invoke("resolve_review", { input });
  undoReview: FinanceAPI["undoReview"] = (id) => invoke("undo_review", { id });
  wallet: FinanceAPI["wallet"] = (input) =>
    invoke<string>("upsert_wallet", { input });
  reconcile: FinanceAPI["reconcile"] = (id, observed, adjust) =>
    invoke("reconcile_wallet", { id, observed, adjust });
  theme: FinanceAPI["theme"] = (theme) => invoke("set_theme", { theme });
  notificationStatus: FinanceAPI["notificationStatus"] = () =>
    invoke<import("../models").NotificationStatus>("notification_status");
  notificationOpenSettings: FinanceAPI["notificationOpenSettings"] = () =>
    invoke("notification_open_settings");
  notificationSetEnabled: FinanceAPI["notificationSetEnabled"] = (enabled) =>
    invoke("notification_set_enabled", { enabled });
  notificationImport: FinanceAPI["notificationImport"] = () =>
    invoke<import("../models").NotificationImportReport>("notification_import");
  checkUpdate: FinanceAPI["checkUpdate"] = () =>
    invoke<import("../models").UpdateInfo>("check_app_update");
  downloadUpdate: FinanceAPI["downloadUpdate"] = () => invoke<boolean>("download_app_update");
  updateInstallPermission: FinanceAPI["updateInstallPermission"] = () => invoke<boolean>("update_install_permission");
  updateInstallSettings: FinanceAPI["updateInstallSettings"] = () => invoke("update_install_settings");
  installUpdate: FinanceAPI["installUpdate"] = () =>
    invoke<boolean>("install_app_update");
}
// Native errors never fall back to mock: this would disguise failed real sync.
export const api: FinanceAPI = isTauri() ? new NativeAPI() : new MockAPI();
export function errorText(e: unknown): string {
  if (typeof e === "string") return e;
  if (e && typeof e === "object" && "message" in e) return String(e.message);
  return "Operasi belum berhasil. Silakan coba lagi.";
}
