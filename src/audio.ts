export interface AudioStream {
  /** Zero-based ordinal used by FFmpeg 0:a:N. */
  index: number; streamIndex: number; codecName: string; channels: number;
  channelLayout?: string | null; language: string; title: string | null;
}
const aliases: Record<string, string> = { en: "eng", ja: "jpn", fr: "fra", fre: "fra", de: "deu", ger: "deu", es: "spa", it: "ita", pt: "por", ru: "rus", zh: "zho", chi: "zho", ko: "kor", sr: "srp", hr: "hrv" };
export function normalizeLanguage(value: string): string {
  const code = value.trim().toLowerCase().split(/[-_]/)[0];
  return /^[a-z]{2,3}$/.test(code) ? aliases[code] ?? code : "und";
}
export function chooseAudioIndex(streams: readonly AudioStream[], preferred: string): number | null {
  return (streams.find(stream => normalizeLanguage(stream.language) === normalizeLanguage(preferred)) ?? streams[0])?.index ?? null;
}
export function parseAudioStreams(value: unknown): AudioStream[] {
  if (!Array.isArray(value) || value.length > 256) throw new Error("Audio metadata is unavailable.");
  if (!value.every((stream, position) => stream && stream.index === position
    && Number.isSafeInteger(stream.streamIndex) && stream.streamIndex >= 0
    && typeof stream.codecName === "string" && stream.codecName.length <= 64
    && Number.isSafeInteger(stream.channels) && stream.channels >= 0
    && typeof stream.language === "string" && stream.language.length <= 32
    && (stream.title === null || typeof stream.title === "string" && stream.title.length <= 512)
    && (stream.channelLayout == null || typeof stream.channelLayout === "string" && stream.channelLayout.length <= 64))) throw new Error("Invalid audio stream metadata.");
  return value;
}
export function audioTrackLabel(stream: AudioStream): string {
  const names: Record<string, string> = { eng: "English", jpn: "Japanese", fra: "French", deu: "German", spa: "Spanish", ita: "Italian", por: "Portuguese", rus: "Russian", zho: "Chinese", kor: "Korean", srp: "Serbian", hrv: "Croatian", und: "Unknown language" };
  const language = normalizeLanguage(stream.language);
  const channels = stream.channelLayout?.startsWith("5.1") ? "5.1" : stream.channelLayout?.startsWith("7.1") ? "7.1"
    : stream.channels === 2 ? "2.0" : stream.channels === 1 ? "1.0" : stream.channels ? `${stream.channels} ch` : "channels unknown";
  return `🔊 Audio ${stream.index + 1}: ${names[language] ?? language.toUpperCase()} (${channels} ${stream.codecName.toUpperCase()})${stream.title ? ` — ${stream.title}` : ""}`;
}
/** Future settings UI may show this suggestion; it never changes subtitle routing. */
export function subtitleSuggestionForAudio(stream: AudioStream | undefined, preferredSubLang: string) {
  const language = normalizeLanguage(preferredSubLang);
  const audio = normalizeLanguage(stream?.language ?? "und");
  return { language, recommendSubtitles: audio !== "und" && audio !== language };
}
