export type AspectRatio = "auto" | "4:3" | "16:9";
export type ResolutionPreset = "640x480" | "848x480" | "720x480" | "custom";
export interface VideoSettings {
  aspectRatio: AspectRatio;
  resolutionPreset: ResolutionPreset;
  customWidth: number;
  customHeight: number;
  autoCrop: boolean;
  encoderOverride: "auto" | "nvenc" | "amf" | "qsv" | "cpu";
}
export const defaultVideo: VideoSettings = {
  aspectRatio: "4:3", resolutionPreset: "640x480", customWidth: 640, customHeight: 480,
  autoCrop: false, encoderOverride: "auto",
};
export function isVideoSettings(value: unknown): value is VideoSettings {
  if (!value || typeof value !== "object") return false;
  const v = value as Record<string, unknown>;
  return ["auto", "4:3", "16:9"].includes(String(v.aspectRatio))
    && ["640x480", "848x480", "720x480", "custom"].includes(String(v.resolutionPreset))
    && typeof v.customWidth === "number" && Number.isInteger(v.customWidth)
    && v.customWidth >= 2 && v.customWidth <= 854 && v.customWidth % 2 === 0
    && typeof v.customHeight === "number" && Number.isInteger(v.customHeight)
    && v.customHeight >= 2 && v.customHeight <= 480 && v.customHeight % 2 === 0
    && typeof v.autoCrop === "boolean"
    && ["auto", "nvenc", "amf", "qsv", "cpu"].includes(String(v.encoderOverride));
}
export function normalizeVideoSettings(value: unknown): VideoSettings | null {
  if (!value || typeof value !== "object" || Array.isArray(value)) return null;
  const merged = { ...defaultVideo, ...value };
  if (String(merged.resolutionPreset) === "854x480") merged.resolutionPreset = "848x480";
  if (typeof merged.customWidth === "number") merged.customWidth = Math.round(merged.customWidth / 2) * 2;
  if (typeof merged.customHeight === "number") merged.customHeight = Math.round(merged.customHeight / 2) * 2;
  return isVideoSettings(merged) ? merged : null;
}
