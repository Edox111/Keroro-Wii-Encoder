import { isTerminal, type BatchProgress } from "./batch";
import { isSubtitleFile, matchSubtitles } from "./subtitle-matching";
import { chooseAudioIndex, parseAudioStreams, type AudioStream } from "./audio";

export const videoExtensions = ["mp4", "mkv", "avi", "mov", "wmv", "flv", "webm"];
export const isQueueVideo = (path: string): boolean => /\.(mp4|mkv|avi|mov|wmv|flv|webm)$/i.test(path);
export const fileName = (path: string): string => path.split(/[\\/]/).pop() || path;
export function parentDirectory(path: string): string {
  const separator = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  if (separator < 0) return ".";
  const parent = path.slice(0, separator);
  return /^(?:\\\\\?\\)?[A-Za-z]:$/.test(parent) ? parent + path[separator] : parent || path[separator];
}
export type QueueStatus = "Pending" | "Encoding" | "Done" | "Error";
export interface QueueItem { id: string; path: string; status: QueueStatus; message: string; percent: number; jobId: string | null; subtitlePath?: string | null;
  audioStreams?: readonly AudioStream[]; selectedAudioIndex?: number | null; audioManual?: boolean;
  audioProbeState?: "pending" | "loading" | "ready" | "error"; audioProbeError?: string;
}
export interface QueueSnapshot { items: readonly QueueItem[]; running: boolean; probing: boolean; percent: number; label: string; complete: boolean }
export class QueueSafetyError extends Error {}
const key = "keroro.queue.v1";

