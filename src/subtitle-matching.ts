export const isSubtitleFile = (path: string): boolean => /\.(srt|ass|ssa|vtt)$/i.test(path);
const leaf = (path: string): string => path.split(/[\\/]/).pop() ?? "";
const base = (path: string): string => leaf(path).replace(/\.[^.]+$/, "").normalize("NFC").toLowerCase();
const order = new Intl.Collator("en", { sensitivity: "base", numeric: false });
function compare(a: string, b: string): number {
  return order.compare(leaf(a), leaf(b)) || order.compare(a, b) || (a < b ? -1 : a > b ? 1 : 0);
}

/** Exact matches are reserved globally before unmatched files consume fallbacks. */
export function matchSubtitles(videos: readonly string[], subtitles: readonly string[]) {
  const available = [...new Set(subtitles.filter(isSubtitleFile))].sort(compare);
  const sortedVideos = videos.map((video, index) => ({ video, index })).sort((a, b) => compare(a.video, b.video));
  const matches = new Map<number, string>();
  const used = new Set<string>();
  for (const { video, index } of sortedVideos) {
    const exact = available.find(subtitle => !used.has(subtitle) && base(subtitle) === base(video));
    if (exact) { matches.set(index, exact); used.add(exact); }
  }
  const remaining = available.filter(subtitle => !used.has(subtitle));
  let next = 0;
  for (const { index } of sortedVideos) {
    if (!matches.has(index) && next < remaining.length) matches.set(index, remaining[next++]);
  }
  // Keep the user's video queue order; only matching is alphabetically sorted.
  return videos.map((video, index) => ({ video, subtitlePath: matches.get(index) ?? null }));
}
