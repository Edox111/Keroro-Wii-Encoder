import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const matchingText = await readFile(new URL("../src/subtitle-matching.ts", import.meta.url), "utf8");
const matchingJs = ts.transpileModule(matchingText, { compilerOptions: { target: ts.ScriptTarget.ES2020, module: ts.ModuleKind.ESNext } }).outputText;
globalThis.__queueMatching = await import(`data:text/javascript;base64,${Buffer.from(matchingJs).toString("base64")}`);
const audioJs = ts.transpileModule(await readFile(new URL("../src/audio.ts", import.meta.url), "utf8"), { compilerOptions: { target: ts.ScriptTarget.ES2020, module: ts.ModuleKind.ESNext } }).outputText;
globalThis.__queueAudio = await import(`data:text/javascript;base64,${Buffer.from(audioJs).toString("base64")}`);
const mediaJs = ts.transpileModule("const { normalizeLanguage, parseAudioStreams } = globalThis.__queueAudio;\n" + (await readFile(new URL("../src/media-streams.ts", import.meta.url), "utf8")).split("\n").slice(1).join("\n"), { compilerOptions: { target: ts.ScriptTarget.ES2020, module: ts.ModuleKind.ESNext } }).outputText;
globalThis.__queueMedia = await import(`data:text/javascript;base64,${Buffer.from(mediaJs).toString("base64")}`);
const driveText = (await readFile(new URL("../src/drive-selector.ts", import.meta.url), "utf8")).split("const gb =")[0].replace(/^import .*;\r?\n/gm, "");
const driveJs = ts.transpileModule(driveText, { compilerOptions: { target: ts.ScriptTarget.ES2020, module: ts.ModuleKind.ESNext } }).outputText;
globalThis.__queueDrive = await import(`data:text/javascript;base64,${Buffer.from(driveJs).toString("base64")}`);
const source = (await readFile(new URL("../src/file-queue.ts", import.meta.url), "utf8"))
  .replace(/^import .*from "\.\/drive-selector";\r?\n/gm, "const { fat32Warning } = globalThis.__queueDrive;\n")
  .replace(/^import .*from "\.\/subtitle-matching";\r?\n/gm, "const { isSubtitleFile, matchSubtitles } = globalThis.__queueMatching;\n")
  .replace(/^import .*from "\.\/media-streams";\r?\n/gm, "const { parseMediaInfo, chooseSubtitleIndex } = globalThis.__queueMedia;\n")
  .replace(/^import .*from "\.\/audio";\r?\n/gm, "const { chooseAudioIndex, parseAudioStreams } = globalThis.__queueAudio;\n")
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

// Audio probing must settle before queue admission; user choices survive re-probes.
const tracks = [
  { index: 0, streamIndex: 1, codecName: "ac3", channels: 6, channelLayout: "5.1", language: "jpn", title: "Japanese Original" },
  { index: 1, streamIndex: 3, codecName: "aac", channels: 2, channelLayout: "stereo", language: "eng", title: "English Dub" },
];
const audioMemory=new Map();
const audioStorage={getItem:k=>audioMemory.get(k)??null,setItem:(k,v)=>audioMemory.set(k,v),removeItem:k=>audioMemory.delete(k)};
const audioQueue=new FileQueue(()=>{},audioStorage);
audioQueue.add(["D:/one.mkv","D:/two.mkv","D:/silent.mkv","D:/broken.mkv"]);
let probes=0,peakProbes=0,releaseProbe;
const gate=new Promise(resolve=>releaseProbe=resolve);
const probing=audioQueue.probePendingAudio(async path=>{probes++;peakProbes=Math.max(peakProbes,probes);await gate;probes--;if(path.includes("broken"))throw Error("Corrupt header");return path.includes("silent")?[]:tracks;},()=>"en");
assert(audioQueue.hasPendingAudioProbes);
let blockedCalls=0;
await audioQueue.run(async()=>{blockedCalls++;return terminal("blocked")},path=>path);
assert.equal(blockedCalls,0,"Start must wait for metadata");
releaseProbe();await probing;
assert.equal(peakProbes,2);assert(!audioQueue.hasPendingAudioProbes);
const noOpProbe=audioQueue.probePendingAudio(async()=>{throw Error("Already probed");},()=>"eng");
assert(!audioQueue.hasPendingAudioProbes,"Adding only subtitles must not create an empty probe task and leave Start locked");await noOpProbe;
assert.deepEqual(audioQueue.snapshot().items.map(i=>i.selectedAudioIndex),[1,1,null,null]);
assert.equal(audioQueue.snapshot().items[3].audioProbeState,"error");
assert.equal(audioQueue.snapshot().items[3].status,"Pending","A probe failure must not discard a video");
const firstAudio=audioQueue.snapshot().items[0];audioQueue.selectAudio(firstAudio.id,0);
audioQueue.reselectAutomaticAudio("jpn");audioQueue.reselectAutomaticAudio("eng");
assert.equal(audioQueue.snapshot().items[0].selectedAudioIndex,0,"Manual choice survives preference changes");
assert.equal(audioQueue.snapshot().items[1].selectedAudioIndex,1);
audioQueue.selectAudio(firstAudio.id,99);assert.equal(audioQueue.snapshot().items[0].selectedAudioIndex,0);
const audioRestored=new FileQueue(()=>{},audioStorage);
await audioRestored.probePendingAudio(async()=>tracks,()=>"eng");
assert.equal(audioRestored.snapshot().items[0].selectedAudioIndex,0);
assert(audioRestored.snapshot().items[0].audioManual);
const removed=new FileQueue(()=>{});removed.add(["D:/removed.mkv"]);
let finishRemoved;
const late=removed.probePendingAudio(()=>new Promise(resolve=>finishRemoved=resolve),()=>"eng");
removed.remove(removed.snapshot().items[0].id);assert(removed.hasPendingAudioProbes);
finishRemoved(tracks);await late;assert(!removed.hasItems);assert(!removed.hasPendingAudioProbes);
console.log("Audio queue admission, bounded probing, fallback, persistence, manual choice, and removal checks passed.");

