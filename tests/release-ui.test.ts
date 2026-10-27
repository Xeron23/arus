import { describe, it, expect, vi, afterEach } from "vitest";
import { installSheetGesture } from "../src/sheet-gesture";
import { getSyncRange, setSyncRange, syncRangeOptions } from "../src/sync-options";

class Node {
  isConnected = true;
  scrollTop = 0;
  clientHeight = 500;
  classes = new Set<string>();
  classList = {
    add: (value: string) => this.classes.add(value),
    remove: (value: string) => this.classes.delete(value),
    contains: (value: string) => this.classes.has(value),
  };
  style = { transform: "", removeProperty: vi.fn() };
  constructor(public role: string) {}
  closest(selector: string): Node | null {
    if (selector === "#sheet") return sheet;
    if (selector === ".overlay") return overlay;
    if (selector === ".sheet-handle,.sheet-header") return this.role === "handle" ? this : null;
    return null;
  }
  querySelector() { return body; }
}
let sheet: Node, body: Node, overlay: Node;
function harness() {
  sheet = new Node("sheet"); body = new Node("body"); overlay = new Node("overlay");
  const events = new Map<string, (event: any) => void>();
  vi.stubGlobal("Element", Node);
  vi.stubGlobal("document", {
    addEventListener: (name: string, fn: (event: any) => void) => events.set(name, fn),
  });
  const dismiss = vi.fn();
  const reset = installSheetGesture(dismiss);
  function send(name: string, x: number, y: number, target: Node = body, count = 1) {
    const preventDefault = vi.fn();
    events.get(name)!({
      target, cancelable: true, preventDefault,
      touches: Array.from({ length: count }, (_, identifier) => ({ identifier, clientX: x, clientY: y })),
    });
    return preventDefault;
  }
  return { send, dismiss, reset };
}
afterEach(() => vi.unstubAllGlobals());

describe("bottom sheet touch gestures", () => {
  it("dismisses a deliberate downward drag", () => {
    const h = harness();
    h.send("touchstart", 20, 20);
    expect(h.send("touchmove", 20, 160)).toHaveBeenCalled();
    h.send("touchend", 20, 160);
    expect(h.dismiss).toHaveBeenCalledTimes(1);
    expect(sheet.classes.has("is-dragging")).toBe(false);
  });
  it("preserves scrolling when body is not at its top", () => {
    const h = harness(); body.scrollTop = 60;
    h.send("touchstart", 20, 20); h.send("touchmove", 20, 200); h.send("touchend", 20, 200);
    expect(h.dismiss).not.toHaveBeenCalled();
  });
  it("preserves horizontal gestures and upward scrolling", () => {
    for (const [x, y] of [[200, 30], [20, -100]]) {
      const h = harness();
      h.send("touchstart", 20, 20); h.send("touchmove", x, y); h.send("touchend", x, y);
      expect(h.dismiss).not.toHaveBeenCalled();
    }
  });
  it("snaps back short drags and cancels interrupted gestures", () => {
    const h = harness();
    h.send("touchstart", 20, 20); h.send("touchmove", 20, 40); h.send("touchend", 20, 40);
    expect(h.dismiss).not.toHaveBeenCalled();
    h.send("touchstart", 20, 20); h.send("touchmove", 20, 200); h.send("touchcancel", 20, 200);
    h.send("touchend", 20, 200);
    expect(h.dismiss).not.toHaveBeenCalled();
  });
  it("allows handle drag over a scrolled sheet and resets before rerender", () => {
    const h = harness(); body.scrollTop = 100;
    h.send("touchstart", 20, 20, new Node("handle"));
    expect(h.send("touchmove", 20, 180)).toHaveBeenCalled();
    h.reset(); h.send("touchend", 20, 180);
    expect(h.dismiss).not.toHaveBeenCalled();
  });
  it("does not dismiss on multiple touches", () => {
    const h = harness();
    h.send("touchstart", 20, 20); h.send("touchmove", 20, 200, body, 2); h.send("touchend", 20, 200);
    expect(h.dismiss).not.toHaveBeenCalled();
  });
});

describe("sync range preferences", () => {
  it("accepts only supported periods", () => {
    setSyncRange("7"); expect(getSyncRange()).toBe("7");
    setSyncRange("365"); expect(getSyncRange()).toBe("7");
    setSyncRange("all"); expect(syncRangeOptions()).toContain('value="all" selected');
    setSyncRange("30");
  });
});
