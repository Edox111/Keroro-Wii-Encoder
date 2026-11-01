import { existsSync, lstatSync, readdirSync, rmSync } from "node:fs";
import { dirname, isAbsolute, join, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const root = resolve(fileURLToPath(new URL("../", import.meta.url)));
const args = process.argv.slice(2);
if (args.some((arg) => arg !== "--dry-run")) {
  throw new Error("Usage: npm run clean -- [--dry-run]");
}
const dryRun = args.includes("--dry-run");

function run(command, args, capture = false) {
  const result = spawnSync(command, args, {
    cwd: root,
    shell: false,
    windowsHide: true,
    encoding: "utf8",
    stdio: capture ? ["ignore", "pipe", "pipe"] : "inherit",
  });
  if (result.error) throw result.error;
  if (result.status !== 0) {
    throw new Error(`${command} failed (${result.status ?? result.signal}). ${result.stderr ?? ""}`);
  }
  return result.stdout;
}

function checkedPath(path) {
  const absolute = resolve(path);
  const child = relative(root, absolute);
  if (!child || isAbsolute(child) || child === ".." || child.startsWith(`..${sep}`)) {
    throw new Error(`Cleanup target must stay inside this project: ${absolute}`);
  }
  for (let current = absolute; ; current = dirname(current)) {
    if (statOrMissing(current)?.isSymbolicLink()) {
      throw new Error(`Cleanup refuses symbolic links or junctions: ${current}`);
    }
    if (current === root) break;
  }
  return absolute;
}

function statOrMissing(path) {
  try { return lstatSync(path); }
  catch (error) {
    if (error.code === "ENOENT") return null;
    throw error;
  }
}

function checkTree(path) {
  checkedPath(path);
  if (!statOrMissing(path)) return;
  const pending = [path];
  while (pending.length) {
    const current = pending.pop();
    const stat = lstatSync(current);
    if (stat.isSymbolicLink()) throw new Error(`Cleanup refuses symbolic links or junctions: ${current}`);
    if (stat.isDirectory()) {
      for (const entry of readdirSync(current, { withFileTypes: true })) {
        if (entry.isSymbolicLink()) throw new Error(`Cleanup refuses symbolic links or junctions: ${join(current, entry.name)}`);
        if (entry.isDirectory()) pending.push(join(current, entry.name));
      }
    }
  }
}

try {
  const manifests = ["src-tauri/Cargo.toml", "src-tauri/crates/encoder-core/Cargo.toml"];
  const targets = new Map();
  for (const manifest of manifests) {
    const metadata = JSON.parse(run("cargo", ["metadata", "--offline", "--no-deps", "--format-version", "1", "--manifest-path", manifest], true));
    const target = checkedPath(metadata.target_directory);
    if (!targets.has(target)) targets.set(target, manifest);
  }
  const viteCaches = ["node_modules/.vite", "node_modules/.vite-temp"].map((path) => checkedPath(join(root, path)));
  // Validate every target before any cleanup. Never follow a redirected target or a junction.
  [...targets.keys(), ...viteCaches].forEach(checkTree);
  const npmCli = process.env.npm_execpath;
  if (!npmCli || !existsSync(npmCli)) throw new Error("Run this helper through npm run clean.");

  console.log("Clean npm's shared cache: npm cache clean --force");
  viteCaches.forEach((path) => console.log(`Remove Vite cache: ${path}`));
  targets.forEach((manifest, target) => console.log(`Cargo clean: ${manifest} -> ${target}`));
  if (!dryRun) {
    // Invoke npm's JS entry point directly; npm.cmd needs no shell quoting on Windows.
    run(process.execPath, [npmCli, "cache", "clean", "--force"]);
    for (const path of viteCaches) {
      checkTree(path);
      rmSync(path, { recursive: true, force: true });
    }
    for (const [target, manifest] of targets) {
      checkTree(target);
      run("cargo", ["clean", "--manifest-path", manifest, "--target-dir", target]);
    }
  }
} catch (error) {
  console.error(`Cleanup stopped: ${error instanceof Error ? error.message : String(error)}`);
  console.error("If an application executable is locked, exit it through its tray menu; do not interrupt an encoding job.");
  process.exitCode = 1;
}
