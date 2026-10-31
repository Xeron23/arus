import { describe, it, expect } from "vitest";
import {
  MockAPI,
  seedSnapshot,
  computeWallets,
  validateEntry,
  classifyMerchant,
} from "../src/native/mock";
import { filtered, esc } from "../src/ui/templates";
import type { EntryInput, Snapshot } from "../src/models";
const entry = (kind: EntryInput["kind"] = "expense"): EntryInput => ({
  walletId: "bca",
  toWalletId: kind === "transfer" ? "gopay" : null,
  kind,
  amount: 25000,
  merchant: "Warteg",
  category: kind === "expense" ? "food" : kind,
  occurredAt: new Date().toISOString(),
  note: "Test",
});
const total = (s: Snapshot) => s.wallets.reduce((n, w) => n + w.balance, 0);
describe("ledger interactions", () => {
  it("transfer only moves balances", async () => {
    const a = new MockAPI(false),
      before = await a.snapshot();
    await a.manual(entry("transfer"));
    const after = await a.snapshot();
    expect(total(after)).toBe(total(before));
    expect(after.wallets.find((w) => w.id === "gopay")!.balance).toBe(
      before.wallets.find((w) => w.id === "gopay")!.balance + 25000,
    );
  });
  it("review does not affect balance until confirmation; undo restores it", async () => {
    const a = new MockAPI(false),
      before = await a.snapshot(),
      email = before.emails[0];
    await a.resolve({ emailId: email.id, action: "post", entry: entry() });
    expect(total(await a.snapshot())).toBe(total(before) - 25000);
    await a.undoReview(email.id);
    expect(total(await a.snapshot())).toBe(total(before));
  });
  it("replays do not create duplicate money", async () => {
    const a = new MockAPI(false),
      r = await a.sync(null, () => {}),
      before = await a.snapshot();
    expect(r.emailDuplicates).toBe(1);
    expect(r.ledgerDuplicates).toBe(1);
    await a.sync(null, () => {});
    const after = await a.snapshot();
    expect(total(after)).toBe(total(before));
    expect(after.transactions.length).toBe(before.transactions.length);
  });
  it("search and category filters compose", () => {
    const s = seedSnapshot(),
      v = {
        page: "transactions" as const,
        month: s.transactions[0].occurredAt.slice(0, 7),
        wallet: "bca",
        search: "indomaret",
        category: "shop",
        kind: "expense",
        status: "review",
        hidden: false,
      };
    expect(filtered(s, v).length).toBe(2);
  });
  it("rejects fractional Rupiah and same-wallet transfers", () => {
    const s = seedSnapshot();
    expect(() => validateEntry({ ...entry(), amount: 12.5 }, s)).toThrow();
    expect(() =>
      validateEntry({ ...entry("transfer"), toWalletId: "bca" }, s),
    ).toThrow();
  });
  it("escapes raw email and recognizes category rules", () => {
    expect(esc('<script>"')).toBe("&lt;script&gt;&quot;");
    expect(classifyMerchant("SPBU Pertamina")).toBe("transport");
    expect(classifyMerchant("Indomaret")).toBe("shop");
  });
  it("delete and undo update wallet balances", async () => {
    const a = new MockAPI(false),
      before = await a.snapshot(),
      id = await a.manual(entry());
    await a.deleteEntry(id, false);
    expect(total(await a.snapshot())).toBe(total(before));
    await a.deleteEntry(id, true);
    expect(total(await a.snapshot())).toBe(total(before) - 25000);
  });
  it("reconciliation without adjustment preserves ledger", async () => {
    const a = new MockAPI(false),
      before = await a.snapshot();
    await a.reconcile("bca", 9000000, false);
    expect(total(await a.snapshot())).toBe(total(before));
    expect(computeWallets(before).length).toBe(4);
  });
});

