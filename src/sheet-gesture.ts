/** Delegated listeners survive shell re-renders. Only one finger may drag. */
export function installSheetGesture(dismiss: () => void): () => void {
  let gesture: {
    sheet: HTMLElement; body: HTMLElement; id: number;
    x: number; y: number; started: number; distance: number; dragging: boolean; fromHandle: boolean;
  } | null = null;

  function reset() {
    if (gesture) {
      gesture.sheet.classList.remove("is-dragging");
      gesture.sheet.style.removeProperty("transform");
      gesture = null;
    }
  }
  function start(event: TouchEvent) {
    reset();
    if (event.touches.length !== 1) return;
    const target = event.target;
    if (!(target instanceof Element)) return;
    const sheet = target.closest<HTMLElement>("#sheet");
    const body = sheet?.querySelector<HTMLElement>("#sheetBody");
    if (!sheet || !body || sheet.closest(".overlay")?.classList.contains("hidden")) return;
    if (target.closest("input,select,textarea,button,a,[contenteditable=true]")) return;
    if (!target.closest(".sheet-handle,.sheet-header") && body.scrollTop > 0) return;
    const touch = event.touches[0];
    gesture = {
      sheet, body, id: touch.identifier, x: touch.clientX, y: touch.clientY,
      started: performance.now(), distance: 0, dragging: false,
      fromHandle: !!target.closest(".sheet-handle,.sheet-header"),
    };
  }
  function move(event: TouchEvent) {
    const g = gesture;
    if (!g) return;
    if (!g.sheet.isConnected || event.touches.length !== 1) return reset();
    const touch = [...event.touches].find(t => t.identifier === g.id);
    if (!touch) return reset();
    const dx = touch.clientX - g.x, dy = touch.clientY - g.y;
    if (!g.dragging) {
      if (Math.max(Math.abs(dx), Math.abs(dy)) < 10) return;
      if (dy <= 0 || Math.abs(dx) >= dy || (!g.fromHandle && g.body.scrollTop > 0)) return reset();
      g.dragging = true;
      g.sheet.classList.add("is-dragging");
    }
    if (!event.cancelable) return reset();
    event.preventDefault();
    g.distance = Math.max(0, dy);
    g.sheet.style.transform = `translate3d(0, ${g.distance}px, 0)`;
  }
  function end(event: TouchEvent) {
    const g = gesture;
    if (!g) return;
    const elapsed = Math.max(1, performance.now() - g.started);
    const threshold = Math.min(140, Math.max(80, g.sheet.clientHeight * 0.22));
    const shouldClose = g.dragging && (
      g.distance >= threshold || (g.distance > 45 && g.distance / elapsed > 0.65)
    );
    if (g.dragging && event.cancelable) event.preventDefault();
    reset();
    if (shouldClose) dismiss();
  }

  document.addEventListener("touchstart", start, { passive: true });
  document.addEventListener("touchmove", move, { passive: false });
  document.addEventListener("touchend", end, { passive: false });
  document.addEventListener("touchcancel", reset, { passive: true });
  return reset;
}
