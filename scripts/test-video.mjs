import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const source = await readFile(new URL("../src/video.ts", import.meta.url), "utf8");
const { outputText } = ts.transpileModule(source, {
  compilerOptions: { target: ts.ScriptTarget.ES2020, module: ts.ModuleKind.ESNext },
});
const { defaultVideo, normalizeVideoSettings, isVideoSettings } = await import(
  `data:text/javascript;base64,${Buffer.from(outputText).toString("base64")}`
);
const legacy = normalizeVideoSettings({ aspectRatio: "16:9", resolutionPreset: "854x480", customWidth: 704, customHeight: 400 });
assert.equal(legacy.resolutionPreset, "848x480");
assert.equal(legacy.aspectRatio, "16:9");
assert.equal(legacy.customWidth, 704);
assert.equal(legacy.autoCrop, false);
assert.equal(legacy.encoderOverride, "auto");
const rounded = normalizeVideoSettings({ ...defaultVideo, resolutionPreset: "custom", customWidth: 705, customHeight: 479 });
assert.equal(rounded.customWidth, 706);
assert.equal(rounded.customHeight, 480);
assert.ok(isVideoSettings(rounded));
for (const resolutionPreset of ["640x480", "720x480", "848x480"]) {
  assert.ok(normalizeVideoSettings({ ...defaultVideo, resolutionPreset }));
}
for (const patch of [
  { customWidth: 855 }, { customHeight: 481 }, { customWidth: NaN },
  { customWidth: Infinity }, { customHeight: 0 }, { resolutionPreset: "1080p" },
  { aspectRatio: "invalid" }, { encoderOverride: "untrusted" }, { autoCrop: "true" },
]) assert.equal(normalizeVideoSettings({ ...defaultVideo, ...patch }), null);
assert.equal(normalizeVideoSettings(null), null);
assert.equal(normalizeVideoSettings([]), null);
console.log("WiiMC preset migration, custom even rounding, and settings validation passed.");
