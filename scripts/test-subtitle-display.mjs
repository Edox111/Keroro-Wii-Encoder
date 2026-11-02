import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const { outputText } = ts.transpileModule(await readFile(new URL("../src/subtitle-display.ts", import.meta.url), "utf8"), {
  compilerOptions: { target: ts.ScriptTarget.ES2020, module: ts.ModuleKind.ESNext },
});
const { subtitlePresentation, defaultSubtitleLabel } = await import(`data:text/javascript;base64,${Buffer.from(outputText).toString("base64")}`);
const missing = "No matching .srt/.ass/.ssa/.vtt subtitle; encoding without external subtitles.";
const progress = {
  message: `[Warning] Ep03.mkv: ${missing}`, subtitleName: null,
  warnings: [{ fileName: "Ep03.mkv", reason: missing }], errors: [],
};
const before = JSON.stringify(progress);
const neutral = subtitlePresentation(progress, false);
assert.equal(neutral.label, defaultSubtitleLabel);
assert.equal(neutral.highlighted, false);
assert.deepEqual(neutral.warnings, []);
assert.ok(!neutral.message.includes("[Warning]"));
const selected = subtitlePresentation(progress, true);
assert.equal(selected.label, "Subtitles: No external match");
assert.equal(selected.highlighted, true);
assert.deepEqual(selected.warnings, progress.warnings);
const damaged = { fileName: "Ep04.mkv", reason: "Subtitle parse error: invalid timestamps" };
const disk = { fileName: "Ep04.mkv", reason: "Low disk space" };
const mixed = subtitlePresentation({ ...progress, warnings: [...progress.warnings, damaged, disk] }, false);
assert.deepEqual(mixed.warnings, [damaged, disk]);
assert.equal(subtitlePresentation({ ...progress, message: "Batch complete: 1 encoded, 0 skipped, 0 failed, 1 total. 1 warning(s)." }, false).message,
  "Batch complete: 1 encoded, 0 skipped, 0 failed, 1 total.");
assert.ok(subtitlePresentation({ ...progress, warnings: [...progress.warnings, disk],
  message: "Batch complete: 1 encoded, 0 skipped, 0 failed, 1 total. 2 warning(s)." }, false).message.endsWith("1 warning(s)."));
assert.equal(subtitlePresentation({ ...progress, subtitleName: "日本語 Ep03.ass" }, true).label,
  "Subtitles: External — 日本語 Ep03.ass");
assert.equal(JSON.stringify(progress), before, "Presentation must not modify backend state");
console.log("Default subtitle notices are neutral; explicit selections and real warnings stay visible; payloads remain unchanged.");