const subtitleTracks=[
 {index:0,streamIndex:4,codecName:"subrip",language:"jpn",title:"Japanese",supported:true},
 {index:1,streamIndex:5,codecName:"ass",language:"eng",title:"English",supported:true},
 {index:2,streamIndex:6,codecName:"hdmv_pgs_subtitle",language:"eng",title:null,supported:false},
];
const media={audioStreams:tracks,subtitleStreams:subtitleTracks};
const {parseMediaInfo,chooseSubtitleIndex}=globalThis.__queueMedia;
assert.equal(parseMediaInfo(media).subtitleStreams.length,3);
assert.equal(chooseSubtitleIndex(subtitleTracks,"en",false),1);
assert.equal(chooseSubtitleIndex(subtitleTracks,"deu",false),0);
assert.equal(chooseSubtitleIndex(subtitleTracks,"eng",true),null);
assert.equal(chooseSubtitleIndex(subtitleTracks,"none",false),null);
assert.throws(()=>parseMediaInfo({...media,subtitleStreams:[{...subtitleTracks[0],index:7}]}));
const subQueue=new FileQueue(()=>{},audioStorage);subQueue.add(["D:/subs-new/Episode_03.mkv"]);
await subQueue.probePendingAudio(async()=>media,()=>"jpn",()=>"eng");
const subItem=subQueue.snapshot().items.find(i=>i.path.includes("subs-new"));
assert.equal(subItem.selectedSubtitleIndex,1);
subQueue.selectSubtitle(subItem.id,0);subQueue.reselectAutomaticSubtitles("eng");
assert.equal(subQueue.snapshot().items.find(i=>i.id===subItem.id).selectedSubtitleIndex,0,"Manual embedded choice wins");
subQueue.selectSubtitle(subItem.id,null);subQueue.reselectAutomaticSubtitles("eng");
assert.equal(subQueue.snapshot().items.find(i=>i.id===subItem.id).selectedSubtitleIndex,null,"Manual None wins");
subQueue.selectSubtitle(subItem.id,2);assert.equal(subQueue.snapshot().items.find(i=>i.id===subItem.id).selectedSubtitleIndex,null,"Bitmap track cannot be selected");
const subRestored=new FileQueue(()=>{},audioStorage);await subRestored.probePendingAudio(async()=>media,()=>"jpn",()=>"eng");
assert.equal(subRestored.snapshot().items.find(i=>i.path.includes("subs-new")).selectedSubtitleIndex,null,"Manual None survives restart");
const externalQueue=new FileQueue(()=>{});externalQueue.add(["D:/media/Ep01.mkv"]);
await externalQueue.probePendingAudio(async()=>media,()=>"jpn",()=>"eng");
externalQueue.add([], ["D:/external/Ep01.jpn.ass","D:/external/Ep01.eng.srt"], "eng");
externalQueue.reselectAutomaticSubtitles("eng");
assert.equal(externalQueue.snapshot().items[0].subtitlePath,"D:/external/Ep01.eng.srt");
assert.equal(externalQueue.snapshot().items[0].selectedSubtitleIndex,null,"Later external match overrides automatic embedded selection");
externalQueue.reselectAutomaticSubtitles("jpn");assert.equal(externalQueue.snapshot().items[0].subtitlePath,"D:/external/Ep01.jpn.ass");
console.log("Unified probing, subtitle preference/fallback, unsupported tracks, manual None persistence, and language-tagged external matching passed.");

