import { invoke } from "./ipc";
import { fileName, type QueueItem } from "./file-queue";
import type { VideoSettings } from "./video";

export function initializeSnapshotModal(settings: () => VideoSettings | null, canOpen: () => boolean, setBusy: (busy: boolean) => void) {
  const modal = document.createElement("dialog"); modal.className = "app-modal snapshot-modal"; modal.id = "snapshotModal";
  modal.setAttribute("aria-labelledby", "snapshotHeading");
  modal.innerHTML = `<div class="modal-layout">
    <header class="modal-heading"><h2 id="snapshotHeading">Preview Snapshot</h2><p class="snapshot-source"></p></header>
    <div class="modal-body">
      <div class="snapshot-stage"><span class="snapshot-spinner" role="status">Rendering frame.</span><img alt="Encoded frame with the selected aspect ratio and subtitles" hidden></div>
      <p class="snapshot-status" role="status"></p>
      <div class="settings-field"><label for="snapshotTime">Preview timestamp</label><input type="range" id="snapshotTime" min="0" max="600" step="1" value="300"></div>
    </div>
    <footer class="modal-actions"><div class="snapshot-jumps" role="group" aria-label="Preview timestamp shortcuts"><button type="button" data-seconds="60">1m</button><button type="button" data-seconds="300">5m</button><button type="button" data-seconds="600">10m</button></div><button type="button" data-close>Close</button></footer>
  </div>`;
  document.body.append(modal);
  const image = modal.querySelector("img")!;
  const spinner = modal.querySelector<HTMLElement>(".snapshot-spinner")!;
  const status = modal.querySelector<HTMLElement>(".snapshot-status")!;
  const source = modal.querySelector<HTMLElement>(".snapshot-source")!;
  const slider = modal.querySelector<HTMLInputElement>("input")!;
  let current: QueueItem | null = null, selectedVideo: VideoSettings | null = null, disposed = false, rendering = false, generation = 0, url: string | null = null;
  function release() { if (url) URL.revokeObjectURL(url); url = null; image.removeAttribute("src"); }
  async function render(seconds: number) {
    if (rendering || !current || !selectedVideo || disposed || !modal.open) return;
    rendering = true; setBusy(true); const version = ++generation;
    seconds = Math.min(Math.max(0, Math.floor(seconds)), Number(slider.max)); slider.value = String(seconds);
    slider.disabled = true; modal.querySelectorAll<HTMLButtonElement>("[data-seconds]").forEach(b => { b.disabled = true; });
    spinner.hidden = false; image.hidden = true; status.textContent = ""; release();
    try {
      const bytes = await invoke("generate_preview_snapshot", { filePath: current.path, timestampSecs: seconds, subIndex: current.selectedSubtitleIndex ?? null, externalSubtitlePath: current.selectedSubtitleIndex != null ? null : current.subtitlePath ?? null, videoScale: selectedVideo.aspectRatio, video: selectedVideo });
      if (disposed || version !== generation || !modal.open) return;
      url = URL.createObjectURL(new Blob([bytes], { type: "image/jpeg" })); image.src = url; image.hidden = false;
      status.textContent = `Frame near ${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")} • Same crop, aspect ratio and subtitle settings as encoding.`;
    } catch (error) { if (!disposed && version === generation && modal.open) status.textContent = String(error); }
    finally {
      rendering = false; setBusy(false); spinner.hidden = true; slider.disabled = false;
      modal.querySelectorAll<HTMLButtonElement>("[data-seconds]").forEach(b => { b.disabled = false; });
    }
  }
  const close = () => { generation++; release(); if (rendering) void invoke("cancel_preview_snapshot").catch(() => {}); };
  const onSlider = () => { void render(Number(slider.value)); };
  const onClick = (event: MouseEvent) => {
    const button = event.target instanceof Element ? event.target.closest<HTMLButtonElement>("button") : null;
    if (button?.hasAttribute("data-close")) modal.close();
    else if (button?.dataset.seconds) void render(Number(button.dataset.seconds));
  };
  modal.addEventListener("click", onClick); modal.addEventListener("close", close); slider.addEventListener("change", onSlider);
  return {
    open(item: QueueItem) {
      if (disposed || rendering || !canOpen()) return;
      selectedVideo = settings(); if (!selectedVideo) return;
      current = item; source.textContent = fileName(item.path); source.title = item.path;
      slider.max = String(item.durationSeconds ? Math.max(0, Math.ceil(item.durationSeconds) - 1) : 3600);
      if (!modal.open) modal.showModal(); void render(300);
    },
    dispose() { disposed = true; close(); modal.removeEventListener("click", onClick); modal.removeEventListener("close", close); slider.removeEventListener("change", onSlider); modal.close(); modal.remove(); },
  };
}
