export type Phase = "queued" | "running" | "succeeded" | "completed_with_errors" | "failed" | "cancelled";

export interface BatchProgress {
  kind: "batch" | "preview";
  jobId: string;
  sequence: number;
  phase: Phase;
  paused?: boolean;
  encoder?: string | null;
  measurements?: { encoder: string; subtitle_applied?: boolean; cpu_mode: string; cpu_threads: number; media_seconds: number; wall_seconds: number; final_fps: number | null; user_cpu_seconds: number | null; system_cpu_seconds: number | null; peak_memory_kib: number | null; output_bytes: number } | null;
  currentFile: number;
  totalFiles: number;
  fileName: string | null;
  subtitleName: string | null;
  filePercent: number | null;
  batchPercent: number;
  succeeded: number;
  skipped?: number;
  failed: number;
  elapsedSeconds: number;
  fps: number | null;
  speed: string | null;
  etaSeconds?: number | null;
  message: string;
  errors: Array<{ fileName: string; reason: string }>;
  warnings: Array<{ fileName: string; reason: string }>;
}

export function isTerminal(phase: Phase): boolean {
  return phase === "succeeded" || phase === "completed_with_errors" || phase === "failed" || phase === "cancelled";
}

export interface EncodingProgress {
  file: string;
  percentage: number | null;
  eta: string;
  jobId: string;
  sequence: number;
  phase: Phase;
  paused?: boolean;
}
