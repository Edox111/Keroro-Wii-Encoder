import type { BatchProgress } from "./batch";

const optionalSubtitleNotice = "No matching .srt/.ass/.ssa/.vtt subtitle; encoding without external subtitles.";
export const defaultSubtitleLabel = "Subtitles: No external file (default)";

/** Presentation only. Never infer embedded tracks or modify the encoding request. */
export function subtitlePresentation(progress: BatchProgress, explicitlySelected: boolean) {
  const warnings = (progress.warnings ?? []).filter((entry) =>
    explicitlySelected || entry.reason !== optionalSubtitleNotice);
  let message = !explicitlySelected && progress.message.endsWith(optionalSubtitleNotice)
    ? "Encoding without external subtitles."
    : progress.message;
  if (warnings.length !== (progress.warnings ?? []).length
    && /^Batch complete: \d+ encoded, \d+ skipped, \d+ failed, \d+ total\. \d+ warning\(s\)\.$/.test(message)) {
    message = message.replace(/ \d+ warning\(s\)\.$/, warnings.length ? ` ${warnings.length} warning(s).` : "");
  }
  return {
    warnings, message,
    label: progress.subtitleName ? `Subtitles: External — ${progress.subtitleName}`
      : explicitlySelected ? progress.currentFile === 0 ? "Subtitles: External source selected"
        : "Subtitles: No external match" : defaultSubtitleLabel,
    highlighted: explicitlySelected,
  };
}
