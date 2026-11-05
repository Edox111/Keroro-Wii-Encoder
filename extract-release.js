import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.dirname(fileURLToPath(import.meta.url));
const buildDirectory = path.join(root, "src-tauri", "target", "release");
const releaseDirectory = path.join(root, "Release");

function rejectLinks(filePath) {
  for (let current = filePath; ; current = path.dirname(current)) {
    try {
      if (fs.lstatSync(current).isSymbolicLink()) {
        throw new Error(`Refusing a symbolic link or junction: ${current}`);
      }
    } catch (error) {
      if (error.code !== "ENOENT") throw error;
    }
    if (current === root) break;
  }
}

try {
  const version = JSON.parse(fs.readFileSync(path.join(root, "package.json"), "utf8")).version;
  if (typeof version !== "string" || !/^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/.test(version)) throw new Error("Invalid release version.");
  const releaseName = `Keroro-Wii-Encoder-v${version}.exe`;
  rejectLinks(buildDirectory);
  const executable = fs.readdirSync(buildDirectory, { withFileTypes: true })
    .find((entry) => entry.isFile() && entry.name.toLowerCase() === "keroro-wii-encoder.exe");
  if (!executable) throw new Error("Compiled application not found. Run npm run build:release first.");

  const binaries = path.join(root, "src-tauri", "binaries");
  const files = [
    [path.join(buildDirectory, executable.name), releaseName],
    [path.join(binaries, "ffmpeg-x86_64-pc-windows-msvc.exe"), "ffmpeg.exe"],
    [path.join(binaries, "ffprobe-x86_64-pc-windows-msvc.exe"), "ffprobe.exe"],
    [path.join(binaries, "FFmpeg-LICENSE.txt"), "FFmpeg-LICENSE.txt"],
    [path.join(binaries, "FFmpeg-README.txt"), "FFmpeg-README.txt"],
  ];

  rejectLinks(releaseDirectory);
  for (const [source, name] of files) {
    rejectLinks(source);
    if (!fs.statSync(source).isFile() || fs.statSync(source).size === 0) {
      throw new Error(`Missing or empty release input: ${source}`);
    }
    rejectLinks(path.join(releaseDirectory, name));
  }
  fs.mkdirSync(releaseDirectory, { recursive: true });
  for (const [source, name] of files) {
    fs.copyFileSync(source, path.join(releaseDirectory, name));
  }
  console.log(`Release ready: ${path.join(releaseDirectory, releaseName)}`);
  console.log("Keep ffmpeg.exe and ffprobe.exe beside the application.");
} catch (error) {
  console.error(`Release extraction failed: ${error instanceof Error ? error.message : String(error)}`);
  console.error("If the destination executable is locked, exit the app through its tray menu and retry.");
  process.exitCode = 1;
}
