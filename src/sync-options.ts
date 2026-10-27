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
export function syncRangeOptions(): string {
  return ranges.map(([value, label]) =>
    `<option value="${value}" ${value === selected ? "selected" : ""}>${label}</option>`
  ).join("");
}
