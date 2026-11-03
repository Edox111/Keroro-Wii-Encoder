import { getLanguagePreferences, subscribeLanguagePreferences } from "./language-preferences";
import { FileQueue, QueueSafetyError, isQueueVideo, parentDirectory, videoExtensions } from "./file-queue";
import { initializeQueueView } from "./queue-view";
import { isSubtitleFile } from "./subtitle-matching";
import { initializeUiMode, type UiModeController } from "./ui-mode";
import { initializeTooltips } from "./tooltips";
import { defaultSubtitleLabel, subtitlePresentation } from "./subtitle-display";
import { defaultVideo, normalizeVideoSettings, type VideoSettings } from "./video";
import "./styles.css";
import "./ui-mode.css";
import "./queue.css";
import { startAutomaticUpdates } from "./updates";
import { getPreferences, savePreferences } from "./preferences";
import { cleanAnimeTitle, outputNameForInput } from "./naming";
import { initializeThemes } from "./themes";
import { isTerminal, type BatchProgress, type EncodingProgress } from "./batch";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

const audioUrls = import.meta.glob<string>(["./assets/*.mp3", "./assets/sfx/*.mp3", "!./assets/sfx/click.mp3", "!./assets/sfx/complete.mp3"], {
  eager: true, query: "?url", import: "default",
});

function createAudioManager() {
  const players = new Map<string, HTMLAudioElement>();
  function stop(): void {
    players.forEach((player) => {
      try { player.pause(); player.currentTime = 0; } catch { /* Audio cannot block a task. */ }
    });
  }
  function play(asset: string): void {
    const checkbox = document.querySelector<HTMLInputElement>("#uiSounds");
    if (!checkbox?.checked || checkbox.disabled || getPreferences().isMuted) return;
    try {
      const url = audioUrls[asset] ?? asset;
      stop();
      let player = players.get(url);
      if (!player) { player = new Audio(url); player.preload = "auto"; players.set(url, player); }
      player.volume = 0.4;
      // Missing placeholder files or rejected autoplay must not trigger another error sound.
      void player.play().catch(() => {});
    } catch { /* Unsupported audio must not affect encoding. */ }
  }
  return { play, stop };
}
const audioManager = createAudioManager();
function playClick(): void { audioManager.play("./assets/click.mp3"); }
function playSuccess(): void { audioManager.play("./assets/success.mp3"); }
function playError(): void { audioManager.play("./assets/error.mp3"); }

function onUiClick(event: MouseEvent): void {
  if (!(event.target instanceof Element)) return;
  const control = event.target.closest("button, input[type='checkbox'], input[type='radio'], summary");
  if (!control || control.matches(":disabled") || control.id === "uiSounds") return;
  playClick();
}
function onUiChange(event: Event): void {
  if (event.target instanceof Element && event.target.matches("select:not(:disabled), input[type='range']:not(:disabled)")) playClick();
}
document.addEventListener("click", onUiClick, true);
document.addEventListener("change", onUiChange, true);
function disposeAudio(): void {
  document.removeEventListener("click", onUiClick, true);
  document.removeEventListener("change", onUiChange, true);
  audioManager.stop();
}
window.addEventListener("pagehide", disposeAudio, { once: true });
import.meta.hot?.dispose(disposeAudio);

function formatEta(seconds: number | null | undefined): string {
  if (typeof seconds !== "number" || !Number.isFinite(seconds) || seconds < 0 || seconds > Number.MAX_SAFE_INTEGER) return "--";
  const total = Math.ceil(seconds);
  const clock = `${String(Math.floor(total / 60) % 60).padStart(2, "0")}m ${String(total % 60).padStart(2, "0")}s`;
  return total >= 3600 ? `${String(Math.floor(total / 3600)).padStart(2, "0")}h ${clock}` : clock;
}

function requireElement<T extends HTMLElement>(selector: string, type: { new (): T }): T {
  const element = document.querySelector(selector);
  if (!(element instanceof type)) throw new Error(`Missing or invalid UI element: ${selector}`);
  return element;
}

interface DropAnalysis { media_path: string | null; sub_path: string | null; output_path: string | null }
interface NativeDragPayload { paths: string[] }

interface RecoveryInfo {
  id: string; nextEpisode: number | null; totalFiles: number; completedFiles: number;
  canResume: boolean; problem: string | null;
  settings: { inputDir: string; outputDir: string; subtitleDir: string | null; animeTitle: string; crf: number; cpuMode?: "balanced" | "maximum"; video?: VideoSettings } | null;
}

