export interface CompletionPreferences { playSoundOnComplete: boolean; openFolderOnComplete: boolean }
export function createCompletionSound() {
  let player: HTMLAudioElement | null = null;
  let disposed = false, generation = 0;
  function release() {
    if (!player) return;
    try { player.pause(); player.removeAttribute("src"); player.load(); } catch { /* Optional sound. */ }
    player = null;
  }
  return {
    async play(): Promise<void> {
      const request = ++generation; release();
      for (const file of ["completion.mp3", "completion.wav"]) {
        if (disposed || request !== generation) return;
        try {
          const audio = new Audio(`${import.meta.env.BASE_URL}sounds/${file}`); player = audio;
          audio.preload = "none"; audio.volume = 0.4;
          await audio.play();
          if (disposed || request !== generation) { audio.pause(); return; }
          return;
        } catch { if (request === generation) release(); /* Missing assets/autoplay never reject a batch. */ }
      }
    },
    dispose() { disposed = true; generation++; release(); },
  };
}
/** One invocation per admitted batch/queue, never one per progress event or episode. */
export function createCompletionActions(options: {
  preferences: () => CompletionPreferences;
  playSound: () => Promise<void>;
  openFolder: (path: string) => Promise<void>;
  report: (message: string) => void;
}) {
  const completed = new Set<string>();
  let disposed = false;
  return {
    complete(id: string, outputDir: string): void {
      if (disposed || completed.has(id)) return;
      completed.add(id);
      if (completed.size > 128) completed.delete(completed.values().next().value as string);
      const settings = options.preferences();
      if (settings.playSoundOnComplete) void Promise.resolve().then(() => { if (!disposed) return options.playSound(); }).catch(() => {});
      if (settings.openFolderOnComplete && outputDir) void Promise.resolve().then(() => {
        if (!disposed) return options.openFolder(outputDir);
      }).catch(error => { if (!disposed) options.report(`Encoding finished, but the output folder could not be opened: ${String(error)}`); });
    },
    dispose() { disposed = true; completed.clear(); },
  };
}
