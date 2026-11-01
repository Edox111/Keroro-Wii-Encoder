import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const source = await readFile(new URL("../src/naming.ts", import.meta.url), "utf8");
const { outputText } = ts.transpileModule(source, {
  compilerOptions: { target: ts.ScriptTarget.ES2020, module: ts.ModuleKind.ESNext },
});
const { episodeNumberFromFilename, outputNameForInput } = await import(
  `data:text/javascript;base64,${Buffer.from(outputText).toString("base64")}`
);
for (const [filename, episode] of [
  ["[Group] Anime Name - 03.mkv", 3], ["AnimeName_Ep03.mp4", 3],
  ["Anime Name - 3", 3], ["Episode 3.mkv", 3], ["Anime S02E07.webm", 7],
  ["Anime - 03v2 [1080p].mkv", 3], ["Anime 003 (1080p).mkv", 3],
  ["Anime Ep00.mkv", 0], ["Anime - 123.mkv", 123],
  ["Movie 2026 1080.mkv", null], ["Ambiguous 01 02.mkv", null],
  ["Anime Ep03 Ep04.mkv", null], ["Movie.mkv", null],
]) {
  assert.equal(episodeNumberFromFilename(filename), episode, filename);
}
assert.equal(outputNameForInput("My Anime", "D:\\Anime\\[Group] Anime - 03.mkv"), "My Anime - Ep03 [Wii].mp4");
assert.equal(outputNameForInput("My Anime", "/anime/Anime_Ep03.MP4"), "My Anime - Ep03 [Wii].mp4");
assert.equal(outputNameForInput("My Anime", "D:\\Anime Ep07\\Movie.mkv"), "My Anime - Ep01 [Wii].mp4");
assert.equal(outputNameForInput("My Anime", "D:\\Anime Ep07"), "My Anime - Ep01 [Wii].mp4");
assert.equal(outputNameForInput("", "D:\\Anime\\Original - 03.mkv"), "Original - 03_Wii.mp4");
assert.equal(outputNameForInput("My: Anime", "Ep03.mkv"), "My Anime - Ep03 [Wii].mp4");
console.log("Single-file episode extraction, title preservation, folder fallback, and original-name checks passed.");
