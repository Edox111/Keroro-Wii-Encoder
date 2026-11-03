import { defaultVideo, normalizeVideoSettings, type VideoSettings } from "./video";
import { normalizeLanguage } from "./audio";
export const themes = ["keroro", "tamama", "giroro", "kururu", "dororo"] as const;
export type Theme = typeof themes[number];
export function isTheme(value: unknown): value is Theme {
  return typeof value === "string" && themes.some((theme) => theme === value);
}
export interface Preferences {
  preferredAudioLang: string;
  preferredSubLang: string;
  uiMode: "simple" | "advanced";
  simplePreset: "classic" | "widescreen" | "fastest";
  showLogs: boolean;
  selectedTheme: Theme;
  isMuted: boolean;
  minimizeToTrayOnClose: boolean;
  defaultCrf: 20 | 23 | 26;
  cpuMode: "balanced" | "maximum";
  video: VideoSettings;
  subtitleFolderOverridden: boolean;
  lastInputFolder: string;
  lastOutputFolder: string;
  lastSubtitleFolder: string;
  animeTitle: string;
}
const key = "keroro.preferences.v1";
const defaults: Preferences = {
  preferredAudioLang: "eng", preferredSubLang: "eng",
  uiMode: "simple", simplePreset: "classic", showLogs: false,
  video: { ...defaultVideo },
  selectedTheme: "keroro", isMuted: false, minimizeToTrayOnClose: true, defaultCrf: 23, cpuMode: "maximum", subtitleFolderOverridden: false,
  lastInputFolder: "", lastOutputFolder: "", lastSubtitleFolder: "", animeTitle: "",
};
function load(): Preferences {
  const result = { ...defaults, video: { ...defaultVideo } };
  try {
    const legacyTheme = localStorage.getItem("keroro.theme");
    if (isTheme(legacyTheme)) result.selectedTheme = legacyTheme;
    result.isMuted = localStorage.getItem("keroro.uiSounds") === "false";
    const raw: unknown = JSON.parse(localStorage.getItem(key) ?? "null");
    if (!raw || typeof raw !== "object" || Array.isArray(raw)) return result;
    const value = raw as Record<string, unknown>;
    for (const field of ["preferredAudioLang", "preferredSubLang"] as const) {
      if (typeof value[field] === "string" && value[field].length <= 16 && /^[a-z]{2,3}(?:[-_][a-z]{2})?$/i.test(value[field].trim())) result[field] = normalizeLanguage(value[field]);
    }
    if (value.uiMode === "simple" || value.uiMode === "advanced") result.uiMode = value.uiMode;
    if (value.simplePreset === "classic" || value.simplePreset === "widescreen" || value.simplePreset === "fastest") result.simplePreset = value.simplePreset;
    if (typeof value.showLogs === "boolean") result.showLogs = value.showLogs;
    result.video = normalizeVideoSettings(value.video) ?? result.video;
    if (isTheme(value.selectedTheme)) result.selectedTheme = value.selectedTheme;
    if (value.cpuMode === "balanced" || value.cpuMode === "maximum") result.cpuMode = value.cpuMode;
    if (typeof value.isMuted === "boolean") result.isMuted = value.isMuted;
    if (typeof value.minimizeToTrayOnClose === "boolean") result.minimizeToTrayOnClose = value.minimizeToTrayOnClose;
    if (value.defaultCrf === 20 || value.defaultCrf === 23 || value.defaultCrf === 26) result.defaultCrf = value.defaultCrf;
    for (const field of ["lastInputFolder", "lastOutputFolder", "lastSubtitleFolder", "animeTitle"] as const) {
      if (typeof value[field] === "string" && value[field].length <= 32768) result[field] = value[field];
    }
    // Older versions stored paths only. Preserve a distinct saved subtitle folder.
    result.subtitleFolderOverridden = typeof value.subtitleFolderOverridden === "boolean"
      ? value.subtitleFolderOverridden
      : result.lastSubtitleFolder !== "" && result.lastSubtitleFolder !== result.lastInputFolder;
    if (!result.subtitleFolderOverridden) result.lastSubtitleFolder = result.lastInputFolder;
  } catch { /* Malformed or unavailable storage uses safe defaults. */ }
  return result;
}
let preferences = load();
const preferenceListeners = new Set<() => void>();
export function subscribePreferences(listener: () => void): () => void {
  preferenceListeners.add(listener);
  return () => { preferenceListeners.delete(listener); };
}
export function getPreferences(): Readonly<Preferences> { return { ...preferences, video: { ...preferences.video } }; }
export function savePreferences(patch: Partial<Preferences>): boolean {
  preferences = { ...preferences, ...patch };
  preferenceListeners.forEach(listener => listener());
  try { localStorage.setItem(key, JSON.stringify(preferences)); return true; }
  catch { return false; }
}
