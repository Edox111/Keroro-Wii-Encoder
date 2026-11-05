import { fileName, type QueueItem } from "./file-queue";

export function initializeErrorLogModal(retry: (id: string) => void, canRetry: () => boolean) {
  const modal = document.createElement("dialog"); modal.className = "app-modal diagnostic-modal"; modal.id = "errorLogModal";
  modal.setAttribute("aria-labelledby", "diagnosticHeading");
  modal.innerHTML = `<div class="modal-layout">
    <header class="modal-heading"><h2 id="diagnosticHeading">Encoding failed</h2><p class="diagnostic-source"></p></header>
    <div class="modal-body"><pre class="diagnostic-log" tabindex="0"></pre><p class="diagnostic-status" role="status"></p></div>
    <footer class="modal-actions"><button type="button" data-copy>Copy Error Log</button><button type="button" data-retry class="modal-primary">Retry Encoding</button><button type="button" data-close>Close</button></footer>
  </div>`;
  document.body.append(modal);
  const source = modal.querySelector<HTMLElement>(".diagnostic-source")!;
  const log = modal.querySelector<HTMLElement>("pre")!;
  const status = modal.querySelector<HTMLElement>(".diagnostic-status")!;
  const retryButton = modal.querySelector<HTMLButtonElement>("[data-retry]")!;
  let itemId: string | null = null, disposed = false;
  const click = (event: MouseEvent) => {
    const button = event.target instanceof Element ? event.target.closest<HTMLButtonElement>("button") : null;
    if (button?.hasAttribute("data-close")) modal.close();
    else if (button?.hasAttribute("data-retry") && itemId && canRetry()) { const id = itemId; modal.close(); retry(id); }
    else if (button?.hasAttribute("data-copy")) {
      void Promise.resolve().then(() => navigator.clipboard.writeText(log.textContent ?? "")).then(() => { if (!disposed) status.textContent = "Copied."; }).catch(() => { if (!disposed) status.textContent = "Clipboard unavailable. Select the log text and press Ctrl+C."; });
    }
  };
  modal.addEventListener("click", click);
  return {
    open(item: QueueItem) {
      if (disposed) return;
      itemId = item.id; source.textContent = fileName(item.path); source.title = item.path; log.textContent = item.errorLog || item.message || "No FFmpeg diagnostics were captured for this failure.";
      status.textContent = "Last 100 stderr lines. Logs may contain local paths; review before sharing.";
      retryButton.disabled = !canRetry(); if (!modal.open) modal.showModal();
    },
    dispose() { disposed = true; modal.removeEventListener("click", click); modal.close(); modal.remove(); },
  };
}
