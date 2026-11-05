import { isHardwareEncoder, backendEncoder, type HardwareEncoder } from "./hardware";
import { getPreferences, savePreferences, subscribePreferences } from "./preferences";
import { normalizeLanguage } from "./audio";

export interface LanguagePreferences { hardwareEncoder: HardwareEncoder; autoRouteWiiDrive: boolean; preferredAudioLang: string; preferredSubLang: string; defaultOutputDir: string; playSoundOnComplete: boolean; openFolderOnComplete: boolean }
function settingsSnapshot(): Readonly<LanguagePreferences> {
  const { hardwareEncoder, autoRouteWiiDrive, preferredAudioLang, preferredSubLang, defaultOutputDir, playSoundOnComplete, openFolderOnComplete } = getPreferences();
  return Object.freeze({ hardwareEncoder, autoRouteWiiDrive, preferredAudioLang, preferredSubLang, defaultOutputDir, playSoundOnComplete, openFolderOnComplete });
}
let current = settingsSnapshot();
const listeners = new Set<() => void>();
subscribePreferences(() => {
  const next = settingsSnapshot();
  if (next.hardwareEncoder === current.hardwareEncoder && next.autoRouteWiiDrive === current.autoRouteWiiDrive && next.preferredAudioLang === current.preferredAudioLang && next.preferredSubLang === current.preferredSubLang && next.defaultOutputDir === current.defaultOutputDir && next.playSoundOnComplete === current.playSoundOnComplete && next.openFolderOnComplete === current.openFolderOnComplete) return;
  current = next;
  listeners.forEach(listener => listener());
});
export const getLanguagePreferences = (): Readonly<LanguagePreferences> => current;
export function subscribeLanguagePreferences(listener: () => void): () => void {
  listeners.add(listener);
  return () => { listeners.delete(listener); };
}
export function updateLanguagePreferences(patch: Partial<LanguagePreferences>): boolean {
  const normalized: Partial<LanguagePreferences> = {};
  if (patch.hardwareEncoder !== undefined) {
    if (!isHardwareEncoder(patch.hardwareEncoder)) throw new Error("Unsupported hardware encoder.");
    normalized.hardwareEncoder = patch.hardwareEncoder;
  }
  for (const key of ["preferredAudioLang", "preferredSubLang"] as const) {
    if (patch[key] !== undefined) {
      const value = patch[key];
      const special = key === "preferredAudioLang" && value === "any" || key === "preferredSubLang" && value === "none";
      if (!special && (value.length > 16 || !/^[a-z]{2,3}(?:[-_][a-z]{2})?$/i.test(value.trim()))) throw new Error("Use a two- or three-letter ISO language code.");
      normalized[key] = value === "any" && key === "preferredAudioLang" || value === "none" && key === "preferredSubLang" ? value : normalizeLanguage(value);
    }
  }
  if (patch.defaultOutputDir !== undefined) {
    if (patch.defaultOutputDir.length > 32768 || /[\x00-\x1f]/.test(patch.defaultOutputDir)) throw new Error("Invalid default output folder.");
    normalized.defaultOutputDir = patch.defaultOutputDir.trim();
  }
  for (const key of ["playSoundOnComplete", "openFolderOnComplete", "autoRouteWiiDrive"] as const) {
    if (patch[key] !== undefined) {
      if (typeof patch[key] !== "boolean") throw new Error("Completion preferences must be boolean.");
      normalized[key] = patch[key];
    }
  }
  const previous = getPreferences();
  if (normalized.hardwareEncoder !== undefined && normalized.hardwareEncoder !== previous.hardwareEncoder) {
    return savePreferences({ ...normalized, video: { ...previous.video, encoderOverride: backendEncoder(normalized.hardwareEncoder) } });
  }
  return savePreferences(normalized);
}
