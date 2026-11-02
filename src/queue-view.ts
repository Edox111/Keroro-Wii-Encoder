import { fileName, type QueueSnapshot } from "./file-queue";

export function initializeQueueView(remove: (id: string) => void, clear: () => void) {
  const panel = document.querySelector<HTMLElement>("#fileQueue");
  const list = document.querySelector<HTMLUListElement>("#queueList");
  const summary = document.querySelector<HTMLElement>("#queueSummary");
  const clearButton = document.querySelector<HTMLButtonElement>("#clearQueueBtn");
  const progress = document.querySelector<HTMLProgressElement>("#queueProgress");
  const label = document.querySelector<HTMLElement>("#queueProgressLabel");
  const progressPanel = document.querySelector<HTMLElement>("#queueOverall");
  if (!panel || !list || !summary || !clearButton || !progress || !label || !progressPanel) throw new Error("Missing queue UI.");
  const rows = new Map<string, { row: HTMLLIElement; name: HTMLElement; status: HTMLElement; message: HTMLElement; button: HTMLButtonElement }>();
  const onRemove = (event: Event) => { const button = event.target instanceof Element ? event.target.closest<HTMLButtonElement>("button[data-remove]") : null; if (button) remove(button.dataset.remove ?? ""); };
  const onClear = () => clear();
  list.addEventListener("click", onRemove); clearButton.addEventListener("click", onClear);
  return {
    render(state: QueueSnapshot) {
      panel.hidden = state.items.length === 0;
      const hasQueue = String(state.items.length > 0);
      if (document.body.dataset.hasQueue !== hasQueue) document.body.dataset.hasQueue = hasQueue;
      const summaryText = `${state.items.length} video${state.items.length === 1 ? "" : "s"} in queue`;
      if (summary.textContent !== summaryText) summary.textContent = summaryText;
      clearButton.hidden = !state.complete;
      progressPanel.hidden = !state.items.length;
      if (label.textContent !== state.label) label.textContent = state.label;
      progress.value = Math.min(100, Math.max(0, state.percent));
      const ids = new Set(state.items.map(item => item.id));
      rows.forEach((value, id) => { if (!ids.has(id)) { value.row.remove(); rows.delete(id); } });
      for (const item of state.items) {
        let value = rows.get(item.id);
        if (!value) {
          const row = document.createElement("li"), name = document.createElement("span"), status = document.createElement("span"), message = document.createElement("small"), button = document.createElement("button");
          name.className = "queue-name"; status.className = "queue-status"; message.className = "queue-message";
          button.type = "button"; button.textContent = "Remove"; button.dataset.remove = item.id;
          name.textContent = fileName(item.path); name.title = item.path;
          button.setAttribute("aria-label", `Remove ${fileName(item.path)}`);
          row.append(name, status, button, message); list.append(row);
          value = { row, name, status, message, button }; rows.set(item.id, value);
        }
        if (value.status.textContent !== item.status) {
          value.status.textContent = item.status; value.row.dataset.status = item.status;
        }
        if (value.message.textContent !== item.message) value.message.textContent = item.message;
        value.message.hidden = !item.message;
        const hideRemove = item.status !== "Pending";
        if (value.button.hidden !== hideRemove) value.button.hidden = hideRemove;
      }
    },
    dispose() { list.removeEventListener("click", onRemove); clearButton.removeEventListener("click", onClear); rows.clear(); },
  };
}
