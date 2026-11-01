export function cleanAnimeTitle(value: string): string {
  return value.replace(/[\p{Cc}<>:"/\\|?*]/gu, " ")
    .replace(/\s+/gu, " ").trim().replace(/[. ]+$/u, "");
}

export function outputNameExample(value: string, episode = 1): string {
  const title = cleanAnimeTitle(value);
  return title ? `${title} - Ep${String(episode).padStart(2, "0")} [Wii].mp4` : "Original filename_Wii.mp4";
}

const VIDEO_EXTENSION = /\.(mp4|mkv|avi|mov|wmv|flv|webm)$/i;
// Keep priority and ambiguity handling aligned with media::episode_number in Rust.
const EPISODE_PATTERNS = [
  /(?:^|[^a-z0-9])s\d{1,2}e(?<ep>\d{1,4})(?:v\d+)?(?=$|[^0-9])/gi,
  /(?:^|[\s._\[\]-])(?:episode|ep|e)[\s._-]*(?<ep>\d{1,4})(?:v\d+)?(?=$|[\s._\[\]-])/gi,
  /(?:^|\s)-\s*(?<ep>\d{1,4})(?:v\d+)?(?=$|[\s._\[\]-])/gi,
  /^(?<ep>\d{1,4})(?:v\d+)?$/gi,
  /(?:^|[\s._\[-])(?<ep>\d{1,4})(?:v\d+)?(?=$|[\s._\]-])/gi,
];

export function episodeNumberFromFilename(path: string): number | null {
  const filename = path.trim().split(/[\\/]/u).pop() ?? "";
  const stem = filename.replace(VIDEO_EXTENSION, "");
  for (const [index, pattern] of EPISODE_PATTERNS.entries()) {
    const candidates = Array.from(stem.matchAll(pattern), (match) => Number(match.groups?.ep))
      .filter((number) => Number.isSafeInteger(number) && !(index === 4 && (
        [360, 480, 576, 720, 1080, 1440, 2160, 4320].includes(number)
        || (number >= 1900 && number <= 2099)
      )));
    if (candidates.length > 1) return null;
    if (candidates.length === 1) return candidates[0];
  }
  return null;
}

export function outputNameForInput(value: string, inputPath: string): string {
  const filename = inputPath.trim().split(/[\\/]/u).pop() ?? "";
  if (!VIDEO_EXTENSION.test(filename)) return outputNameExample(value);
  if (!cleanAnimeTitle(value)) return `${filename.replace(VIDEO_EXTENSION, "")}_Wii.mp4`;
  return outputNameExample(value, episodeNumberFromFilename(filename) ?? 1);
}