async function initialize(): Promise<void> {
  // Persist migrated settings before deleting their legacy keys.
  if (savePreferences({})) {
    try {
      for (const key of ["keroro.notifications", "keroro.uiSounds", "keroro.theme"]) {
        localStorage.removeItem(key);
      }
    } catch { /* Storage restrictions must not block startup. */ }
  }
  initializeThemes((theme) => {
    const asset = `./assets/sfx/${theme}.mp3`;
    if (audioUrls[asset]) audioManager.play(asset);
  });
  const disposeTooltips = initializeTooltips();
  const overlay = requireElement("#drag-overlay", HTMLDivElement);
  const overlayText = requireElement("#drag-overlay-text", HTMLHeadingElement);
  const input = requireElement("#inputFolder", HTMLInputElement);
  const output = requireElement("#outputFolder", HTMLInputElement);
  const subtitles = requireElement("#subtitleFolder", HTMLInputElement);
  const browseSubtitles = requireElement("#browseSubtitleBtn", HTMLButtonElement);
  const resetSubtitles = requireElement("#resetSubtitleBtn", HTMLButtonElement);
  const animeTitle = requireElement("#animeTitle", HTMLInputElement);
  const outputHint = requireElement("#outputNameHint", HTMLElement);
  const preview = requireElement("#previewBtn", HTMLButtonElement);
  const modal = requireElement("#previewModal", HTMLDialogElement);
  const video = requireElement("#previewVideo", HTMLVideoElement);
  const closePreview = requireElement("#closePreviewBtn", HTMLButtonElement);
  const playbackStatus = requireElement("#previewPlaybackStatus", HTMLElement);
  const subtitleStatus = requireElement("#subtitleStatus", HTMLElement);
  const browseInput = requireElement("#browseInputBtn", HTMLButtonElement);
  const addFiles = requireElement("#addFilesBtn", HTMLButtonElement);
  const browseOutput = requireElement("#browseOutputBtn", HTMLButtonElement);
  const cpuMode = requireElement("#cpuMode", HTMLSelectElement);
  const aspectRatio = requireElement("#aspectRatio", HTMLSelectElement);
  const resolutionPreset = requireElement("#resolutionPreset", HTMLSelectElement);
  const customWidth = requireElement("#customWidth", HTMLInputElement);
  const customHeight = requireElement("#customHeight", HTMLInputElement);
  const autoCrop = requireElement("#autoCrop", HTMLInputElement);
  const encoderOverride = requireElement("#encoderOverride", HTMLSelectElement);
  const customSize = requireElement("#customSize", HTMLDivElement);

  const performanceDetails = requireElement("#performanceDetails", HTMLDetailsElement);
  const performanceText = requireElement("#performanceText", HTMLElement);
  const quality = requireElement("#crfSelect", HTMLSelectElement);
  const qualityValue = requireElement("#crfValue", HTMLElement);
  const sounds = requireElement("#uiSounds", HTMLInputElement);
  const soundObserver = new MutationObserver(() => { if (sounds.disabled || !sounds.checked) audioManager.stop(); });
  soundObserver.observe(sounds, { attributes: true, attributeFilter: ["disabled"] });
  const minimizeOnClose = requireElement("#minimizeOnClose", HTMLInputElement);
  const convert = requireElement("#convertBtn", HTMLButtonElement);
  const liveProgress = requireElement("#encodingProgress", HTMLElement);
  const liveTrack = requireElement("#encodingProgressTrack", HTMLElement);
  const liveFill = requireElement("#encodingProgressFill", HTMLElement);
  const liveText = requireElement("#encodingProgressText", HTMLElement);
  const status = requireElement("#status", HTMLElement);
  const notice = requireElement("#notice", HTMLElement);
  const fileName = requireElement("#fileName", HTMLElement);
  const fileLabel = requireElement("#fileLabel", HTMLElement);
  const filePercent = requireElement("#filePercent", HTMLElement);
  const batchPercent = requireElement("#batchPercent", HTMLElement);
  const fileProgress = requireElement("#fileProgress", HTMLElement);
  const batchProgress = requireElement("#batchProgress", HTMLElement);
  const fileBar = requireElement("#fileProgressBar", HTMLElement);
  const batchBar = requireElement("#progressBar", HTMLElement);
  const successCount = requireElement("#successCount", HTMLElement);
  const skippedCount = requireElement("#skippedCount", HTMLElement);
  const failedCount = requireElement("#failedCount", HTMLElement);
  const elapsed = requireElement("#elapsed", HTMLElement);
  const speed = requireElement("#speed", HTMLElement);
  const fps = requireElement("#fps", HTMLElement);
  const errorDetails = requireElement("#errorDetails", HTMLDetailsElement);
  const errorList = requireElement("#errorList", HTMLUListElement);
  const errorModal = requireElement("#errorModal", HTMLDialogElement);
  const errorMessage = requireElement("#errorMessage", HTMLElement);
  const closeError = requireElement("#closeErrorBtn", HTMLButtonElement);
  const recoveryModal = requireElement("#recoveryModal", HTMLDialogElement);
  const recoveryMessage = requireElement("#recoveryMessage", HTMLElement);
  const resumeBatch = requireElement("#resumeBatchBtn", HTMLButtonElement);
  const laterRecovery = requireElement("#laterRecoveryBtn", HTMLButtonElement);
  const discardRecovery = requireElement("#discardRecoveryBtn", HTMLButtonElement);
  const reviewRecovery = requireElement("#recoverBatchBtn", HTMLButtonElement);
  const themeButtons = document.querySelectorAll<HTMLButtonElement>(".platoon-btn[data-theme]");
  const controls = [addFiles, input, output, subtitles, browseInput, browseOutput, browseSubtitles, resetSubtitles, animeTitle, quality, cpuMode, aspectRatio, resolutionPreset, customWidth, customHeight, autoCrop, encoderOverride, preview];
  const unlisteners: UnlistenFn[] = [];
  const buffered = new Map<string, BatchProgress>();
  const completedSounds = new Set<string>();
  let busy = false;
  let activeKind: "batch" | "preview" = "batch";
  let closing = false;
  let previewLoading = false;
  let previewResource: { jobId: string; url: string } | null = null;
  let previewCleanup: Promise<void> = Promise.resolve();
  const shownPreviews = new Set<string>();
  let ready = false;
  let syncing = true;
  let starting = false;
  let pausePending = false;
  let pauseAction: "pause" | "resume" = "pause";
  let paused = false;
  let dropScanning = false;
  let dragVersion = 0;
  let disposed = false;
  let updating = false;
  let stopUpdates = () => {};
  let uiModeController: UiModeController | undefined;
  let fileQueue: FileQueue | null = null;
  let queueView: ReturnType<typeof initializeQueueView> | null = null;
  let stopLanguagePreferences = () => {};
  let currentJob: string | null = null;
  let sequence = -1;
  let diagnosticsKey = "";
  let measurementsKey = "";
  let encodingJob: string | null = null;
  let encodingSequence = -1;
  let recoveryPending: RecoveryInfo | null = null;
  let recoveryAction = false;
  let recoveryLoading = true;
  let recoveryRequest = 0;
  const reportedFailures = new Set<string>();
  let feedbackJob: string | null = null;
  let feedbackFailures = 0;
  function showError(message: string, sound = true): void {
    if (sound) playError();
    errorMessage.textContent = message;
    if (!disposed && !errorModal.open) errorModal.showModal();
  }
  closeError.addEventListener("click", () => errorModal.close());
  function showRecovery(): void {
    if (!recoveryPending || disposed || updating || closing) return;
    const info = recoveryPending;
    recoveryMessage.textContent = info.problem ?? (info.nextEpisode === null
      ? "All recorded episodes completed. Discard this leftover checkpoint to begin a new batch."
      : `An interrupted batch queue was detected. Would you like to resume from episode ${info.nextEpisode}? ${info.completedFiles} of ${info.totalFiles} completed outputs will be preserved.`);
    resumeBatch.disabled = !info.canResume || recoveryAction;
    discardRecovery.disabled = recoveryAction;
    if (!recoveryModal.open) recoveryModal.showModal();
  }
  reviewRecovery.addEventListener("click", showRecovery);
  laterRecovery.addEventListener("click", () => { if (!recoveryAction) recoveryModal.close(); });
  recoveryModal.addEventListener("cancel", (event) => { if (recoveryAction) event.preventDefault(); });
  async function refreshRecovery(show = false): Promise<void> {
    const request = ++recoveryRequest;
    recoveryLoading = true; setEncodingState(busy);
    try {
      const pending = await invoke<RecoveryInfo | null>("get_recovery");
      if (request !== recoveryRequest || disposed) return;
      recoveryPending = pending;
      reviewRecovery.hidden = recoveryPending === null;
      if (show) showRecovery();
    } finally { if (request === recoveryRequest) { recoveryLoading = false; setEncodingState(busy); } }
  }

  const preferences = getPreferences();
  minimizeOnClose.checked = preferences.minimizeToTrayOnClose;
  let subtitleFolderOverridden = preferences.subtitleFolderOverridden;
  input.value = preferences.lastInputFolder;
  output.value = preferences.lastOutputFolder;
  subtitles.value = preferences.lastSubtitleFolder;
  animeTitle.value = preferences.animeTitle;
  quality.value = String(preferences.defaultCrf);
  cpuMode.value = preferences.cpuMode;
  function restoreVideo(value: VideoSettings): void {
    aspectRatio.value = value.aspectRatio;
    resolutionPreset.value = value.resolutionPreset;
    customWidth.value = String(value.customWidth);
    customHeight.value = String(value.customHeight);
    autoCrop.checked = value.autoCrop;
    encoderOverride.value = value.encoderOverride;
    customSize.hidden = value.resolutionPreset !== "custom";
  }
  restoreVideo(preferences.video);
  function selectedVideo(): VideoSettings | null {
    const previous = getPreferences().video;
    const value = {
      aspectRatio: aspectRatio.value, resolutionPreset: resolutionPreset.value,
      customWidth: resolutionPreset.value === "custom" ? Number(customWidth.value) : previous.customWidth,
      customHeight: resolutionPreset.value === "custom" ? Number(customHeight.value) : previous.customHeight,
      autoCrop: autoCrop.checked, encoderOverride: encoderOverride.value,
    };
    return normalizeVideoSettings(value);
  }

  qualityValue.textContent = quality.value;
  function updateOutputNameHint(): void {
    outputHint.textContent = outputNameForInput(animeTitle.value, input.value);
  }
  updateOutputNameHint();
  function renderSubtitleSelection(): void {
    const externalSelected = subtitleFolderOverridden && subtitles.value.trim() !== "";
    subtitles.dataset.externalSelected = String(externalSelected);
    if (!busy) {
      subtitleStatus.textContent = externalSelected ? "Subtitles: External source selected" : defaultSubtitleLabel;
      subtitleStatus.dataset.externalSelected = String(externalSelected);
    }
  }
  renderSubtitleSelection();
  function saveForm(): void {
    updateOutputNameHint();
    renderSubtitleSelection();
    const crf = Number(quality.value);
    const video = selectedVideo();
    savePreferences({
      ...(video ? { video } : {}),
      lastInputFolder: input.value, lastOutputFolder: output.value,
      lastSubtitleFolder: subtitles.value, animeTitle: animeTitle.value,
      subtitleFolderOverridden,
      cpuMode: cpuMode.value === "balanced" ? "balanced" : "maximum",
      ...([20, 23, 26].includes(crf) ? { defaultCrf: crf as 20 | 23 | 26 } : {}),
    });
  }
  function syncSubtitles(): void {
    if (!subtitleFolderOverridden) subtitles.value = input.value;
  }
  input.addEventListener("input", () => { syncSubtitles(); saveForm(); });
  subtitles.addEventListener("input", () => { fileQueue?.clearSubtitleMatches(); subtitleFolderOverridden = true; saveForm(); });
  [output, animeTitle].forEach((field) => field.addEventListener("input", saveForm));
  quality.addEventListener("change", saveForm);
  cpuMode.addEventListener("change", saveForm);
  aspectRatio.addEventListener("change", saveForm);
  resolutionPreset.addEventListener("change", () => {
    customSize.hidden = resolutionPreset.value !== "custom";
    if (resolutionPreset.value === "640x480") aspectRatio.value = "4:3";
    else if (resolutionPreset.value !== "custom") aspectRatio.value = "16:9";
    saveForm();
  });
  [customWidth, customHeight].forEach((field) => field.addEventListener("input", saveForm));
  [customWidth, customHeight].forEach((field) => field.addEventListener("change", () => {
    const video = selectedVideo();
    if (video && resolutionPreset.value === "custom") {
      customWidth.value = String(video.customWidth);
      customHeight.value = String(video.customHeight);
      saveForm();
    }
  }));
  [autoCrop, encoderOverride].forEach((field) => field.addEventListener("change", saveForm));

  resetSubtitles.addEventListener("click", () => {
    fileQueue?.clearSubtitleMatches();
    subtitleFolderOverridden = false;
    syncSubtitles();
    saveForm();
  });

  function hideDropOverlay(): void {
    dragVersion += 1;
    overlay.hidden = true;
    overlay.classList.remove("is-visible");
    overlay.setAttribute("aria-hidden", "true");
  }
  function canAcceptDrop(): boolean {
    return ready && !fileQueue?.running && !busy && !starting && !pausePending && !dropScanning && !previewLoading && !closing && !disposed
      && !recoveryAction && !recoveryLoading && !browseInput.disabled && !browseOutput.disabled
      && !browseSubtitles.disabled && !document.querySelector("dialog[open]");
  }
  function dragPaths(payload: NativeDragPayload): string[] {
    return Array.isArray(payload?.paths) ? payload.paths.filter((path): path is string => typeof path === "string") : [];
  }
  function showDropOverlay(payload: NativeDragPayload): void {
    const paths = dragPaths(payload);
    if (!canAcceptDrop() || paths.length === 0) { hideDropOverlay(); return; }
    overlayText.textContent = paths.filter(isQueueVideo).length > 1 ? "Drop Videos to Queue" : paths.length > 1 ? "Drop Media & Subtitles"
      : /\.(ass|ssa|srt|vtt)$/i.test(paths[0]) ? "Drop Subtitle Here" : "Drop Media Here";
    overlay.hidden = false;
    overlay.classList.add("is-visible");
    overlay.setAttribute("aria-hidden", "false");
    const version = ++dragVersion;
    if (paths.length === 1) {
      void invoke<DropAnalysis>("analyze_dropped_paths", { paths }).then((analysis) => {
        if (version === dragVersion && !overlay.hidden && canAcceptDrop() && analysis?.output_path) {
          overlayText.textContent = "Drop Folder to set Output";
        }
      }).catch(() => { /* Drop execution reports errors; hover never changes the form. */ });
    }
  }
  async function applyDroppedPaths(payload: NativeDragPayload): Promise<void> {
    hideDropOverlay();
    if (!canAcceptDrop()) return;
    const paths = dragPaths(payload);
    if (!paths.length) return;
    dropScanning = true;
    setEncodingState(busy);
    try {
      const analysis = await invoke<DropAnalysis>("analyze_dropped_paths", { paths });
      if (disposed || updating || closing || busy || starting) return;
      if (!analysis || ![analysis.media_path, analysis.sub_path, analysis.output_path].every((path) => path === null || typeof path === "string")) {
        throw new Error("The dropped-path scanner returned an invalid response.");
      }
      if (!analysis.media_path && !analysis.sub_path && !analysis.output_path) throw new Error("No supported media, subtitles, or empty output folder found.");
      if (analysis.media_path && isQueueVideo(analysis.media_path)) fileQueue?.add(paths.filter(isQueueVideo), paths.filter(isSubtitleFile));
      else if (paths.some(isSubtitleFile)) fileQueue?.add([], paths.filter(isSubtitleFile));
      probeQueueAudio();
      if (analysis.output_path) output.value = analysis.output_path;
      if (analysis.media_path) input.value = analysis.media_path;
      const explicitSubtitle = paths.some((path) => /\.(ass|ssa|srt|vtt)$/i.test(path));
      if (analysis.sub_path && (!analysis.media_path || analysis.sub_path !== analysis.media_path || !subtitleFolderOverridden || explicitSubtitle)) {
        subtitles.value = analysis.sub_path;
        subtitleFolderOverridden = !analysis.media_path || analysis.sub_path !== analysis.media_path || explicitSubtitle;
      } else if (analysis.media_path) {
        syncSubtitles();
      }
      saveForm();
      playClick();
      notice.textContent = fileQueue?.hasItems ? "Videos queued in drop order. Start processes the queue one file at a time." : "Dropped folders applied. Review the paths before starting.";
    } catch (error) {
      if (!disposed && !closing) showError(`Cannot use dropped paths: ${String(error)}`);
    } finally {
      dropScanning = false;
      setEncodingState(busy);
    }
  }

  function showStatus(message: string, tone = "normal"): void {
    if (status.textContent !== message) status.textContent = message;
    status.dataset.tone = tone;
  }
  function resetEncodingProgress(): void {
    liveProgress.hidden = true;
    liveProgress.classList.remove("is-visible");
    liveText.textContent = "";
    setBar(liveTrack, liveFill, 0);
  }
  function renderEncodingProgress(payload: EncodingProgress): void {
    if (disposed || payload.jobId !== currentJob || activeKind !== "batch") return;
    if (!Number.isSafeInteger(payload.sequence) || payload.sequence < sequence) return;
    if (encodingJob !== payload.jobId) { encodingJob = payload.jobId; encodingSequence = -1; }
    if (payload.sequence <= encodingSequence) return;
    encodingSequence = payload.sequence;
    if (!busy || isTerminal(payload.phase)) { resetEncodingProgress(); return; }
    if (payload.paused || paused) {
      liveText.textContent = "Encoding paused - select Resume to continue.";
      return;
    }
    if (payload.phase !== "running" && payload.phase !== "queued") return;
    const percentage = typeof payload.percentage === "number" && Number.isFinite(payload.percentage)
      ? Math.min(100, Math.max(0, payload.percentage)) : null;
    liveProgress.hidden = false;
    liveProgress.classList.add("is-visible");
    setBar(liveTrack, liveFill, percentage);
    const eta = typeof payload.eta === "string" ? payload.eta.slice(0, 64) : "--";
    const file = typeof payload.file === "string" ? payload.file.slice(0, 128) : "video";
    liveText.textContent = payload.phase === "queued" ? "Preparing downscale... (ETA: --)"
      : `Encoding ${file}... ${percentage === null ? "Measuring..." : `${Math.floor(percentage)}%`} (ETA: ${eta})`;
  }

  function setEncodingState(isEncoding: boolean): void {
    busy = isEncoding;
    const queueRunning = fileQueue?.running ?? false;
    const locked = isEncoding || queueRunning;
    themeButtons.forEach((button) => { button.disabled = false; });
    if (isEncoding && activeKind === "batch" && !closing && !disposed) {
      liveProgress.hidden = false;
      liveProgress.classList.add("is-visible");
      if (!liveText.textContent) liveText.textContent = "Preparing downscale... (ETA: --)";
    } else { resetEncodingProgress(); }
    if (isEncoding || starting || dropScanning || closing || updating || disposed) hideDropOverlay();
    controls.forEach((control) => { control.disabled = (control === preview && !!fileQueue?.hasPendingAudioProbes) || updating || locked || starting || pausePending || dropScanning || disposed || !ready || previewLoading || closing || recoveryAction || recoveryLoading; });
    document.querySelector("main.card")?.setAttribute("aria-busy", String(updating || locked || starting || pausePending || dropScanning || previewLoading || closing));
    const activeBatch = isEncoding && activeKind === "batch";
    convert.classList.toggle("resume-btn", activeBatch && paused);
    document.body.classList.toggle("encoding-paused", activeBatch && paused);
    convert.textContent = activeBatch ? pausePending ? (pauseAction === "resume" ? "Resuming..." : "Pausing...") : paused ? "Resume" : "Pause" : queueRunning ? "Preparing next file..." : "Start Platoon Downscale";
    convert.disabled = !ready || starting || pausePending || dropScanning || disposed || previewLoading || updating || closing
      || recoveryAction || recoveryLoading || (isEncoding && activeKind !== "batch") || (queueRunning && !isEncoding)
      || !!fileQueue?.hasPendingAudioProbes || (!!fileQueue?.hasItems && !fileQueue.hasPending && !queueRunning && !isEncoding);
    preview.textContent = isEncoding && activeKind === "preview" ? "Generating preview..." : "Preview First 10 Seconds";
    if (recoveryPending && !isEncoding) convert.disabled = true;
    convert.title = recoveryPending ? "Resume or discard the interrupted checkpoint before starting another batch." : "";
    reviewRecovery.disabled = isEncoding || pausePending || dropScanning || !ready || updating || closing || recoveryAction;
    uiModeController?.setLocked(updating || locked || starting || pausePending || dropScanning || disposed || !ready || previewLoading || closing || recoveryAction || recoveryLoading);
  }
  function setBar(container: HTMLElement, bar: HTMLElement, value: number | null): void {
    container.classList.toggle("indeterminate", value === null);
    if (value === null) { container.removeAttribute("aria-valuenow"); return; }
    const percent = Math.max(0, Math.min(100, Number.isFinite(value) ? value : 0));
    container.setAttribute("aria-valuenow", String(Math.round(percent)));
    bar.style.width = `${percent}%`;
  }
  function renderMeasurements(m: NonNullable<BatchProgress["measurements"]>): void {
    const key = JSON.stringify(m);
    if (key === measurementsKey) return;
    measurementsKey = key;
    const number = (value: number | null | undefined, unit: string): string => typeof value === "number" && Number.isFinite(value) ? `${value.toFixed(2)}${unit}` : "Unavailable";
    performanceDetails.hidden = false;
    performanceText.textContent = [
      `Encoder: ${m.encoder}`,
      `External subtitles: ${m.subtitle_applied ? "burned in" : "off"}`,
      `CPU usage: ${m.cpu_mode} (${m.cpu_threads === 0 ? "automatic threads" : `${m.cpu_threads} threads`})`,
      `Media duration: ${number(m.media_seconds, " s")}`,
      `Wall time (includes pauses/verification): ${number(m.wall_seconds, " s")}`,
      `Processing rate: ${number(m.wall_seconds > 0 ? m.media_seconds / m.wall_seconds : null, "x")}`,
      `CPU user / system: ${number(m.user_cpu_seconds, " s")} / ${number(m.system_cpu_seconds, " s")}`,
      `Peak memory: ${number(m.peak_memory_kib == null ? null : m.peak_memory_kib / 1024, " MiB")}`,
      `Output size: ${number(m.output_bytes / 1024 / 1024, " MiB")}`,
    ].join("\n");
  }
  function render(progress: BatchProgress, effects: boolean): void {
    fileQueue?.observe(progress);
    if (progress.jobId !== currentJob || progress.sequence <= sequence) return;
    sequence = progress.sequence;
    if (feedbackJob !== progress.jobId) { feedbackJob = progress.jobId; feedbackFailures = 0; }
    if (effects && progress.failed > feedbackFailures) playError();
    feedbackFailures = Math.max(feedbackFailures, progress.failed);
    const terminal = isTerminal(progress.phase);
    activeKind = progress.kind ?? "batch";
    paused = !terminal && !!progress.paused;
    setEncodingState(!terminal);
    if (!effects || starting) {
      renderEncodingProgress({
        jobId: progress.jobId, sequence: progress.sequence, phase: progress.phase,
        file: `Ep${String(Math.max(1, progress.currentFile)).padStart(2, "0")}`,
        percentage: progress.filePercent, eta: formatEta(progress.etaSeconds),
      });
    }
    const subtitleView = subtitlePresentation(progress, subtitleFolderOverridden && subtitles.value.trim() !== "");
    const warnings = subtitleView.warnings;
    showStatus(subtitleView.message, progress.failed > 0 ? "error" : warnings.length > 0 ? "warning" : progress.phase === "succeeded" ? "success" : "normal");
    fileName.textContent = progress.fileName ?? "Preparing task...";
    subtitleStatus.textContent = subtitleView.label;
    subtitleStatus.dataset.externalSelected = String(subtitleView.highlighted);
    fileLabel.textContent = progress.currentFile > 0
      ? activeKind === "preview" ? "Generating first 10 seconds" : `Encoding file ${progress.currentFile} of ${progress.totalFiles}`
      : `Queued ${progress.totalFiles} files`;
    if (!paused || terminal) {
      filePercent.textContent = progress.filePercent === null ? (terminal ? "—" : "Measuring…") : `${progress.filePercent.toFixed(1)}%`;
      batchPercent.textContent = `${progress.batchPercent.toFixed(1)}%`;
      setBar(fileProgress, fileBar, progress.filePercent === null && terminal ? 0 : progress.filePercent);
      setBar(batchProgress, batchBar, progress.batchPercent);
    }
    successCount.textContent = String(progress.succeeded);
    skippedCount.textContent = String(progress.skipped ?? 0);
    failedCount.textContent = String(progress.failed);
    const seconds = Math.max(0, Math.floor(progress.elapsedSeconds));
    elapsed.textContent = `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
    speed.textContent = progress.speed ?? "—";
    fps.textContent = progress.fps === null ? "—" : progress.fps.toFixed(1);
    if (progress.encoder) fileLabel.textContent += ` | ${progress.encoder}`;
    if (progress.measurements) renderMeasurements(progress.measurements);
    const diagnostics = [
      ...progress.errors.map((entry) => ({ ...entry, level: "Error" })),
      ...warnings.map((entry) => ({ ...entry, level: "Warning" })),
    ];
    errorDetails.hidden = diagnostics.length === 0;
    const newDiagnostics = diagnostics.length > errorList.childElementCount;
    const nextKey = `${progress.jobId}:${JSON.stringify(diagnostics)}`;
    if (diagnosticsKey !== nextKey) {
      diagnosticsKey = nextKey;
      errorList.replaceChildren(...diagnostics.map((error) => {
        const item = document.createElement("li");
        item.dataset.level = error.level;
        item.textContent = `[${error.level}] ${error.fileName}: ${error.reason}${error.level === "Error" ? " Skipped." : ""}`;
        return item;
      }));
      if (newDiagnostics) errorDetails.open = true;
    }
    if (activeKind === "preview" && progress.phase === "succeeded") void openPreview(progress.jobId);
    if (activeKind === "batch" && !fileQueue?.running && effects && progress.phase === "succeeded" && !completedSounds.has(progress.jobId)) {
      completedSounds.add(progress.jobId);
      playSuccess();
    }
    if (!fileQueue?.running && effects && (progress.phase === "failed" || progress.phase === "completed_with_errors") && !reportedFailures.has(progress.jobId)) {
      reportedFailures.add(progress.jobId);
      showError(`${progress.message}\n\n${progress.errors.map((entry) => `${entry.fileName}: ${entry.reason}`).join("\n")}\n\nDetails are recorded locally in keroro-encoder.log.`, progress.failed === 0);
    }
    if (terminal && activeKind === "batch" && effects && !fileQueue?.running) void refreshRecovery().catch((error: unknown) => { playError(); notice.textContent = `Cannot read recovery state: ${String(error)}`; });
  }
  function receive(progress: BatchProgress): void {
    fileQueue?.observe(progress);
    if (syncing || starting) {
      const previous = buffered.get(progress.jobId);
      if (!previous || previous.sequence < progress.sequence) buffered.set(progress.jobId, progress);
      return;
    }
    if (progress.phase === "queued" && currentJob !== progress.jobId) { currentJob = progress.jobId; sequence = -1; }
    render(progress, true);
  }
  function clearPreview(): Promise<void> {
    const resource = previewResource;
    previewResource = null;
    video.pause();
    video.removeAttribute("src");
    video.load();
    if (resource) {
      URL.revokeObjectURL(resource.url);
      previewCleanup = previewCleanup.then(async () => {
        if (isTauri()) await invoke("release_preview", { jobId: resource.jobId });
      }).catch((error: unknown) => { playError(); notice.textContent = `Preview cleanup needs attention: ${String(error)}`; });
    }
    return previewCleanup;
  }
  async function openPreview(jobId: string): Promise<void> {
    if (shownPreviews.has(jobId) || disposed || updating || closing) return;
    shownPreviews.add(jobId);
    previewLoading = true;
    setEncodingState(busy);
    try {
      const bytes = await invoke<ArrayBuffer>("read_preview", { jobId });
      if (disposed || updating || closing || currentJob !== jobId) { await invoke("release_preview", { jobId }); return; }
      const url = URL.createObjectURL(new Blob([bytes], { type: "video/mp4" }));
      previewResource = { jobId, url };
      playbackStatus.textContent = "";
      video.src = url;
      modal.showModal();
      video.load();
    } catch (error) {
      playError();
      notice.textContent = `Cannot open preview: ${String(error)}`;
      await invoke("release_preview", { jobId }).catch(() => {});
    } finally {
      previewLoading = false;
      setEncodingState(busy);
    }
  }
  closePreview.addEventListener("click", () => modal.close());
  modal.addEventListener("close", () => { void clearPreview(); });
  video.addEventListener("error", () => { playError(); playbackStatus.textContent = "The preview could not be played. Close it and generate a new preview."; });
  function dispose(): void {
    disposed = true;
    stopUpdates();
    fileQueue?.stop();
    fileQueue?.stopProbing();
    stopLanguagePreferences();
    queueView?.dispose();
    uiModeController?.dispose();
    disposeTooltips();
    resetEncodingProgress();
    hideDropOverlay();
    soundObserver.disconnect();
    window.removeEventListener("keydown", onKeyDown);
    unlisteners.splice(0).forEach((unlisten) => { void Promise.resolve(unlisten()).catch(() => {}); });
    void clearPreview();
  }
  window.addEventListener("pagehide", dispose, { once: true });
  import.meta.hot?.dispose(dispose);

  sounds.checked = !getPreferences().isMuted;
  sounds.addEventListener("change", () => {
    savePreferences({ isMuted: !sounds.checked });
    if (sounds.checked) playClick(); else audioManager.stop();
  });

  let fullscreenPending = false;
  function onKeyDown(event: KeyboardEvent): void {
    if (event.key !== "F11" || event.ctrlKey || event.altKey || event.metaKey) return;
    // Prevent browser/WebView default fullscreen, including held-key repeats.
    event.preventDefault();
    if (event.repeat || fullscreenPending || disposed || updating || closing || !isTauri()) return;
    fullscreenPending = true;
    const desktopWindow = getCurrentWindow();
    void desktopWindow.isFullscreen()
      .then((fullscreen) => desktopWindow.setFullscreen(!fullscreen))
      .catch((error: unknown) => { playError(); notice.textContent = `Cannot toggle full screen: ${String(error)}`; })
      .finally(() => { fullscreenPending = false; });
  }
  window.addEventListener("keydown", onKeyDown);
  quality.addEventListener("change", () => { qualityValue.textContent = quality.value; });

  if (!isTauri()) {
    [browseInput, browseOutput, browseSubtitles, convert, preview, minimizeOnClose].forEach((control) => { control.disabled = true; });
    showStatus("UI preview — use the desktop application to encode and access native features.");
    return;
  }

  minimizeOnClose.addEventListener("change", async () => {
    const previous = getPreferences().minimizeToTrayOnClose;
    minimizeOnClose.disabled = true;
    try {
      await invoke("set_close_preferences", { minimizeOnClose: minimizeOnClose.checked });
      savePreferences({ minimizeToTrayOnClose: minimizeOnClose.checked });
    } catch (error) {
      minimizeOnClose.checked = previous;
      showError(`Cannot save close behavior: ${String(error)}`);
    } finally { minimizeOnClose.disabled = disposed || updating || closing; }
  });

  queueView = initializeQueueView(id => { if (!updating && !closing) fileQueue?.remove(id); }, () => { if (!updating && !closing) fileQueue?.clear(); }, (id, index) => { if (!updating && !closing) fileQueue?.selectAudio(id, index); });
  let queueStorage: Storage | undefined;
  try { queueStorage = localStorage; } catch { /* Queue stays usable without storage. */ }
  let queueUiKey = "";
  fileQueue = new FileQueue(snapshot => {
    if (disposed) return;
    queueView?.render(snapshot);
    const next = `${snapshot.running}:${snapshot.probing}:${snapshot.items.length}:${snapshot.complete}:${snapshot.items.some(item => item.status === "Pending")}`;
    if (next !== queueUiKey) { queueUiKey = next; setEncodingState(busy); }
  }, queueStorage);
  queueView.render(fileQueue.snapshot());
  stopLanguagePreferences = subscribeLanguagePreferences(() => fileQueue?.reselectAutomaticAudio(getLanguagePreferences().preferredAudioLang));
  function probeQueueAudio(): void {
    if (!fileQueue || disposed || !isTauri()) return;
    void fileQueue.probePendingAudio(path => invoke("probe_media_streams", { path }), () => getLanguagePreferences().preferredAudioLang)
      .catch(error => { if (!disposed) notice.textContent = `Cannot probe audio: ${String(error)}`; });
  }
  addFiles.addEventListener("click", () => {
    if (!canAcceptDrop()) return;
    addFiles.disabled = true;
    void open({ multiple: true, directory: false, title: "Add videos to queue", filters: [{ name: "Video Files", extensions: videoExtensions }] })
      .then(selected => { if (selected) return applyDroppedPaths({ paths: Array.isArray(selected) ? selected : [selected] }); })
      .catch(error => showError(`Cannot add videos: ${String(error)}`))
      .finally(() => setEncodingState(busy));
  });
  async function startFileQueue(): Promise<void> {
    if (!fileQueue || !fileQueue.hasPending || fileQueue.hasPendingAudioProbes || fileQueue.running || !canAcceptDrop()) return;
    if (recoveryPending) { showRecovery(); return; }
    const title = cleanAnimeTitle(animeTitle.value);
    if (!output.value.trim() || !selectedVideo() || ![20, 23, 26].includes(Number(quality.value))
      || (animeTitle.value.trim() && !title) || Array.from(title).length > 100) {
      showStatus("Choose an output folder, valid Wii video settings, and a filename-safe title before starting the queue.", "error");
      playError(); return;
    }
    await fileQueue.run(async (item, signal) => {
      if (disposed || closing || updating || signal.aborted) throw new QueueSafetyError("Queue stopped.");
      const result = await startEncoding("batch", { path: item.path, signal, subtitlePath: item.subtitlePath, selectedAudioIndex: item.selectedAudioIndex });
      if (!result) throw new QueueSafetyError("Queue stopped. Review input/output settings or the active native task before restarting.");
      return result;
    }, path => outputNameForInput(animeTitle.value, path));
    if (disposed || closing) return;
    const result = fileQueue.snapshot();
    showStatus(`Queue: ${result.items.filter(item => item.status === "Done").length} done, ${result.items.filter(item => item.status === "Error").length} errors, ${result.items.filter(item => item.status === "Pending").length} pending.`, result.items.some(item => item.status === "Error") ? "warning" : "success");
    if (result.complete && result.items.every(item => item.status === "Done")) playSuccess();
    await refreshRecovery().catch(error => { notice.textContent = `Cannot refresh recovery: ${String(error)}`; });
  }
  setEncodingState(false);
  try {
    await invoke("set_close_preferences", { minimizeOnClose: minimizeOnClose.checked });
    minimizeOnClose.disabled = false;
    await invoke("check_environment");
    const previousMeasurements = await invoke<BatchProgress["measurements"]>("get_performance_report");
    if (previousMeasurements) renderMeasurements(previousMeasurements);
    unlisteners.push(await listen<NativeDragPayload>("tauri://drag-enter", ({ payload }) => showDropOverlay(payload)));
    for (const event of ["tauri://drag-drop", "tauri://drop"]) {
      unlisteners.push(await listen<NativeDragPayload>(event, ({ payload }) => { void applyDroppedPaths(payload); }));
    }
    for (const event of ["tauri://drag-leave", "tauri://drag-cancel"]) {
      unlisteners.push(await listen(event, hideDropOverlay));
    }
    unlisteners.push(await listen<BatchProgress>("wii-progress", ({ payload }) => receive(payload)));
    unlisteners.push(await listen<EncodingProgress>("encoding-progress", ({ payload }) => {
      if (!starting && !syncing) renderEncodingProgress(payload);
    }));
    unlisteners.push(await listen<string>("task-cancelling", ({ payload }) => {
      closing = true;
      fileQueue?.stop();
      setEncodingState(true);
      showStatus(payload);
    }));
    const snapshot = await invoke<BatchProgress | null>("get_batch_state");
    if (disposed) { dispose(); return; }
    if (snapshot) { currentJob = snapshot.jobId; render(snapshot, false); }
    else setEncodingState(false);
    syncing = false;
    if (currentJob) {
      const latest = buffered.get(currentJob);
      if (latest) render(latest, false);
    }
    buffered.clear();
  } catch (error) {
    dispose();
    showStatus(`Cannot initialize encoding events: ${String(error)}`, "error");
    playError();
    errorMessage.textContent = `Cannot initialize the desktop backend: ${String(error)}`;
    if (!errorModal.open) errorModal.showModal();
    return;
  }

  async function selectFolder(target: HTMLInputElement, button: HTMLButtonElement): Promise<void> {
    button.disabled = true;
    try {
      const selected = await open({ directory: true, multiple: false, defaultPath: target.value || undefined, title: target === input ? "Select Input Anime Folder" : target === subtitles ? "Select Subtitle Folder" : "Select Output Wii Folder" });
      if (typeof selected === "string") {
        target.value = selected;
        if (target === subtitles) { fileQueue?.clearSubtitleMatches(); subtitleFolderOverridden = true; }
        if (target === input) syncSubtitles();
        saveForm();
      }
    } catch (error) { playError(); notice.textContent = `Cannot select folder: ${String(error)}`; }
    finally { button.disabled = busy || !ready || previewLoading || updating || closing; }
  }
  browseInput.addEventListener("click", () => { void selectFolder(input, browseInput); });
  browseOutput.addEventListener("click", () => { void selectFolder(output, browseOutput); });
  browseSubtitles.addEventListener("click", () => { void selectFolder(subtitles, browseSubtitles); });
  async function startEncoding(kind: "batch" | "preview", queued?: { path: string; signal: AbortSignal; subtitlePath?: string | null; selectedAudioIndex?: number | null }): Promise<BatchProgress | null> {
    if (fileQueue?.hasPendingAudioProbes) return null;
    if (!ready || busy || starting || pausePending || dropScanning || previewLoading || updating || closing || disposed || recoveryAction || recoveryLoading) return null;
    if (kind === "batch" && recoveryPending) { showRecovery(); return null; }
    const queuedBatch = !!queued && kind === "batch";
    saveForm();
    const inputDir = queued?.path ?? input.value.trim();
    const outputDir = output.value.trim();
    const crf = Number(quality.value);
    const title = cleanAnimeTitle(animeTitle.value);
    const videoSettings = selectedVideo();
    if (!videoSettings) {
      playError();
      showStatus("Use even custom dimensions: width 2–854 and height 2–480.", "error");
      return null;
    }

    if (!inputDir || (kind === "batch" && !outputDir) || ![20, 23, 26].includes(crf)) {
      playError();
      showStatus(kind === "preview" ? "Choose an input folder and quality preset." : "Choose input/output folders and a supported quality preset.", "error");
      return null;
    }
    if ((animeTitle.value.trim() && !title) || Array.from(title).length > 100) {
      playError();
      showStatus("Use an anime title with 1–100 filename-safe characters, or leave it blank.", "error");
      return null;
    }
    starting = true;
    encodingJob = null; encodingSequence = -1;
    activeKind = kind;
    buffered.clear();
    notice.textContent = "";
    setEncodingState(true);
    let acceptedJob = false;
    let queuedJobId: string | null = null;
    try {
      if (modal.open) modal.close();
      await clearPreview();
      showStatus(kind === "preview" ? "Preparing 10-second preview..." : "Preparing batch...");
      setBar(fileProgress, fileBar, null);
      setBar(batchProgress, batchBar, 0);
      filePercent.textContent = "Measuring...";
      batchPercent.textContent = "0%";
      if (queued && (queued.signal.aborted || disposed || closing)) throw new QueueSafetyError("Queue stopped before submission.");
      const args = {
        settings: { inputDir, outputDir, crf, cpuMode: cpuMode.value, video: videoSettings, selectedAudioIndex: queued?.selectedAudioIndex ?? null, preferredAudioLang: getLanguagePreferences().preferredAudioLang, subtitleDir: queued?.subtitlePath !== undefined ? queued.subtitlePath : queued && !subtitleFolderOverridden ? parentDirectory(queued.path) : subtitles.value.trim() || null, animeTitle: title },
      };
      const jobId = await invoke<string>(kind === "preview" ? "start_preview" : "start_batch", args);
      acceptedJob = true;
      queuedJobId = jobId;
      if (disposed) return null;
      currentJob = jobId;
      sequence = -1;
      const latest = buffered.get(jobId) ?? await invoke<BatchProgress | null>("get_batch_state");
      if (latest) render(latest, true);
    } catch (error) {
      setEncodingState(acceptedJob);
      if (queuedBatch && !acceptedJob) {
        if (!String(error).includes("An encoding task, update installation, or close confirmation is already active.")) playError();
        throw error;
      }
      if (String(error).includes("OUTPUT_PERMISSION_DENIED")) {
        notice.textContent = "Windows denied output-folder access. Choose a writable folder or review the app's Controlled Folder Access permissions.";
      }
      setBar(fileProgress, fileBar, 0);
      showStatus(acceptedJob ? `Task started; waiting for progress. Status refresh failed: ${String(error)}` : `Cannot start ${kind}: ${String(error)}`, "error");
      if (!queuedBatch) showError(String(error));
      filePercent.textContent = "0%";
    } finally {
      starting = false;
      if (currentJob) { const latest = buffered.get(currentJob); if (latest) render(latest, true); }
      buffered.clear();
      setEncodingState(busy);
    }
    if (queuedBatch && queued && queuedJobId && fileQueue) return fileQueue.waitForJob(queuedJobId,
      () => invoke<BatchProgress | null>("get_batch_state"), queued.signal);
    return null;
  }
  async function togglePause(): Promise<void> {
    if (!busy || activeKind !== "batch" || starting || pausePending || closing || updating || disposed || !currentJob) return;
    const jobId = currentJob;
    pauseAction = paused ? "resume" : "pause";
    const command = pauseAction === "resume" ? "resume_encode" : "pause_encode";
    pausePending = true;
    setEncodingState(busy);
    try {
      await invoke(command, { jobId });
      const snapshot = await invoke<BatchProgress | null>("get_batch_state");
      if (snapshot?.jobId === jobId) render(snapshot, false);
    } catch (error) {
      showError(`Cannot change pause state: ${String(error)}`);
    } finally {
      pausePending = false;
      setEncodingState(busy);
    }
  }
  convert.addEventListener("click", () => {
    if (busy) void togglePause(); else if (fileQueue?.hasItems) void startFileQueue(); else void startEncoding("batch");
  });
  preview.addEventListener("click", () => {
    const items = fileQueue?.snapshot().items ?? [];
    const item = items.find(item => item.status === "Pending") ?? items[0];
    void startEncoding("preview", item ? { path: item.path, signal: new AbortController().signal, subtitlePath: item.subtitlePath, selectedAudioIndex: item.selectedAudioIndex } : undefined);
  });
  discardRecovery.addEventListener("click", async () => {
    if (!recoveryPending || recoveryAction || busy) return;
    recoveryAction = true; discardRecovery.disabled = true; resumeBatch.disabled = true;
    try {
      await invoke("discard_recovery", { recoveryId: recoveryPending.id });
      recoveryPending = null; reviewRecovery.hidden = true; recoveryModal.close();
    } catch (error) { showError(`Cannot discard checkpoint: ${String(error)}`); }
    finally { recoveryAction = false; setEncodingState(busy); if (recoveryModal.open) showRecovery(); }
  });
  resumeBatch.addEventListener("click", async () => {
    if (!recoveryPending?.canResume || recoveryAction || busy || starting || pausePending || dropScanning || closing) return;
    const saved = recoveryPending.settings;
    if (saved) {
      input.value = saved.inputDir; output.value = saved.outputDir; subtitles.value = saved.subtitleDir ?? "";
      cpuMode.value = saved.cpuMode ?? "maximum";
      restoreVideo(normalizeVideoSettings(saved.video) ?? defaultVideo);
      animeTitle.value = saved.animeTitle; quality.value = String(saved.crf); qualityValue.textContent = quality.value;
      subtitleFolderOverridden = fileQueue?.hasItems ? getPreferences().subtitleFolderOverridden : saved.subtitleDir !== saved.inputDir;
      saveForm();
    }
    encodingJob = null; encodingSequence = -1;
    recoveryAction = true; starting = true; activeKind = "batch"; buffered.clear(); setEncodingState(true);
    resumeBatch.disabled = true; discardRecovery.disabled = true;
    let accepted = false;
    try {
      const jobId = await invoke<string>("resume_batch", { recoveryId: recoveryPending.id });
      accepted = true; recoveryPending = null; reviewRecovery.hidden = true; recoveryModal.close();
      currentJob = jobId; sequence = -1;
      const latest = buffered.get(jobId) ?? await invoke<BatchProgress | null>("get_batch_state");
      if (latest) render(latest, true);
    } catch (error) {
      setEncodingState(accepted); showError(`Cannot resume batch: ${String(error)}`);
      if (!accepted) await refreshRecovery().catch(() => { playError(); });
    } finally {
      starting = false; recoveryAction = false;
      if (currentJob) { const latest = buffered.get(currentJob); if (latest) render(latest, true); }
      buffered.clear(); setEncodingState(busy); if (recoveryModal.open) showRecovery();
    }
  });
  uiModeController = initializeUiMode({
    applyPreset: (preset) => {
      restoreVideo(preset.video);
      quality.value = String(preset.crf);
      qualityValue.textContent = quality.value;
      cpuMode.value = preset.cpuMode;
      saveForm();
    },
    readSettings: () => ({ video: selectedVideo(), crf: Number(quality.value), cpuMode: cpuMode.value }),
  });
  ready = true;
  probeQueueAudio();
  setEncodingState(busy);
  try { await refreshRecovery(true); } catch (error) { showError(`Cannot load interrupted batch state: ${String(error)}`); }
  if (!import.meta.env.DEV) stopUpdates = startAutomaticUpdates({
    canInstall: () => ready && !fileQueue?.running && !fileQueue?.hasPendingAudioProbes && !busy && !starting && !pausePending && !dropScanning && !previewLoading
      && !closing && !disposed && !recoveryAction && !recoveryLoading && !browseInput.disabled
      && !browseOutput.disabled && !browseSubtitles.disabled && !addFiles.disabled && !document.querySelector("dialog[open]"),
    setInstalling: (value) => {
      updating = value;
      setEncodingState(busy);
      if (value) showStatus("Installing update... The app will restart automatically.");
      else if (!disposed && !closing) showStatus("Ready.");
    },
  });
}

function start(): void {
  void initialize().catch((error: unknown) => {
    playError();
    const status = document.querySelector<HTMLElement>("#status");
    if (status) { status.textContent = `Cannot initialize application: ${String(error)}`; status.dataset.tone = "error"; }
    const convert = document.querySelector<HTMLButtonElement>("#convertBtn");
    if (convert) convert.disabled = true;
  });
}
if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", start, { once: true });
else start();
