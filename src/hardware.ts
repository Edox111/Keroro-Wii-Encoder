import type { VideoSettings } from "./video";
export type HardwareEncoder = "auto" | "nvenc" | "qsv" | "amf" | "x264";
export const encoderOptions: ReadonlyArray<{ value: HardwareEncoder; codec: string; label: string }> = [
  { value: "x264", codec: "libx264", label: "Software x264 (Default)" },
  { value: "nvenc", codec: "h264_nvenc", label: "NVIDIA NVENC" },
  { value: "qsv", codec: "h264_qsv", label: "Intel QuickSync" },
  { value: "amf", codec: "h264_amf", label: "AMD AMF" },
];
export function isHardwareEncoder(value: unknown): value is HardwareEncoder { return value === "auto" || encoderOptions.some(option => option.value === value); }
export function backendEncoder(value: HardwareEncoder): VideoSettings["encoderOverride"] { return value === "x264" ? "cpu" : value; }
export function hardwarePreference(value: VideoSettings["encoderOverride"]): HardwareEncoder { return value === "cpu" ? "x264" : value; }
