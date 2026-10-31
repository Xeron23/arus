import type { SyncRange } from "./models";

export const ranges: ReadonlyArray<readonly [SyncRange, string]> = [
  ["7", "7 hari terakhir"], ["30", "30 hari terakhir"],
  ["90", "90 hari terakhir"], ["all", "Semua email"],
];
let selected: SyncRange = "30";
try {
  const saved = localStorage.getItem("arus.syncRange");
  if (ranges.some(([value]) => value === saved)) selected = saved as SyncRange;
} catch { /* Preferences remain in memory when storage is unavailable. */ }

export function getSyncRange(): SyncRange { return selected; }
export function setSyncRange(value: string): void {
  if (!ranges.some(([key]) => key === value)) return;
  selected = value as SyncRange;
  try { localStorage.setItem("arus.syncRange", selected); } catch { /* Optional preference. */ }
}
export const ALL_BANKS: ReadonlyArray<string> = [
  "Mandiri",
  "BCA",
  "BRI",
  "BNI",
  "Jenius",
  "Bank Jago",
  "Seabank",
  "GoPay",
  "DANA",
  "OVO",
  "ShopeePay",
];

export function getSelectedBanks(): string[] {
  try {
    const saved = localStorage.getItem("arus.syncBanks");
    if (saved) {
      const parsed = JSON.parse(saved);
      if (Array.isArray(parsed) && parsed.length > 0) return parsed;
    }
  } catch {}
  return [...ALL_BANKS];
}

export function setSelectedBanks(banks: string[]): void {
  try {
    localStorage.setItem("arus.syncBanks", JSON.stringify(banks));
  } catch {}
}

export function toggleSyncBank(bank: string): string[] {
  const current = getSelectedBanks();
  let updated: string[];
  if (current.includes(bank)) {
    updated = current.filter((b) => b !== bank);
  } else {
    updated = [...current, bank];
  }
  setSelectedBanks(updated);
  return updated;
}

export function toggleAllBanks(): string[] {
  const current = getSelectedBanks();
  const allSelected = current.length === ALL_BANKS.length;
  const updated = allSelected ? [] : [...ALL_BANKS];
  setSelectedBanks(updated);
  return updated;
}

export function syncRangeOptions(): string {
  return ranges.map(([value, label]) =>
    `<option value="${value}" ${value === selected ? "selected" : ""}>${label}</option>`
  ).join("");
}
