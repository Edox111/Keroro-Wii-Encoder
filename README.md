# Keroro Wii Encoder

**Fast, subtitle-aware video encoding for Nintendo Wii homebrew playback.**

![Windows x64](https://img.shields.io/badge/platform-Windows_x64-0078D4)
![Tauri 2](https://img.shields.io/badge/Tauri-2-24C8DB)
![Version 1.0.0](https://img.shields.io/badge/version-1.0.0-22C55E)
[![MIT License](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

Turn anime episodes and other videos into compact **640 × 480 H.264/AAC MP4s** through a desktop interface built with Rust, Tauri 2, and TypeScript. Drop your files, review the subtitles, and let the encoder handle the queue.

[Download for Windows](../../releases) · [User Guide](docs/USER_GUIDE.md) · [Changelog](CHANGELOG.md)

## Features

- **Fast GPU encoding with automatic fallback.** Detects and tests NVIDIA **NVENC**, AMD **AMF**, and Intel **Quick Sync (QSV)**, in that order. If no usable hardware encoder is available, it uses `libx264` with the `superfast` preset. A failed hardware encode can retry on the CPU.
- **Wii-focused H.264 constraints.** Every encoder follows the same SD output limits: Baseline/Main Level 3.0, 8-bit 4:2:0 video, and at most 30 fps. Scaling preserves the source aspect ratio with padding.
- **Fansub-friendly burn-in.** Supports **SRT, ASS, SSA, and VTT**, including UTF-8 normalization of supported legacy text encodings. Matches subtitles by filename or episode number, with a natural-order fallback.
- **Smart drag-and-drop.** Drop individual files or mixed folders. The scanner routes media and subtitles to their fields and recognizes an empty folder as an output destination.
- **Episode-aware filenames.** A single `Anime - 03.mkv` selection can become `Your Title - Ep03 [Wii].mp4`. Folder batches retain sequential numbering; leaving the title blank preserves the source stem with `_Wii` appended.
- **Pause, resume, and recover.** Suspend an active FFmpeg process without discarding its work. Finished files are preserved across interrupted sessions; `.mp4.part` staging keeps incomplete encodes separate from finalized outputs.
- **Preview before committing.** Generate the first 10 seconds to inspect aspect ratio, quality, and subtitle placement.
- **Desktop-friendly controls.** Choose **Full Throttle** or **Background Mode**, watch progress and ETA, inspect encoding statistics, and keep jobs running in the system tray.
- **Custom OLED installer.** A black, high-contrast NSIS setup wizard with custom installer and uninstaller artwork. FFmpeg and ffprobe are bundled, so users need no separate installation or PATH configuration.
- **Local processing.** Media stays on your computer. Diagnostic logging is local, with no application telemetry.

Subtitle rendering and scaling run on the CPU; the GPU handles final video encoding. Actual speed depends on the source, subtitle complexity, hardware, and drivers. See [hardware encoding](docs/HARDWARE_ENCODING.md) and the [synthetic benchmark report](docs/PERFORMANCE_BENCHMARK.md).

## Quick Start / Download

1. Open **[GitHub Releases](../../releases)** and download the Windows x64 **setup `.exe`**.
2. Run the installer. The per-machine installation requests administrator permission.
3. Open **Keroro Wii Encoder** and select an input video or folder and a separate output folder. You can also drag and drop them.
4. Review the subtitle selection, enter an optional anime title, and choose your quality and CPU usage settings.
5. Select **Preview First 10 Seconds**, then **Start Platoon Downscale**.

**Requirements:** Windows x64, the Microsoft Edge WebView2 Runtime, and a compatible Wii homebrew media player for playback. A supported GPU is optional.

For a **portable release**, extract the complete package and keep the application `.exe`, `ffmpeg.exe`, `ffprobe.exe`, and supplied notices together. The portable application is not a single self-contained executable.

Test a preview and a finished episode on your Wii before converting a large collection. Compatibility depends on your media player and setup; desktop checks do not certify physical Wii playback. See the [playback checklist](docs/WII_PLAYBACK_CHECKLIST.md).

## Output Profile

| Setting | Output |
| --- | --- |
| Container | MP4 with fast-start metadata |
| Video | H.264 Baseline; Main when using AMD AMF |
| H.264 level / pixel format | Level 3.0 / `yuv420p` |
| Resolution | 640 × 480, aspect-ratio-preserving padding |
| Frame rate | At most 30 fps |
| Video rate control | 2 Mbps VBV maximum rate; 4 Mb buffer |
| Audio | AAC-LC, stereo, 160 kbps, 44.1 kHz |

**Video inputs:** MP4, MKV, AVI, MOV, WMV, FLV, and WebM.

## Pause and Recovery

During encoding, the action button becomes **Pause**. Select it to suspend the active process; **Resume** continues the same encode.

After an app crash or power interruption, recovery preserves verified finished episodes and restarts the interrupted episode from the beginning. It does not continue encoding from the bytes of an abandoned `.part` file. Existing outputs are preserved; conflicting settings are reported rather than silently overwriting your files.

See the [User Guide](docs/USER_GUIDE.md) for matching, recovery, and output reuse details.

## Build from Source

### Prerequisites

- Windows x64.
- Node.js 22.12 or newer, with npm.
- Rust with the Windows MSVC toolchain.
- Visual Studio C++ Build Tools, including a Windows SDK.
- Microsoft Edge WebView2 Runtime.

From a checkout of this repository, ensure these sidecars are present. If they are not included in your checkout, obtain them from the project's complete portable release and rename them as shown:

```text
src-tauri/binaries/
├── ffmpeg-x86_64-pc-windows-msvc.exe
├── ffprobe-x86_64-pc-windows-msvc.exe
├── FFmpeg-LICENSE.txt
└── FFmpeg-README.txt
```

Install the locked dependencies and start development mode:

```powershell
npm ci
npm run tauri dev
```

Use `npm install` instead of `npm ci` when intentionally updating npm dependencies, and commit the resulting lockfile.

Build the production NSIS installer:

```powershell
npm run tauri build
```

The configured build hook cleans old application release artifacts and rebuilds the frontend. Exit any running release through its tray menu before building.

The setup executable is generated under `src-tauri/target/release/bundle/nsis/`.

To build the installer **and** copy the portable application and its dependencies into `Release/`:

```powershell
npm run build:release
```

Keep `package-lock.json`, Cargo lockfiles, sidecars, license notices, and installer assets available when reproducing a build. The final installer should be tested on a fresh Windows user profile before publication.

## Documentation and Contributions

- [User Guide](docs/USER_GUIDE.md)
- [Hardware Encoding](docs/HARDWARE_ENCODING.md)
- [Encoding Safety and Performance](docs/ENCODING_SAFETY_AND_PERFORMANCE.md)
- [Runtime Architecture and Packaging](docs/RUNTIME_ARCHITECTURE.md)
- [Changelog](CHANGELOG.md)

Bug reports and pull requests are welcome. Include the application version, selected encoder, reproduction steps, and relevant local log excerpts. Review logs before sharing paths or filenames; sample media is only needed when you have permission to share it.

## License and Credits

Original project code is available under the **[MIT License](LICENSE)**, copyright © 2026 Ironclad Platoon. The installer displays the same license through `license.txt`.

Bundled **FFmpeg is separately licensed under GPLv3**; its license and build/source information are in `src-tauri/binaries/FFmpeg-LICENSE.txt` and `src-tauri/binaries/FFmpeg-README.txt`. Third-party dependencies and the adapted NSIS template retain their own licenses and notices. The MIT grant does not relicense third-party artwork, character assets, sounds, or trademarks.

Keroro Wii Encoder is an independent project and is not affiliated with Nintendo or the owners of the featured characters. Process and redistribute only content and assets you have permission to use.
