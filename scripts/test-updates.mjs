import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const text = await readFile(new URL("../src/updates.ts", import.meta.url), "utf8");
const source = text.replace(/^import .*;\r?\n/gm, "")
  .replace("interface UpdateLifecycle", "const { invoke, check, relaunch, showUpdateReady, plugins } = globalThis.__updaterTest;\ninterface UpdateLifecycle");
const { outputText } = ts.transpileModule(source, {
  compilerOptions: { target: ts.ScriptTarget.ES2020, module: ts.ModuleKind.ESNext },
});
const tick = () => new Promise(resolve => setTimeout(resolve, 5));
async function until(predicate, description) {
  for (let i = 0; i < 100 && !predicate(); i++) await tick();
  assert(predicate(), description);
}
for (const scenario of ["placeholder", "disabled", "none", "offline", "download-failure", "ready", "success", "install-failure", "busy", "native-denied", "cancelled", "cancel-download", "double-click"]) {
  const calls = [];
  let idle = scenario !== "busy";
  let choose = () => {};
  let finishDownload = () => {};
  let reservations = 0;
  globalThis.__updaterTest = {
    plugins: { updater: { pubkey: scenario === "placeholder" ? "YOUR_GENERATED_PUBLIC_KEY_HERE" : "real-test-public-key" } },
    invoke: async (command) => {
      calls.push(command);
      if (command === "updater_is_configured") return scenario !== "disabled";
      if (command === "prepare_update_install") return scenario !== "native-denied" || ++reservations > 1;
      return null;
    },
    check: async () => {
      calls.push("check");
      if (scenario === "offline") throw new Error("network unavailable");
      if (scenario === "none") return null;
      return {
        version: "1.1.1",
        download: async () => {
          calls.push("download");
          if (scenario === "download-failure") throw new Error("bad signature");
          if (scenario === "cancel-download") await new Promise(resolve => { finishDownload = resolve; });
        },
        install: async (options) => {
          calls.push("install"); assert.equal(options.restartAfterInstall, true);
          if (scenario === "install-failure") throw new Error("installation denied");
        },
        close: async () => { calls.push("close"); },
      };
    },
    showUpdateReady: (version, canInstall, signal) => {
      assert.equal(version, "1.1.1");
      calls.push("notice");
      let pending = null;
      const cancel = () => { pending?.(false); pending = null; };
      signal.addEventListener("abort", cancel);
      choose = () => { const next = pending; pending = null; next?.(true); };
      return {
        waitForRestart: () => {
          calls.push("waiting");
          assert.equal(canInstall(), idle);
          return new Promise(resolve => { pending = resolve; });
        },
        setInstalling: () => calls.push("notice:installing"),
        close: () => { signal.removeEventListener("abort", cancel); cancel(); calls.push("notice:close"); },
      };
    },
    relaunch: async () => { calls.push("relaunch"); },
  };
  const { startAutomaticUpdates } = await import(`data:text/javascript;base64,${Buffer.from(outputText).toString("base64")}#${scenario}`);
  const stop = startAutomaticUpdates({ canInstall: () => idle, setInstalling: value => calls.push(`ui:${value}`) });
  if (scenario === "cancel-download") {
    await until(() => calls.includes("download"), scenario);
    stop(); finishDownload();
  } else if (["ready", "success", "install-failure", "busy", "native-denied", "cancelled", "double-click"].includes(scenario)) {
    await until(() => calls.includes("waiting"), scenario);
    assert(!calls.includes("prepare_update_install") && !calls.includes("ui:true"), "Download must not lock encoding or install automatically");
    if (scenario === "ready" || scenario === "cancelled") stop();
    else if (scenario === "busy") {
      choose(); await tick();
      assert(!calls.includes("prepare_update_install"));
      idle = true; await tick();
      assert(!calls.includes("install"), "Finishing encoding must not trigger an unrequested restart");
      choose();
    } else if (scenario === "native-denied") {
      choose(); await tick();
      assert(!calls.includes("install") && !calls.includes("ui:true"));
      choose();
    } else { choose(); if (scenario === "double-click") choose(); }
  }
  const hasResource = !["placeholder", "disabled", "none", "offline"].includes(scenario);
  if (hasResource) await until(() => calls.includes("close"), scenario + " cleanup");
  else await tick();
  if (["success", "busy", "native-denied", "double-click"].includes(scenario)) {
    assert(calls.indexOf("download") < calls.indexOf("notice"));
    assert(calls.indexOf("prepare_update_install") < calls.indexOf("install"));
    assert.equal(calls.filter(c => c === "install").length, 1);
    assert(calls.includes("relaunch") && calls.includes("release_update_install"));
  } else if (scenario === "install-failure") {
    assert(calls.includes("release_update_install") && calls.includes("ui:false"));
    assert(!calls.includes("relaunch"));
  } else assert(!calls.includes("install") && !calls.includes("relaunch"));
  if (["placeholder", "disabled"].includes(scenario)) assert(!calls.includes("check"));
  if (scenario === "cancel-download") assert(!calls.includes("notice"));
  stop();
}
delete globalThis.__updaterTest;
console.log("Updater checks passed: silent download, explicit restart, active-job/native guards, duplicate clicks, offline/signature failures, and resource cleanup.");
