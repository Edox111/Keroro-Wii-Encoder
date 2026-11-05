import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

class Element {
  children = []; dataset = {}; hidden = false; disabled = false;
  textContent = ""; value = ""; title = ""; replacements = 0;
  append(...children) { this.children.push(...children); }
  replaceChildren(...children) { this.children = children; this.replacements++; }
  setAttribute() {}
  addEventListener() {}
  removeEventListener() {}
  remove() {}
}
const elements = new Map(["fileQueue", "queueList", "queueSummary", "clearQueueBtn", "queueProgress", "queueProgressLabel", "queueOverall"].map(id => [`#${id}`, new Element()]));
globalThis.document = { querySelector: selector => elements.get(selector), createElement: () => new Element(), body: new Element() };
const frames = new Map(); let nextFrame = 0;
globalThis.requestAnimationFrame = callback => { frames.set(++nextFrame, callback); return nextFrame; };
globalThis.cancelAnimationFrame = id => frames.delete(id);
const flush = () => { const work = [...frames.values()]; frames.clear(); work.forEach(callback => callback()); };
const source = (await readFile(new URL("../src/queue-view.ts", import.meta.url), "utf8"))
  .replace(/^import .*;\r?\n/gm, "");
const helpers = `const fileName = path => path;
const audioTrackLabel = stream => 'Audio ' + stream.index;
const subtitleTrackLabel = stream => 'Sub ' + stream.index;
const performanceLabel = percent => percent + '%';\n`;
const js = ts.transpileModule(helpers + source, { compilerOptions: { target: ts.ScriptTarget.ES2020, module: ts.ModuleKind.ESNext } }).outputText;
const { initializeQueueView } = await import(`data:text/javascript;base64,${Buffer.from(js).toString("base64")}`);
const view = initializeQueueView(() => {}, () => {}, () => {}, () => {});
const item = { id: "one", path: "Ep01.mkv", status: "Pending", message: "", percent: 0, audioProbeState: "ready", audioStreams: [{ index: 0 }, { index: 1 }], subtitleStreams: [{ index: 0, supported: true }, { index: 1, supported: true }], selectedAudioIndex: 0, selectedSubtitleIndex: 0 };
const state = { items: [item], running: false, complete: false, label: "1 video", percent: 0 };
view.render(state); flush();
const list = elements.get("#queueList");
const row = list.children[0], [audio, subtitle] = row.children[3].children;
const audioOption = audio.children[1], subtitleOption = subtitle.children[2];
view.render({ ...state, items: [{ ...item, selectedAudioIndex: 1, selectedSubtitleIndex: 1 }] }); flush();
assert.equal(audio.value, "1"); assert.equal(subtitle.value, "1");
assert.equal(audio.children[1], audioOption, "Changing selection preserves option nodes");
assert.equal(subtitle.children[2], subtitleOption);
for (let percent = 1; percent <= 100; percent++) {
  view.render({ ...state, running: true, percent, items: [{ ...item, status: "Encoding", percent }] });
}
assert.equal(frames.size, 1, "A burst of progress schedules one paint");
flush();
assert.equal(row.children[5].textContent, "100%");
assert.equal(audio.replacements, 1, "Progress does not rebuild track options");
assert.equal(subtitle.replacements, 1);
view.render({ ...state, running: true, items: [{ ...item, status: "Encoding" }] }); view.dispose();
assert.equal(frames.size, 0, "Disposal cancels pending rendering");
console.log("Queue view tests passed: one paint per frame, stable selections, no option churn, and cleanup.");
