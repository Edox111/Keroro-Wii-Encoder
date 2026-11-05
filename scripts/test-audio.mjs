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
assert.deepEqual(settings.getLanguagePreferences(),{hardwareEncoder:"auto",autoRouteWiiDrive:false,preferredAudioLang:"jpn",preferredSubLang:"eng",defaultOutputDir:"",playSoundOnComplete:true,openFolderOnComplete:true});
let changes=0;const unsubscribe=settings.subscribeLanguagePreferences(()=>changes++);
settings.updateLanguagePreferences({preferredAudioLang:"eng",preferredSubLang:"en-US"});
assert.deepEqual(settings.getLanguagePreferences(),{hardwareEncoder:"auto",autoRouteWiiDrive:false,preferredAudioLang:"eng",preferredSubLang:"eng",defaultOutputDir:"",playSoundOnComplete:true,openFolderOnComplete:true});
assert.equal(changes,1);prefs.savePreferences({animeTitle:"Test"});assert.equal(changes,1);
assert.equal(JSON.parse(memory.get("keroro.preferences.v1")).preferredAudioLang,"eng");
assert.throws(()=>settings.updateLanguagePreferences({preferredAudioLang:"not a language"}));
unsubscribe();settings.updateLanguagePreferences({preferredAudioLang:"eng"});assert.equal(changes,1);
const stable=settings.getLanguagePreferences();assert.equal(stable,settings.getLanguagePreferences(),"Stable snapshot for future useSyncExternalStore");
console.log("Audio language aliases, labels, validation, default fallback, subtitle suggestions, and persistent settings passed.");

settings.updateLanguagePreferences({preferredAudioLang:"any",preferredSubLang:"none",defaultOutputDir:"D:/Wii Videos"});
assert.deepEqual(settings.getLanguagePreferences(),{hardwareEncoder:"auto",autoRouteWiiDrive:false,preferredAudioLang:"any",preferredSubLang:"none",defaultOutputDir:"D:/Wii Videos",playSoundOnComplete:true,openFolderOnComplete:true});
const reloaded=await import((await moduleUrl("preferences"))+"#preferences-reload");
assert.equal(reloaded.getPreferences().preferredSubLang,"none");
assert.equal(reloaded.getPreferences().defaultOutputDir,"D:/Wii Videos");
assert.throws(()=>settings.updateLanguagePreferences({defaultOutputDir:"bad"+String.fromCharCode(0)}));
console.log("Any/None settings and default output persistence passed.");

settings.updateLanguagePreferences({playSoundOnComplete:false,openFolderOnComplete:false});
const savedActions=JSON.parse(memory.get("keroro.preferences.v1"));
assert.equal(savedActions.playSoundOnComplete,false);assert.equal(savedActions.openFolderOnComplete,false);
const actionReload=await import((await moduleUrl("preferences"))+"#completion-reload");
assert.equal(actionReload.getPreferences().playSoundOnComplete,false);assert.equal(actionReload.getPreferences().openFolderOnComplete,false);
assert.throws(()=>settings.updateLanguagePreferences({playSoundOnComplete:"yes"}));
console.log("Completion action defaults, validation, and false-value persistence passed.");

settings.updateLanguagePreferences({hardwareEncoder:"qsv",autoRouteWiiDrive:true});
assert.equal(prefs.getPreferences().hardwareEncoder,"qsv");
assert.equal(prefs.getPreferences().video.encoderOverride,"qsv");
assert.equal(JSON.parse(memory.get("keroro.preferences.v1")).autoRouteWiiDrive,true);
assert.throws(()=>settings.updateLanguagePreferences({hardwareEncoder:"injected"}));
prefs.savePreferences({video:{...prefs.getPreferences().video,encoderOverride:"cpu"}});
settings.updateLanguagePreferences({hardwareEncoder:"qsv",playSoundOnComplete:true});
assert.equal(prefs.getPreferences().video.encoderOverride,"cpu","Saving unrelated preferences must preserve the advanced encoder override");
settings.updateLanguagePreferences({hardwareEncoder:"nvenc"});
assert.equal(prefs.getPreferences().video.encoderOverride,"nvenc","An explicit encoder preference change updates the default");
console.log("Hardware preference changes preserve unrelated manual overrides.");
