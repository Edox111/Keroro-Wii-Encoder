export const isSubtitleFile = (path: string): boolean => /\.(srt|ass|ssa|vtt)$/i.test(path);
const leaf = (path: string): string => path.split(/[\\/]/).pop() ?? "";
const base = (path: string): string => leaf(path).replace(/\.[^.]+$/, "").normalize("NFC").toLowerCase();
const order = new Intl.Collator("en", { sensitivity: "base", numeric: false });
function compare(a: string, b: string): number {
  return order.compare(leaf(a), leaf(b)) || order.compare(a, b) || (a < b ? -1 : a > b ? 1 : 0);
}

/** Exact matches are reserved globally before unmatched files consume fallbacks. */
const languageAliases: Record<string, string> = { en: "eng", english: "eng", ja: "jpn", japanese: "jpn", fr: "fra", french: "fra", de: "deu", german: "deu", es: "spa", spanish: "spa", sr: "srp", serbian: "srp" };
function subtitleName(path: string) {
  const name = base(path);
  const tag = name.match(/(?:[. _-]|\[)(eng|en|english|jpn|ja|japanese|fra|fr|french|deu|de|german|spa|es|spanish|srp|sr|serbian)\]?$/i);
  return { base: tag ? name.slice(0, tag.index).replace(/[. _-]+$/, "") : name, language: tag ? languageAliases[tag[1]] ?? tag[1] : "und" };
}
export function matchSubtitles(videos: readonly string[], subtitles: readonly string[], preferred?: string) {
  const available = [...new Set(subtitles.filter(isSubtitleFile))].sort(compare);
  const sortedVideos = videos.map((video, index) => ({ video, index })).sort((a, b) => compare(a.video, b.video));
  const matches = new Map<number, string>();
  const used = new Set<string>();
  for (const { video, index } of sortedVideos) {
    const candidates = available.filter(subtitle => !used.has(subtitle)
      && (base(subtitle) === base(video) || preferred && subtitleName(subtitle).base === base(video)));
    const exact = (preferred && candidates.find(subtitle => subtitleName(subtitle).language === (languageAliases[preferred] ?? preferred))) || candidates[0];
    if (exact) { matches.set(index, exact); used.add(exact); }
  }
  let remaining = available.filter(subtitle => !used.has(subtitle));
  if (preferred) {
    const localized = remaining.filter(subtitle => subtitleName(subtitle).language === (languageAliases[preferred] ?? preferred));
    if (localized.length >= videos.length - matches.size) remaining = localized;
  }
  let next = 0;
  for (const { index } of sortedVideos) {
    if (!matches.has(index) && next < remaining.length) matches.set(index, remaining[next++]);
  }
  // Keep the user's video queue order; only matching is alphabetically sorted.
  return videos.map((video, index) => ({ video, subtitlePath: matches.get(index) ?? null }));
}
