/** A presentation-only portal, outside scrolling cards and inside the browser top layer. */
export function initializeTooltips(): () => void {
  const popup = document.createElement("div");
  popup.id = "settings-tooltip";
  popup.className = "settings-tooltip";
  popup.setAttribute("role", "tooltip");
  popup.setAttribute("popover", "manual");
  const canPopover = typeof popup.showPopover === "function";
  if (!canPopover) popup.removeAttribute("popover");
  popup.hidden = true;
  document.body.append(popup);
  let active: HTMLElement | null = null;
  let describedBy: string | null = null;
  let hideTimer: ReturnType<typeof setTimeout> | undefined;
  const removers: Array<() => void> = [];
  const on = (node: EventTarget, event: string, handler: EventListener) => {
    node.addEventListener(event, handler);
    removers.push(() => node.removeEventListener(event, handler));
  };
  function cancelHide(): void { clearTimeout(hideTimer); }
  function hide(): void {
    cancelHide();
    if (active) {
      if (describedBy === null) active.removeAttribute("aria-describedby");
      else active.setAttribute("aria-describedby", describedBy);
    }
    active = null;
    if (canPopover && popup.matches(":popover-open")) popup.hidePopover();
    popup.hidden = true;
  }
  function show(anchor: HTMLElement): void {
    cancelHide();
    if (active !== anchor) {
      hide();
      active = anchor;
      describedBy = anchor.getAttribute("aria-describedby");
      anchor.setAttribute("aria-describedby", [describedBy, popup.id].filter(Boolean).join(" "));
    }
    popup.textContent = anchor.dataset.tooltip ?? "";
    popup.hidden = false;
    // Older WebViews still get a fixed body-level overlay.
    if (canPopover && !popup.matches(":popover-open")) {
      try { popup.showPopover(); } catch { /* Fixed-position fallback remains visible. */ }
    }
    const edge = 10, gap = 8;
    const box = anchor.getBoundingClientRect();
    const width = document.documentElement.clientWidth;
    const height = document.documentElement.clientHeight;
    const size = popup.getBoundingClientRect();
    const above = box.top - size.height - gap;
    const below = box.bottom + gap;
    const top = above >= edge ? above : below + size.height <= height - edge
      ? below : Math.max(edge, Math.min(above, height - size.height - edge));
    popup.style.left = `${Math.max(edge, Math.min(box.left + box.width / 2 - size.width / 2, width - size.width - edge))}px`;
    popup.style.top = `${top}px`;
  }
  function scheduleHide(): void {
    cancelHide();
    hideTimer = setTimeout(() => {
      if (active !== document.activeElement && !popup.matches(":hover")) hide();
    }, 120);
  }
  for (const anchor of document.querySelectorAll<HTMLElement>(".info-tip[data-tooltip]")) {
    const title = anchor.getAttribute("title");
    anchor.removeAttribute("title");
    on(anchor, "mouseenter", () => show(anchor));
    on(anchor, "mouseleave", scheduleHide);
    on(anchor, "focus", () => show(anchor));
    on(anchor, "blur", scheduleHide);
    on(anchor, "click", (event) => { event.preventDefault(); show(anchor); });
    removers.push(() => { if (title !== null) anchor.setAttribute("title", title); });
  }
  on(popup, "mouseenter", cancelHide);
  on(popup, "mouseleave", scheduleHide);
  const onScroll = (event: Event) => {
    if (event.target === popup) return;
    // Keyboard focus can scroll the panel after its focus event has shown help.
    // Keep that tooltip anchored; discard hover help and off-screen anchors.
    if (active && active === document.activeElement) {
      const rect = active.getBoundingClientRect();
      const clip = active.closest(".advanced-content")?.getBoundingClientRect();
      const x = rect.left + rect.width / 2, y = rect.top + rect.height / 2;
      if (x >= 0 && x <= document.documentElement.clientWidth
        && y >= 0 && y <= document.documentElement.clientHeight
        && (!clip || (x >= clip.left && x <= clip.right && y >= clip.top && y <= clip.bottom))) {
        show(active);
        return;
      }
    }
    hide();
  };
  document.addEventListener("scroll", onScroll, true);
  on(window, "resize", hide);
  on(document, "keydown", (event) => { if ((event as KeyboardEvent).key === "Escape") hide(); });
  on(document, "pointerdown", (event) => {
    if (active && event.target instanceof Node && !active.contains(event.target) && !popup.contains(event.target)) hide();
  });
  return () => {
    hide();
    document.removeEventListener("scroll", onScroll, true);
    removers.splice(0).forEach((remove) => remove());
    popup.remove();
  };
}
