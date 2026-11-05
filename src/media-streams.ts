import { normalizeLanguage, parseAudioStreams, type AudioStream } from "./audio";
export interface SubtitleStream { index: number; streamIndex: number; codecName: string; language: string; title: string | null; supported: boolean }
export interface MediaInfo { durationSeconds?: number | null; audioStreams: AudioStream[]; subtitleStreams: SubtitleStream[] }
export function parseMediaInfo(value: unknown): MediaInfo {
  // Accept the audio-only response from earlier builds during a rolling upgrade.
  if (Array.isArray(value)) return { audioStreams: parseAudioStreams(value), subtitleStreams: [] };
  if (!value || typeof value !== "object") throw new Error("Media metadata is unavailable.");
  const media = value as Partial<MediaInfo>;
  const audioStreams = parseAudioStreams(media.audioStreams);
  if (!Array.isArray(media.subtitleStreams) || media.subtitleStreams.length > 256
    || !media.subtitleStreams.every((s, index) => s && s.index === index && Number.isSafeInteger(s.streamIndex) && s.streamIndex >= 0
      && typeof s.codecName === "string" && s.codecName.length <= 64 && typeof s.language === "string" && s.language.length <= 32
      && (s.title === null || typeof s.title === "string" && s.title.length <= 512) && typeof s.supported === "boolean")) throw new Error("Invalid subtitle metadata.");
  return { audioStreams, subtitleStreams: media.subtitleStreams, durationSeconds: typeof media.durationSeconds === "number" && Number.isFinite(media.durationSeconds) && media.durationSeconds > 0 ? media.durationSeconds : null };
}
export function chooseSubtitleIndex(streams: readonly SubtitleStream[], preferred: string, external: boolean): number | null {
  if (preferred === "none" || external) return null;
  const supported = streams.filter(s => s.supported);
  return (supported.find(s => normalizeLanguage(s.language) === normalizeLanguage(preferred)) ?? supported[0])?.index ?? null;
}
export function subtitleTrackLabel(stream: SubtitleStream): string {
  const language = normalizeLanguage(stream.language);
  const names: Record<string, string> = { eng: "English", jpn: "Japanese", und: "Unknown language", fra: "French", deu: "German", spa: "Spanish", srp: "Serbian" };
  return `Sub ${stream.index + 1}: ${names[language] ?? language.toUpperCase()} (${stream.codecName.toUpperCase()})${stream.title ? ` - ${stream.title}` : ""}${stream.supported ? "" : " (unsupported bitmap/codec)"}`;
}
