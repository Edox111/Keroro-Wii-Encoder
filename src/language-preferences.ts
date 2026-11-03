import { getPreferences, savePreferences, subscribePreferences } from "./preferences";
import { normalizeLanguage } from "./audio";

export interface LanguagePreferences { preferredAudioLang: string; preferredSubLang: string }
let current: Readonly<LanguagePreferences> = Object.freeze({ preferredAudioLang: getPreferences().preferredAudioLang, preferredSubLang: getPreferences().preferredSubLang });
const listeners = new Set<() => void>();
subscribePreferences(() => {
  const next = getPreferences();
  if (next.preferredAudioLang === current.preferredAudioLang && next.preferredSubLang === current.preferredSubLang) return;
  current = Object.freeze({ preferredAudioLang: next.preferredAudioLang, preferredSubLang: next.preferredSubLang });
  listeners.forEach(listener => listener());
});
export const getLanguagePreferences = (): Readonly<LanguagePreferences> => current;
export function subscribeLanguagePreferences(listener: () => void): () => void {
  listeners.add(listener);
  return () => { listeners.delete(listener); };
}
export function updateLanguagePreferences(patch: Partial<LanguagePreferences>): boolean {
  const normalized: Partial<LanguagePreferences> = {};
  for (const key of ["preferredAudioLang", "preferredSubLang"] as const) {
    if (patch[key] !== undefined) {
      const value = patch[key];
      if (value.length > 16 || !/^[a-z]{2,3}(?:[-_][a-z]{2})?$/i.test(value.trim())) throw new Error("Use a two- or three-letter ISO language code.");
      normalized[key] = normalizeLanguage(value);
    }
  }
  return savePreferences(normalized);
}