/** One native job at a time. IPC acknowledgement is not encoding completion. */
export class FileQueue {
  private items: QueueItem[] = [];
  private nextId = 0;
  private subtitlePaths: string[] = [];
  private probeTask: Promise<void> | null = null;
  private probesDisposed = false;
  private abort: AbortController | null = null;
  private latest = new Map<string, BatchProgress>();
  private active: QueueItem | null = null;
  private observers = new Set<(progress: BatchProgress) => void>();
  running = false;
  constructor(private changed: (snapshot: QueueSnapshot) => void,
    private storage?: Pick<Storage, "getItem" | "setItem" | "removeItem">) {
    try {
      const raw = this.storage?.getItem(key);
      if (!raw || raw.length > 2_000_000) return;
      const data = JSON.parse(raw);
      const saved: unknown = Array.isArray(data) ? data : data?.items;
      if (!Array.isArray(saved) || saved.length > 500) return;
      if (!Array.isArray(data) && Array.isArray(data?.subtitlePaths)) {
        this.subtitlePaths = data.subtitlePaths.filter((path: unknown): path is string =>
          typeof path === "string" && path.length <= 32768 && isSubtitleFile(path)).slice(0, 500);
      }
      for (const value of saved) {
        if (!value || typeof value.path !== "string" || !isQueueVideo(value.path) || value.path.length > 32768) continue;
        const status = value.status === "Done" || value.status === "Error" ? value.status : "Pending";
        this.items.push({ id: `q${++this.nextId}`, path: value.path, status,
          message: value.status === "Encoding" ? "Interrupted. Existing output will be verified when restarted."
            : typeof value.message === "string" ? value.message.slice(0, 2048) : "",
          percent: status === "Done" || status === "Error" ? 100 : 0, jobId: null,
          ...(Number.isSafeInteger(value.selectedAudioIndex) && value.selectedAudioIndex >= 0 && value.selectedAudioIndex < 256
            ? { selectedAudioIndex: value.selectedAudioIndex, audioManual: value.audioManual === true } : {}),
          ...(value.subtitlePath === null || (typeof value.subtitlePath === "string" && isSubtitleFile(value.subtitlePath) && value.subtitlePath.length <= 32768)
            ? { subtitlePath: value.subtitlePath } : {}) });
      }
    } catch { /* Storage cannot prevent normal encoding. */ }
  }
  get hasPending(): boolean { return this.items.some(item => item.status === "Pending"); }
  get hasItems(): boolean { return this.items.length > 0; }
  get hasPendingAudioProbes(): boolean { return this.probeTask !== null || this.items.some(item => item.status === "Pending" && (item.audioProbeState === "pending" || item.audioProbeState === "loading")); }
  get paths(): string[] { return this.items.map(item => item.path); }
  snapshot(): QueueSnapshot {
    const settled = this.items.filter(item => item.status === "Done" || item.status === "Error").length;
    const current = this.active ? this.items.indexOf(this.active) + 1 : 0;
    return { items: this.items.map(item => ({ ...item })), running: this.running, probing: this.hasPendingAudioProbes,
      percent: this.items.length ? (settled + (this.active?.percent ?? 0) / 100) / this.items.length * 100 : 0,
      label: this.running ? `Processing ${current || Math.min(settled + 1, this.items.length)} of ${this.items.length}`
        : `${settled} of ${this.items.length} complete`,
      complete: this.items.length > 0 && settled === this.items.length && !this.running };
  }
  private emit(persist = true): void {
    if (persist) try {
      if (this.items.length || this.subtitlePaths.length) this.storage?.setItem(key, JSON.stringify({
        items: this.items.map(({ path, status, message, subtitlePath, selectedAudioIndex, audioManual }) => ({ path, status, message, subtitlePath, selectedAudioIndex, audioManual })), subtitlePaths: this.subtitlePaths,
      }));
      else this.storage?.removeItem(key);
    } catch { /* Quota/privacy errors leave the in-memory queue usable. */ }
    this.changed(this.snapshot());
  }
  add(paths: string[], subtitles: string[] = []): void {
    if (this.running) return;
    const normalized = (path: string) => path.replace(/\\/g, "/").toLocaleLowerCase("en-US");
    const existing = new Set(this.items.map(item => normalized(item.path)));
    const added = paths.filter(path => {
      if (!isQueueVideo(path)) return false;
      const name = normalized(path);
      if (existing.has(name)) return false;
      existing.add(name); return true;
    });
    const selectedSubtitles = [...new Set([...this.subtitlePaths, ...subtitles.filter(isSubtitleFile)])];
    if (this.items.length + added.length > 500 || added.some(path => path.length > 32768)
      || selectedSubtitles.length > 500 || selectedSubtitles.some(path => path.length > 32768)
      || [...this.paths, ...added, ...selectedSubtitles].reduce((sum, path) => sum + path.length, 0) > 1_000_000) {
      throw new Error("The file queue supports up to 500 videos within the saved-path size limit.");
    }
    this.items.push(...added.map(path => ({ id: `q${++this.nextId}`, path, status: "Pending" as const, message: "", percent: 0, jobId: null })));
    this.subtitlePaths = selectedSubtitles;
    if (selectedSubtitles.length) {
      const pending = this.items.filter(item => item.status === "Pending");
      // Completed assignments cannot be reused by subsequently added videos.
      const reserved = new Set(this.items.filter(item => item.status !== "Pending").map(item => item.subtitlePath));
      const pairs = matchSubtitles(pending.map(item => item.path), selectedSubtitles.filter(path => !reserved.has(path)));
      pending.forEach((item, index) => { item.subtitlePath = pairs[index].subtitlePath; });
    }
    this.emit();
  }
  clearSubtitleMatches(): void {
    if (this.running) return;
    this.subtitlePaths = [];
    this.items.filter(item => item.status === "Pending").forEach(item => { delete item.subtitlePath; });
    this.emit();
  }
  selectAudio(id: string, index: number): void {
    const item = this.items.find(item => item.id === id);
    if (this.running || !item || item.status !== "Pending" || item.audioProbeState !== "ready"
      || !item.audioStreams?.some(stream => stream.index === index)) return;
    item.selectedAudioIndex = index; item.audioManual = true; item.audioProbeError = ""; this.emit();
  }
  reselectAutomaticAudio(preferred: string): void {
    if (this.running) return;
    let changed = false;
    for (const item of this.items) {
      if (item.status !== "Pending" || item.audioManual || item.audioProbeState !== "ready") continue;
      const selected = chooseAudioIndex(item.audioStreams ?? [], preferred);
      if (selected !== item.selectedAudioIndex) { item.selectedAudioIndex = selected; changed = true; }
    }
    if (changed) this.emit();
  }
  probePendingAudio(probe: (path: string) => Promise<unknown>, preferred: () => string): Promise<void> {
    if (this.running || this.probesDisposed) return Promise.resolve();
    this.items.filter(item => item.status === "Pending" && item.audioProbeState === undefined)
      .forEach(item => { item.audioProbeState = "pending"; });
    if (!this.hasPendingAudioProbes) return Promise.resolve();
    this.emit(false);
    if (this.probeTask) return this.probeTask;
    const worker = async () => {
      while (!this.probesDisposed) {
        const item = this.items.find(item => item.status === "Pending" && item.audioProbeState === "pending");
        if (!item) return;
        item.audioProbeState = "loading"; this.emit(false);
        try {
          const streams = parseAudioStreams(await probe(item.path));
          if (this.probesDisposed || !this.items.includes(item)) continue;
          item.audioStreams = streams;
          if (!item.audioManual || !streams.some(stream => stream.index === item.selectedAudioIndex)) {
            if (item.audioManual) item.audioProbeError = "Previous audio choice is unavailable. Review the current selection.";
            item.selectedAudioIndex = chooseAudioIndex(streams, preferred()); item.audioManual = false;
          }
          item.audioProbeState = "ready";
        } catch (error) {
          if (this.probesDisposed || !this.items.includes(item)) continue;
          if (!item.audioManual) item.selectedAudioIndex = null;
          item.audioProbeState = "error"; item.audioProbeError = String(error).slice(0, 512);
        }
        this.emit();
      }
    };
    this.probeTask = Promise.all([worker(), worker()]).then(() => {}).finally(() => { this.probeTask = null; this.emit(false); });
    return this.probeTask;
  }
  stopProbing(): void { this.probesDisposed = true; }
  remove(id: string): void { this.items = this.items.filter(item => item.id !== id || item.status !== "Pending"); this.emit(); }
  clear(): void { if (this.snapshot().complete) { this.items = []; this.subtitlePaths = []; this.emit(); } }
  observe(progress: BatchProgress): void {
    if (progress.kind !== "batch") return;
    const previous = this.latest.get(progress.jobId);
    if (previous && previous.sequence >= progress.sequence) return;
    this.latest.set(progress.jobId, progress);
    if (this.latest.size > 8) this.latest.delete(this.latest.keys().next().value as string);
    if (this.active?.jobId === progress.jobId) {
      if (Number.isFinite(progress.batchPercent)) this.active.percent = Math.min(100, Math.max(0, progress.batchPercent));
      this.emit(false);
    }
    this.observers.forEach(observer => observer(progress));
  }
  waitForJob(jobId: string, read: () => Promise<BatchProgress | null>, signal: AbortSignal): Promise<BatchProgress> {
    if (this.active) { this.active.jobId = jobId; this.emit(); }
    return new Promise((resolve, reject) => {
      let polling = false, failures = 0, finished = false;
      const cleanup = () => { finished = true; clearInterval(timer); this.observers.delete(onProgress); signal.removeEventListener("abort", onAbort); };
      const onProgress = (progress: BatchProgress) => {
        if (progress.jobId === jobId && isTerminal(progress.phase) && !finished) { cleanup(); resolve(progress); }
      };
      const onAbort = () => { cleanup(); reject(new QueueSafetyError("Queue stopped. No further files were started.")); };
      const timer = setInterval(() => {
        if (polling || finished) return;
        polling = true;
        void read().then(progress => {
          if (finished) return;
          if (!progress || progress.jobId !== jobId) throw new Error("The active job could not be verified.");
          failures = 0; this.observe(progress);
          onProgress(progress);
        }).catch(() => {
          if (!finished && ++failures >= 3) { cleanup(); reject(new QueueSafetyError("Lost contact with the active encoder. Queue stopped; check the native task before restarting.")); }
        }).finally(() => { polling = false; });
      }, 1000);
      this.observers.add(onProgress);
      signal.addEventListener("abort", onAbort, { once: true });
      if (signal.aborted) onAbort();
      else { const cached = this.latest.get(jobId); if (cached) onProgress(cached); }
    });
  }
  async run(submit: (item: Readonly<QueueItem>, signal: AbortSignal) => Promise<BatchProgress>,
    outputName: (path: string) => string): Promise<void> {
    if (this.running || this.hasPendingAudioProbes || !this.hasPending) return;
    this.running = true;
    this.abort = new AbortController();
    const signal = this.abort.signal;
    const used = new Set<string>();
    this.emit(false);
    try {
      while (!signal.aborted) {
        const item = this.items.find(candidate => candidate.status === "Pending");
        if (!item) break;
        this.active = item; item.status = "Encoding"; item.message = ""; item.percent = 0; item.jobId = null; this.emit();
        try {
          const name = outputName(item.path).toLocaleLowerCase("en-US");
          if (used.has(name)) throw new Error("Output name collides with another queued file. Use distinct episode numbers, original filenames, or a separate output folder.");
          used.add(name);
          let progress: BatchProgress | undefined;
          for (let attempt = 0; attempt < 40; attempt++) {
            try { progress = await submit(item, signal); break; }
            catch (error) {
              // Terminal publish precedes release of the native job reservation.
              // Only retry that exact admission conflict, never a media failure.
              if (!String(error).includes("An encoding task, update installation, or close confirmation is already active.")) throw error;
              if (attempt === 39) throw new QueueSafetyError("Native task is still active. Queue stopped without starting another encoder.");
              await new Promise(resolve => setTimeout(resolve, 50));
              if (signal.aborted) throw new QueueSafetyError("Queue stopped.");
            }
          }
          if (!progress || signal.aborted) throw new QueueSafetyError("Queue stopped.");
          if (!isTerminal(progress.phase)) throw new QueueSafetyError("The native job has not finished. Queue stopped without launching another encoder.");
          item.status = progress.phase === "succeeded" ? "Done" : "Error";
          item.message = progress.phase === "succeeded" ? progress.skipped ? "Existing output verified or preserved by the backend." : "Encoded successfully."
            : (progress.errors.map(entry => entry.reason).join("\n") || progress.message).slice(0, 2048);
          if (progress.phase === "cancelled" || /recovery state could not be saved|worker stopped unexpectedly/i.test(progress.message)) {
            throw new QueueSafetyError(item.message);
          }
          if (progress.warnings?.some(entry => entry.fileName === "Recovery state")) {
            throw new QueueSafetyError("The completed output was preserved, but native recovery state needs attention. Queue stopped before submitting another file.");
          }
        } catch (error) {
          item.status = signal.aborted ? "Pending" : "Error"; item.message = String(error).slice(0, 2048);
          if (error instanceof QueueSafetyError) break;
        } finally {
          item.percent = item.status === "Pending" ? 0 : 100; this.active = null; this.emit();
        }
      }
    } finally {
      this.running = false; this.abort = null; this.emit();
    }
  }
  stop(): void { this.abort?.abort(); }
}
