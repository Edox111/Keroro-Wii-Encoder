import type { DriveInfo } from "./drive-selector";
import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import type { BatchProgress } from "./batch";
import type { MediaInfo } from "./media-streams";
import type { VideoSettings } from "./video";

export interface EncodingSettings {
  targetVolumeId?: string | null;
  inputDir: string;
  outputDir: string;
  crf: number;
  cpuMode: "balanced" | "maximum";
  video: VideoSettings;
  subtitleDir: string | null;
  animeTitle: string;
  selectedAudioIndex: number | null;
  selectedSubtitleIndex: number | null;
  preferredAudioLang: string | null;
  preferredSubLang: string | null;
}

export interface DropAnalysis {
  media_path: string | null;
  sub_path: string | null;
  output_path: string | null;
}

export interface RecoveryInfo {
  id: string;
  nextEpisode: number | null;
  totalFiles: number;
  completedFiles: number;
  canResume: boolean;
  problem: string | null;
  settings: EncodingSettings | null;
}

type Command<Args, Result> = { args: Args; result: Result };
interface Commands {
  get_available_encoders: Command<undefined, string[]>;
  get_connected_drives: Command<undefined, DriveInfo[]>;
  inspect_output_target: Command<{ path: string }, DriveInfo>;
  prepare_wii_drive: Command<{ driveId: string }, string>;
  generate_preview_snapshot: Command<{ filePath: string; timestampSecs?: number; subIndex: number | null; videoScale: string; video: VideoSettings; externalSubtitlePath: string | null }, ArrayBuffer>;
  cancel_preview_snapshot: Command<undefined, void>;
  pause_encoding: Command<{ fileId: string }, void>;
  resume_encoding: Command<{ fileId: string }, void>;
  start_batch: Command<{ settings: EncodingSettings }, string>;
  start_preview: Command<{ settings: EncodingSettings }, string>;
  pause_encode: Command<{ jobId: string }, void>;
  resume_encode: Command<{ jobId: string }, void>;
  get_batch_state: Command<undefined, BatchProgress | null>;
  probe_media_streams: Command<{ path: string }, MediaInfo>;
  analyze_dropped_paths: Command<{ paths: string[] }, DropAnalysis>;
  get_recovery: Command<undefined, RecoveryInfo | null>;
  resume_batch: Command<{ recoveryId: string }, string>;
  discard_recovery: Command<{ recoveryId: string }, void>;
  read_preview: Command<{ jobId: string }, ArrayBuffer>;
  release_preview: Command<{ jobId: string }, void>;
  check_environment: Command<undefined, void>;
  get_performance_report: Command<undefined, BatchProgress["measurements"]>;
  set_close_preferences: Command<{ minimizeOnClose: boolean }, void>;
  open_output_folder: Command<{ path: string }, void>;
  updater_is_configured: Command<undefined, boolean>;
  prepare_update_install: Command<undefined, boolean>;
  release_update_install: Command<undefined, void>;
}

export function invoke<C extends keyof Commands>(
  command: C,
  ...args: Commands[C]["args"] extends undefined ? [] : [Commands[C]["args"]]
): Promise<Commands[C]["result"]> {
  return tauriInvoke<Commands[C]["result"]>(command, args[0]);
}
