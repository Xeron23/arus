import { describe, it, expect } from "vitest";
import { MockAPI } from "../src/native/mock";
import { btn, review } from "../src/ui/templates";

describe("priority fix frontend contracts", () => {
  it("uses explicit button type for non-submit actions", () => {
    expect(btn("ignore-email", "Abaikan", "secondary", "id")).toContain('type="button"');
  });
  it("keeps uncertain review items out of bulk confirmation", async () => {
    const api = new MockAPI(false);
    const s = await api.snapshot();
    s.emails.forEach(e => { if (e.parsed) e.parsed.confident = false; });
    const v = { page: "review" as const, month: "2026-10", wallet: "all", search: "", category: "all", kind: "all", status: "review", hidden: false };
    expect(review(s, v)).not.toContain('data-act="catat-semua"');
  });
  it("reports actual notification outcomes and exposes enable state", async () => {
    const api = new MockAPI(false);
    expect((await api.notificationStatus()).enabled).toBe(false);
    await api.notificationSetEnabled(true);
    expect((await api.notificationStatus()).enabled).toBe(true);
    expect(await api.notificationImport()).toEqual({posted:0,review:0,ignored:0,duplicates:0,failed:0,acknowledged:0,warnings:[]});
  });
});
