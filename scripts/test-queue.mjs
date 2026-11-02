import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const matchingText = await readFile(new URL("../src/subtitle-matching.ts", import.meta.url), "utf8");
const matchingJs = ts.transpileModule(matchingText, { compilerOptions: { target: ts.ScriptTarget.ES2020, module: ts.ModuleKind.ESNext } }).outputText;
globalThis.__queueMatching = await import(`data:text/javascript;base64,${Buffer.from(matchingJs).toString("base64")}`);
const source = (await readFile(new URL("../src/file-queue.ts", import.meta.url), "utf8"))
  .replace(/^import .*from "\.\/subtitle-matching";\r?\n/gm, "const { isSubtitleFile, matchSubtitles } = globalThis.__queueMatching;\n")
  .replace(/^import .*from "\.\/batch";\r?\n/gm, 'const isTerminal = phase => ["succeeded", "failed", "completed_with_errors", "cancelled"].includes(phase);\n');
const { outputText } = ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2020, module: ts.ModuleKind.ESNext } });
const { FileQueue, QueueSafetyError, parentDirectory } = await import(`data:text/javascript;base64,${Buffer.from(outputText).toString("base64")}`);
const tick = () => new Promise(resolve => setTimeout(resolve, 5));
async function until(predicate) { for(let i=0;i<100&&!predicate();i++) await tick(); assert(predicate()); }
const terminal = (jobId, failed = false) => ({ kind: "batch", jobId, sequence: 5, phase: failed ? "failed" : "succeeded", batchPercent: 100, message: "Finished", errors: failed ? [{reason:"Corrupt input"}] : [] });
const memory = new Map();
const storage = { getItem: k => memory.get(k) ?? null, setItem: (k,v) => memory.set(k,v), removeItem: k => memory.delete(k) };
const snapshots=[];
const queue = new FileQueue(state=>snapshots.push(state),storage);
queue.add(["D:/anime/Ep01.mkv", "D:/anime/Ep02.mp4", "D:/anime/Ep03.avi", "D:/anime/Ep04.webm", "D:/anime/EP01.MKV", "D:/subs/Ep01.srt"]);
assert.equal(queue.snapshot().items.length,4);
let submits=0, active=0, maximum=0;
const running = queue.run(async (item,signal)=>{
  ++submits; active++;maximum=Math.max(maximum,active);
  const jobId=`job${submits}`;
  if(submits===2){queue.observe(terminal(jobId,true));active--;}
  const result=await queue.waitForJob(jobId,async()=>null,signal);
  if(submits!==2)active--;
  return result;
}, path=>path);
await until(()=>submits===1);
assert(queue.running);
assert.equal(queue.snapshot().items[0].status,"Encoding");
queue.remove(queue.snapshot().items[0].id);
assert.equal(queue.snapshot().items.length,4,"Cannot remove active item");
queue.remove(queue.snapshot().items[3].id);
queue.observe({...terminal("job1"), sequence:3,phase:"running",batchPercent:50});
assert(Math.abs(queue.snapshot().percent-50/3)<0.00001);
await tick();assert.equal(submits,1,"Acknowledgement/progress must not start the next job");
queue.observe(terminal("unrelated"));await tick();assert.equal(submits,1);
queue.observe(terminal("job1"));
await until(()=>submits===3);
assert.equal(maximum,1);
assert.deepEqual(queue.snapshot().items.map(item=>item.status),["Done","Error","Encoding"]);
queue.observe(terminal("job3"));await running;
assert(queue.snapshot().complete);
assert.equal(queue.snapshot().items[1].message,"Corrupt input");
assert.equal(queue.snapshot().percent,100);
const restored=new FileQueue(()=>{},storage);
assert.deepEqual(restored.snapshot().items.map(item=>item.status),["Done","Error","Done"]);
queue.clear();assert(!queue.hasItems);assert.equal(memory.size,0);

