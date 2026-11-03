import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";
const urls=new Map();
async function moduleUrl(name) {
  if(urls.has(name))return urls.get(name);
  let source=await readFile(new URL(`../src/${name}.ts`,import.meta.url),"utf8");
  let js=ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2020,module:ts.ModuleKind.ESNext}}).outputText;
  for(const match of [...js.matchAll(/from "\.\/([^".]+)"/g)]) js=js.replaceAll(`from "./${match[1]}"`,`from "${await moduleUrl(match[1])}"`);
  const url=`data:text/javascript;base64,${Buffer.from(js).toString("base64")}`;urls.set(name,url);return url;
}
const memory=new Map();
globalThis.localStorage={getItem:key=>memory.get(key)??null,setItem:(key,value)=>memory.set(key,value)};
const audio=await import(await moduleUrl("audio"));
const streams=audio.parseAudioStreams([
 {index:0,streamIndex:2,codecName:"ac3",channels:6,channelLayout:"5.1",language:"jpn",title:"Japanese"},
 {index:1,streamIndex:5,codecName:"aac",channels:2,language:"en",title:null},
]);
assert.equal(audio.chooseAudioIndex(streams,"eng"),1);
assert.equal(audio.chooseAudioIndex(streams,"ja"),0);
assert.equal(audio.chooseAudioIndex(streams,"deu"),0);
assert.equal(audio.chooseAudioIndex([],"eng"),null);
assert.equal(audio.chooseAudioIndex([{...streams[0],language:"und"}],"eng"),0);
assert(audio.audioTrackLabel(streams[0]).includes("Japanese (5.1 AC3)"));
assert.throws(()=>audio.parseAudioStreams([{...streams[0],index:2}]));
assert.deepEqual(audio.subtitleSuggestionForAudio(streams[0],"en"),{language:"eng",recommendSubtitles:true});
assert.equal(audio.subtitleSuggestionForAudio(streams[1],"eng").recommendSubtitles,false);
const settings=await import(await moduleUrl("language-preferences"));
const prefs=await import(await moduleUrl("preferences"));
assert.deepEqual(settings.getLanguagePreferences(),{preferredAudioLang:"eng",preferredSubLang:"eng"});
let changes=0;const unsubscribe=settings.subscribeLanguagePreferences(()=>changes++);
settings.updateLanguagePreferences({preferredAudioLang:"ja",preferredSubLang:"en-US"});
assert.deepEqual(settings.getLanguagePreferences(),{preferredAudioLang:"jpn",preferredSubLang:"eng"});
assert.equal(changes,1);prefs.savePreferences({animeTitle:"Test"});assert.equal(changes,1);
assert.equal(JSON.parse(memory.get("keroro.preferences.v1")).preferredAudioLang,"jpn");
assert.throws(()=>settings.updateLanguagePreferences({preferredAudioLang:"not a language"}));
unsubscribe();settings.updateLanguagePreferences({preferredAudioLang:"eng"});assert.equal(changes,1);
const stable=settings.getLanguagePreferences();assert.equal(stable,settings.getLanguagePreferences(),"Stable snapshot for future useSyncExternalStore");
console.log("Audio language aliases, labels, validation, default fallback, subtitle suggestions, and persistent settings passed.");
