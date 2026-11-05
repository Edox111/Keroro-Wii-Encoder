import { fileName, type QueueSnapshot, type QueueStatus } from "./file-queue";
import { audioTrackLabel, type AudioStream } from "./audio";
import { subtitleTrackLabel, type SubtitleStream } from "./media-streams";
import { performanceLabel } from "./performance";

interface QueueActions { snapshot(id: string): void; pause(id: string): void; error(id: string): void }
const statusLabels: Record<QueueStatus, string> = {
  Pending: "Queued", Encoding: "Encoding", Paused: "Paused", Done: "Completed", Error: "Failed",
};
export function initializeQueueView(remove: (id: string) => void, clear: () => void, selectAudio: (id: string, index: number) => void, selectSubtitle: (id: string, index: number | null) => void, actions?: QueueActions) {
  const panel = document.querySelector<HTMLElement>("#fileQueue");
  const list = document.querySelector<HTMLUListElement>("#queueList");
  const summary = document.querySelector<HTMLElement>("#queueSummary");
  const clearButton = document.querySelector<HTMLButtonElement>("#clearQueueBtn");
  const progress = document.querySelector<HTMLProgressElement>("#queueProgress");
  const label = document.querySelector<HTMLElement>("#queueProgressLabel");
  const progressPanel = document.querySelector<HTMLElement>("#queueOverall");
  if (!panel || !list || !summary || !clearButton || !progress || !label || !progressPanel) throw new Error("Missing queue UI.");
  const rows = new Map<string, { row: HTMLLIElement; warning: HTMLElement; snapshot: HTMLButtonElement; pause: HTMLButtonElement; error: HTMLButtonElement; status: HTMLElement; message: HTMLElement; button: HTMLButtonElement; audio: HTMLSelectElement; subtitle: HTMLSelectElement; stats: HTMLElement; subtitleKey: string; subtitleStreams?: readonly SubtitleStream[]; audioKey: string; streams?: readonly AudioStream[] }>();
  const onRemove = (event: Event) => { const button = event.target instanceof Element ? event.target.closest<HTMLButtonElement>("button[data-remove]") : null; if (button) remove(button.dataset.remove ?? ""); const action = event.target instanceof Element ? event.target.closest<HTMLButtonElement>("button[data-action]") : null; if (action && !action.disabled) { const id = action.dataset.id ?? ""; if (action.dataset.action === "snapshot") actions?.snapshot(id); else if (action.dataset.action === "pause") actions?.pause(id); else if (action.dataset.action === "error") actions?.error(id); } };
  const onClear = () => clear();
  const onAudio = (event: Event) => {
    if (!(event.target instanceof HTMLSelectElement)) return;
    if (event.target.dataset.audioId) selectAudio(event.target.dataset.audioId, Number(event.target.value));
    if (event.target.dataset.subtitleId) selectSubtitle(event.target.dataset.subtitleId, event.target.value === "" ? null : Number(event.target.value));
  };
  list.addEventListener("click", onRemove); clearButton.addEventListener("click", onClear);
  list.addEventListener("change", onAudio);
  let pending: QueueSnapshot | null = null;
  let frame: number | null = null;
  let previous: QueueSnapshot | null = null;
  const render = (state: QueueSnapshot): void => {
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
        const tracks = document.createElement("div"); tracks.className = "queue-tracks";
        const subtitle = document.createElement("select"); subtitle.className = "queue-subtitle"; subtitle.dataset.subtitleId = item.id;
        subtitle.setAttribute("aria-label", `Subtitle track for ${fileName(item.path)}`);
        const stats = document.createElement("small"); stats.className = "queue-stats";
        tracks.append(audio, subtitle);
        const warning = document.createElement("small"); warning.className = "queue-storage-warning";
        const buttons = document.createElement("div"); buttons.className = "queue-item-actions";
        buttons.setAttribute("role", "group");
        buttons.setAttribute("aria-label", `Actions for ${fileName(item.path)}`);
        const actionButton = (action: string, label: string) => {
          const control = document.createElement("button");
          control.type = "button";
          control.dataset.action = action;
          control.dataset.id = item.id;
          control.textContent = label;
          control.setAttribute("aria-label", `${label} for ${fileName(item.path)}`);
          buttons.append(control);
          return control;
        };
        const snapshot = actionButton("snapshot", "Preview"), pause = actionButton("pause", "Pause"), error = actionButton("error", "View Log");
        snapshot.title = `Preview snapshot for ${fileName(item.path)}`;
        buttons.append(button);
        row.append(name, status, buttons, tracks, message, stats, warning);
        list.append(row);
        value = { row, warning, snapshot, pause, error, status, message, button, audio, subtitle, stats, subtitleKey: "", audioKey: "" }; rows.set(item.id, value);
      }
      const statusLabel = statusLabels[item.status];
      value.warning.textContent = item.storageWarning ?? ""; value.warning.hidden = !item.storageWarning;
      value.snapshot.disabled = state.running || state.probing; value.snapshot.hidden = !actions;
      value.pause.hidden = !actions || (item.status !== "Encoding" && item.status !== "Paused"); value.pause.disabled = !item.jobId;
      value.pause.textContent = item.paused ? "Resume" : "Pause";
      value.pause.setAttribute("aria-label", `${item.paused ? "Resume" : "Pause"} ${fileName(item.path)}`);
      value.error.hidden = !actions || item.status !== "Error";
      if (value.status.textContent !== statusLabel) {
        value.status.textContent = statusLabel; value.row.dataset.status = item.status;
      }
      if (value.message.textContent !== item.message) value.message.textContent = item.message;
      value.message.hidden = !item.message;
      const hideRemove = item.status !== "Pending";
      if (value.button.hidden !== hideRemove) value.button.hidden = hideRemove;
      const audioKey = `${item.audioProbeState}:${item.audioProbeError ?? ""}:${item.audioStreams?.length ? "" : item.selectedAudioIndex ?? ""}`;
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
      }
      const audioValue = item.selectedAudioIndex == null ? "" : String(item.selectedAudioIndex);
      if (value.audio.value !== audioValue) value.audio.value = audioValue;
      if (!item.audioStreams?.length) value.audio.selectedIndex = 0;
      const selected = item.audioStreams?.find(stream => stream.index === item.selectedAudioIndex);
      const audioTitle = item.audioProbeError || `${selected ? audioTrackLabel(selected) : "Source audio"}. Output is AAC stereo at 48 kHz.`;
      if (value.audio.title !== audioTitle) value.audio.title = audioTitle;
      const subtitleKey = item.audioProbeState ?? "";
      if (value.subtitleKey !== subtitleKey || value.subtitleStreams !== item.subtitleStreams) {
        value.subtitleStreams = item.subtitleStreams;
        value.subtitleKey = subtitleKey;
        const fallback = document.createElement("option"); fallback.value = ""; fallback.textContent = "None / External Subtitle File";
        const options = (item.subtitleStreams ?? []).map(stream => {
          const option = document.createElement("option"); option.value = String(stream.index);
          option.textContent = subtitleTrackLabel(stream); option.disabled = !stream.supported; return option;
        });
        value.subtitle.replaceChildren(fallback, ...options);
        value.subtitle.title = item.audioProbeState === "error" ? "Stream probing unavailable. Existing external-subtitle routing remains available." : "Selected embedded text subtitles are burned into the video. None keeps existing external-subtitle routing.";
      }
      const subtitleValue = item.selectedSubtitleIndex == null ? "" : String(item.selectedSubtitleIndex);
      if (value.subtitle.value !== subtitleValue) value.subtitle.value = subtitleValue;
      const selectedSubtitle = item.subtitleStreams?.find(stream => stream.index === item.selectedSubtitleIndex);
      value.subtitle.title = selectedSubtitle ? subtitleTrackLabel(selectedSubtitle)
        : item.subtitlePath || "None / External Subtitle File. External subtitle routing remains available.";
      value.subtitle.disabled = state.running || item.status !== "Pending" || item.audioProbeState !== "ready" || !item.subtitleStreams?.some(s => s.supported);
      value.stats.hidden = item.status !== "Encoding" && item.status !== "Paused";
      const statsText = item.status === "Encoding" || item.status === "Paused" ? performanceLabel(item.percent, item.fps, item.speed, item.etaSeconds, item.paused) : "";
      if (value.stats.textContent !== statsText) value.stats.textContent = statsText;
      const disabled = state.running || item.status !== "Pending" || item.audioProbeState !== "ready" || (item.audioStreams?.length ?? 0) < 2;
      if (value.audio.disabled !== disabled) value.audio.disabled = disabled;
    }
  };
  return {
    render(state: QueueSnapshot): void {
      const before = previous;
      const progressOnly = state.running && before?.running
        && before.items.length === state.items.length
        && state.items.every((item, index) => item.id === before.items[index].id
          && item.status === before.items[index].status);
      previous = state;
      if (!progressOnly) {
        if (frame !== null) cancelAnimationFrame(frame);
        frame = null; pending = null;
        render(state);
        return;
      }
      pending = state;
      if (frame !== null) return;
      frame = requestAnimationFrame(() => {
        frame = null;
        const snapshot = pending;
        pending = null;
        if (snapshot) render(snapshot);
      });
    },
    dispose() {
      if (frame !== null) cancelAnimationFrame(frame);
      frame = null; pending = null; previous = null;
      list.removeEventListener("click", onRemove);
      list.removeEventListener("change", onAudio);
      clearButton.removeEventListener("click", onClear);
      rows.clear();
    },
  };
}