const stopped=new FileQueue(()=>{},storage);stopped.add(["D:/Ep01.mkv","D:/Ep02.mkv"]);
const stopping=stopped.run((item,signal)=>stopped.waitForJob("stop",async()=>null,signal),path=>path);
await tick();stopped.stop();await stopping;
assert.deepEqual(stopped.snapshot().items.map(item=>item.status),["Pending","Pending"]);

let calls=0;
const isolation=new FileQueue(()=>{});isolation.add(["D:/bad.mkv","D:/good.mp4"]);
await isolation.run(async()=>{if(++calls===1)throw new Error("Unreadable file");return terminal("ok")},path=>path);
assert.deepEqual(isolation.snapshot().items.map(item=>item.status),["Error","Done"]);
const fatal=new FileQueue(()=>{});fatal.add(["D:/Ep01.mkv","D:/Ep02.mkv"]);
await fatal.run(async()=>{throw new QueueSafetyError("Lost native state")},path=>path);
assert.deepEqual(fatal.snapshot().items.map(item=>item.status),["Error","Pending"]);
const uncompleted=new FileQueue(()=>{});uncompleted.add(["D:/Ep01.mkv","D:/Ep02.mkv"]);
await uncompleted.run(async()=>({...terminal("not-terminal"),phase:"running"}),path=>path);
assert.deepEqual(uncompleted.snapshot().items.map(item=>item.status),["Error","Pending"]);
const checkpoint=new FileQueue(()=>{});checkpoint.add(["D:/Ep01.mkv","D:/Ep02.mkv"]);
await checkpoint.run(async()=>({...terminal("checkpoint"),warnings:[{fileName:"Recovery state",reason:"Cannot clear checkpoint"}]}),path=>path);
assert.deepEqual(checkpoint.snapshot().items.map(item=>item.status),["Error","Pending"]);
const collisions=new FileQueue(()=>{});collisions.add(["D:/MovieA.mkv","D:/MovieB.mkv"]);
calls=0;await collisions.run(async()=>{calls++;return terminal("name")},()=>"Same.mp4");
assert.equal(calls,1);assert.equal(collisions.snapshot().items[1].status,"Error");
assert.equal(parentDirectory("D:\\Ep03.mkv"),"D:\\");
assert.equal(parentDirectory("D:/anime/Ep03.mkv"),"D:/anime");
assert.equal(parentDirectory(String.raw`\\?\D:\Ep03.mkv`),String.raw`\\?\D:`+"\\");
assert.equal(parentDirectory("/Ep03.mkv"),"/");
console.log("Queue tests passed: sequential completion, early/stale events, removal, per-file errors, safe stop, persistence, collisions, and drive-root paths.");

memory.clear();
const paired = new FileQueue(()=>{},storage);
paired.add(["D:/Episode_02.mp4", "D:/Episode_01.mp4"], ["D:/subs/Episode_01.ass", "D:/subs/Episode_02.srt"]);
assert.deepEqual(paired.snapshot().items.map(i=>i.subtitlePath),["D:/subs/Episode_02.srt","D:/subs/Episode_01.ass"]);
const recoveredPairs = new FileQueue(()=>{},storage);
assert.deepEqual(recoveredPairs.snapshot().items.map(i=>i.subtitlePath),paired.snapshot().items.map(i=>i.subtitlePath));
recoveredPairs.clearSubtitleMatches();
assert(recoveredPairs.snapshot().items.every(i=>i.subtitlePath===undefined));
const later = new FileQueue(()=>{});
later.add(["D:/B.mp4","D:/A.mp4"]);
later.add([], ["D:/Z.srt","D:/Y.ass"]);
assert.deepEqual(later.snapshot().items.map(i=>i.subtitlePath),["D:/Z.srt","D:/Y.ass"]);
const missingPair = new FileQueue(()=>{});
missingPair.add(["D:/A.mp4","D:/B.mp4"], ["D:/A.ass"]);
assert.deepEqual(missingPair.snapshot().items.map(i=>i.subtitlePath),["D:/A.ass",null]);
delete globalThis.__queueMatching;
console.log("Per-video subtitle assignments, independent sorted fallback, later subtitle drops, missing matches, and persistence passed.");
