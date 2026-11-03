import { fileName, type QueueSnapshot } from "./file-queue";
import { audioTrackLabel, type AudioStream } from "./audio";

export function initializeQueueView(remove: (id: string) => void, clear: () => void, selectAudio: (id: string, index: number) => void) {
  const panel = document.querySelector<HTMLElement>("#fileQueue");
  const list = document.querySelector<HTMLUListElement>("#queueList");
  const summary = document.querySelector<HTMLElement>("#queueSummary");
  const clearButton = document.querySelector<HTMLButtonElement>("#clearQueueBtn");
  const progress = document.querySelector<HTMLProgressElement>("#queueProgress");
  const label = document.querySelector<HTMLElement>("#queueProgressLabel");
  const progressPanel = document.querySelector<HTMLElement>("#queueOverall");
  if (!panel || !list || !summary || !clearButton || !progress || !label || !progressPanel) throw new Error("Missing queue UI.");
  const rows = new Map<string, { row: HTMLLIElement; name: HTMLElement; status: HTMLElement; message: HTMLElement; button: HTMLButtonElement; audio: HTMLSelectElement; audioKey: string; streams?: readonly AudioStream[] }>();
  const onRemove = (event: Event) => { const button = event.target instanceof Element ? event.target.closest<HTMLButtonElement>("button[data-remove]") : null; if (button) remove(button.dataset.remove ?? ""); };
  const onClear = () => clear();
  const onAudio = (event: Event) => { if (event.target instanceof HTMLSelectElement && event.target.dataset.audioId) selectAudio(event.target.dataset.audioId, Number(event.target.value)); };
  list.addEventListener("click", onRemove); clearButton.addEventListener("click", onClear);
  list.addEventListener("change", onAudio);
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
          const audio = document.createElement("select"); audio.className = "queue-audio"; audio.dataset.audioId = item.id;
          audio.setAttribute("aria-label", `Audio track for ${fileName(item.path)}`);
          row.append(name, status, button, audio, message); list.append(row);
          value = { row, name, status, message, button, audio, audioKey: "" }; rows.set(item.id, value);
        }
        if (value.status.textContent !== item.status) {
          value.status.textContent = item.status; value.row.dataset.status = item.status;
        }
        if (value.message.textContent !== item.message) value.message.textContent = item.message;
        value.message.hidden = !item.message;
        const hideRemove = item.status !== "Pending";
        if (value.button.hidden !== hideRemove) value.button.hidden = hideRemove;
        const audioKey = JSON.stringify([item.audioProbeState, item.selectedAudioIndex, item.audioProbeError]);
        if (value.audioKey !== audioKey || value.streams !== item.audioStreams) {
          value.audioKey = audioKey;
          value.streams = item.audioStreams;
          const streams = item.audioStreams ?? [];
          const options = streams.map(stream => { const option = document.createElement("option"); option.value = String(stream.index); option.textContent = audioTrackLabel(stream); return option; });
          if (!options.length) {
            const option = document.createElement("option"); option.value = "";
            option.textContent = item.audioProbeState === "error" ? item.audioManual && item.selectedAudioIndex != null ? `🔊 Audio ${item.selectedAudioIndex + 1} (probe unavailable)` : "🔊 Audio: Auto (probe unavailable)"
              : item.audioProbeState === "ready" ? "🔇 No audio stream" : "Probing audio tracks…";
            options.push(option);
          }
          value.audio.replaceChildren(...options);
          value.audio.value = item.selectedAudioIndex == null ? "" : String(item.selectedAudioIndex);
          if (!streams.length) value.audio.selectedIndex = 0;
          const selected = streams.find(stream => stream.index === item.selectedAudioIndex);
          value.audio.title = item.audioProbeError || `${selected ? audioTrackLabel(selected) : "Source audio"}. Output is AAC stereo at 48 kHz.`;
        }
        const disabled = state.running || item.status !== "Pending" || item.audioProbeState !== "ready" || (item.audioStreams?.length ?? 0) < 2;
        if (value.audio.disabled !== disabled) value.audio.disabled = disabled;
      }
    },
    dispose() { list.removeEventListener("click", onRemove); list.removeEventListener("change", onAudio); clearButton.removeEventListener("click", onClear); rows.clear(); },
  };
}
