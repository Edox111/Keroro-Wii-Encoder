export interface UpdateNotice {
  waitForRestart(): Promise<boolean>;
  setInstalling(): void;
  close(): void;
}

/** A fixed, non-modal notice. Downloading an update never reserves the encoder. */
export function showUpdateReady(version: string, canInstall: () => boolean, signal: AbortSignal): UpdateNotice {
  const notice = document.createElement("section");
  notice.className = "update-ready-toast";
  notice.setAttribute("aria-label", "Application update");
  const text = document.createElement("span");
  text.setAttribute("role", "status");
  text.textContent = `Version ${version} Update Ready — Restart to Apply`;
  const hint = document.createElement("small");
  hint.id = "update-restart-hint";
  const button = document.createElement("button");
  button.type = "button";
  button.textContent = "Restart Now";
  button.setAttribute("aria-describedby", hint.id);
  notice.append(text, button, hint);
  document.body.append(notice);
  let pending: ((accepted: boolean) => void) | null = null;
  let installing = false;
  let closed = false;
  function refresh(): void {
    button.disabled = closed || installing || signal.aborted || pending === null || !canInstall();
    hint.textContent = installing ? "Installing update…" : button.disabled
      ? "Available when encoding and other active tasks finish." : "Your preferences will be kept.";
  }
  function chooseRestart(): void {
    refresh();
    if (button.disabled || !pending) return;
    const resolve = pending;
    pending = null;
    button.disabled = true;
    resolve(true);
  }
  function close(): void {
    if (closed) return;
    closed = true;
    clearInterval(timer);
    signal.removeEventListener("abort", close);
    button.removeEventListener("click", chooseRestart);
    pending?.(false);
    pending = null;
    notice.remove();
  }
  button.addEventListener("click", chooseRestart);
  const timer = setInterval(refresh, 500);
  signal.addEventListener("abort", close, { once: true });
  refresh();
  if (signal.aborted) close();
  return {
    waitForRestart() {
      if (closed || signal.aborted) return Promise.resolve(false);
      const result = new Promise<boolean>((resolve) => { pending = resolve; });
      refresh();
      return result;
    },
    setInstalling() { installing = true; refresh(); },
    close,
  };
}
