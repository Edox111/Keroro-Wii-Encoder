import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";
const source = await readFile(new URL("../src/subtitle-matching.ts", import.meta.url), "utf8");
const { outputText } = ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2020, module: ts.ModuleKind.ESNext } });
const { matchSubtitles } = await import(`data:text/javascript;base64,${Buffer.from(outputText).toString("base64")}`);
const videos = ["D:/videos/Episode_02.mp4", "D:/videos/Episode_01.mkv"];
const subs = ["D:/subs/Episode_01.ass", "D:/subs/Episode_02.srt"];
for (const videoOrder of [videos, [...videos].reverse()]) {
  for (const subOrder of [subs, [...subs].reverse()]) {
    const pairs = matchSubtitles(videoOrder, subOrder);
    assert(pairs.every(pair => pair.video.includes("Episode_01") ? pair.subtitlePath.endsWith("Episode_01.ass") : pair.subtitlePath.endsWith("Episode_02.srt")));
    assert.deepEqual(pairs.map(pair => pair.video), videoOrder);
  }
}
assert.deepEqual(matchSubtitles(["C.mp4", "A.mp4", "B.mp4"], ["Z.srt", "B.ass"]).map(pair => pair.subtitlePath), [null, "Z.srt", "B.ass"]);
assert.deepEqual(matchSubtitles(["B.mp4", "A.mp4"], ["Y.srt", "X.ass"]).map(pair => pair.subtitlePath), ["Y.srt", "X.ass"]);
assert.deepEqual(matchSubtitles(["Ep2.mp4", "Ep10.mp4"], ["unmatched2.srt", "unmatched1.srt"]).map(pair => pair.subtitlePath), ["unmatched2.srt", "unmatched1.srt"]);
assert.equal(matchSubtitles(["D:\\ケロロ.01.MKV"], ["D:\\字幕\\ケロロ.01.ASS"])[0].subtitlePath, "D:\\字幕\\ケロロ.01.ASS");
assert.equal(matchSubtitles(["EPISODE_01.mp4"], ["episode_01.srt"])[0].subtitlePath, "episode_01.srt");
assert.deepEqual(matchSubtitles(["A.mp4", "B.mp4"], ["A.srt", "A.srt"]).map(pair => pair.subtitlePath), ["A.srt", null]);
assert.equal(matchSubtitles(["A.mp4"], ["not-a-subtitle.mkv"])[0].subtitlePath, null);
assert.deepEqual(videos, ["D:/videos/Episode_02.mp4", "D:/videos/Episode_01.mkv"]);
assert.deepEqual(subs, ["D:/subs/Episode_01.ass", "D:/subs/Episode_02.srt"]);
console.log("Subtitle matching passed: selection-order independence, exact-first reservation, sorted fallback, Unicode, duplicates, and missing files.");