assert(queue.completedNormally,"Whole queue with isolated errors finishes normally");
assert(!stopped.completedNormally,"Cancelled queue must not run completion actions");
assert(!fatal.completedNormally,"Lost process state must not run completion actions");
const cancelledTerminal=new FileQueue(()=>{});cancelledTerminal.add(["D:/cancelled.mkv"]);
await cancelledTerminal.run(async()=>({...terminal("cancelled"),phase:"cancelled",message:"Cancelled"}),path=>path);
assert(!cancelledTerminal.completedNormally,"A last-item cancellation is not normal completion even if all rows are settled");
console.log("Completion lifecycle differentiates settled per-file errors from cancellation/safety stops.");

const orderedSnapshots = [];
const ordered = new FileQueue(snapshot => orderedSnapshots.push(snapshot));
ordered.add(["D:/Ep01.mkv"]);
const orderedRun = ordered.run((item, signal) => ordered.waitForJob("ordered", async () => null, signal), path => path);
await tick();
const stats = (sequence, progressPercent) => ({ jobId: "ordered", sequence, phase: "running", progressPercent, fps: 48, speed: 2, etaSeconds: 30, paused: false });
ordered.observeEncoding(stats(10, 60));
const events = orderedSnapshots.length;
ordered.observeEncoding(stats(9, 40));
ordered.observeEncoding(stats(10, 50));
assert.equal(orderedSnapshots.length, events, "Stale or duplicate stats do not render");
ordered.observe({ ...terminal("ordered"), sequence: 8, phase: "running", batchPercent: 25 });
assert.equal(ordered.snapshot().items[0].percent, 60, "Delayed full progress cannot rewind newer metrics");
ordered.observeEncoding(stats(11, 60));
assert.equal(orderedSnapshots.length, events, "Identical metric values do not render twice");
ordered.observe({ ...terminal("ordered"), sequence: 12 });
await orderedRun;
assert(ordered.completedNormally);
console.log("Progress ordering tests passed: duplicate events and cross-channel regressions are ignored.");

const hardwareQueue = new FileQueue(()=>{});
hardwareQueue.add(["D:/Ep03.mkv"]);
await hardwareQueue.probePendingAudio(async()=>({audioStreams:[],subtitleStreams:[],durationSeconds:10000}),()=>"jpn");
hardwareQueue.setOutputVolume({id:"sd",root:"E:/",label:"Wii",fileSystem:"FAT32",totalBytes:32000000000,availableBytes:16000000000});
assert.match(hardwareQueue.snapshot().items[0].storageWarning,/FAT32 4GB Warning/);
hardwareQueue.setOutputVolume({id:"pc",root:"D:/",label:"PC",fileSystem:"NTFS",totalBytes:32000000000,availableBytes:16000000000});
assert.equal(hardwareQueue.snapshot().items[0].storageWarning,null);
const trace=Array.from({length:100},(_,i)=>`stderr ${i+1}`).join("\n");
await hardwareQueue.run(async()=>({...terminal("bad",true),errors:[{reason:`FFmpeg failed.\n\nFFmpeg stderr:\n${trace}`}]}),()=>"Ep03.mp4");
let failed=hardwareQueue.snapshot().items[0];
assert.equal(failed.message,"FFmpeg failed.");assert(failed.errorLog.endsWith("stderr 100"));
hardwareQueue.retry(failed.id);
assert.equal(hardwareQueue.snapshot().items[0].status,"Pending");assert.equal(hardwareQueue.snapshot().items[0].errorLog,"");
await hardwareQueue.probePendingAudio(async()=>({audioStreams:[],subtitleStreams:[]}),()=>"jpn");
const pausedRun=hardwareQueue.run(async(item,signal)=>hardwareQueue.waitForJob("pause-test",async()=>null,signal),()=>"Ep03.mp4");
await tick();
hardwareQueue.observe({kind:"batch",jobId:"pause-test",sequence:1,phase:"running",paused:true,batchPercent:40});
assert.equal(hardwareQueue.snapshot().items[0].status,"Paused");
hardwareQueue.observe({kind:"batch",jobId:"pause-test",sequence:2,phase:"running",paused:false,batchPercent:40});
assert.equal(hardwareQueue.snapshot().items[0].status,"Encoding");
hardwareQueue.observe(terminal("pause-test"));await pausedRun;
assert.equal(hardwareQueue.snapshot().items[0].status,"Done");
console.log("FAT32 badges, diagnostic tails, failed-item retry, and queue pause/resume passed.");
