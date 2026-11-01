# Wave 3: Subtitle traps — Ironclad Platoon

Current dependency resolution and native compilation now pass. See the latest
verification in [UI_LIFECYCLE.md](UI_LIFECYCLE.md); older blocker notes below are
historical, and no final release installer has been rebuilt.

The implementation is applied to the existing project. The Tauri commands in
`src-tauri/src/lib.rs` continue to call the testable media core; subtitle scanning,
normalization, and staging live in `src-tauri/crates/encoder-core/src/`.
[V1_1_CODE.md](V1_1_CODE.md) contains complete current source blocks and manifests.

## Supported files and canonical paths

`subtitles::EXTENSIONS` is the single SRT/ASS/SSA/VTT whitelist. Matching remains
case-insensitive with exact stems preferred over unique episode numbers.
Ambiguous matches remain per-file errors.

The selected subtitle directory is canonicalized and verified as a directory
before scanning. Symlink entries are excluded by the existing regular-file
scan. Immediately before reading, the matched file is canonicalized again and
its parent must equal the selected canonical root. A file resolving outside that
folder is not read; it produces a warning and video-only encoding.

Canonicalization resolves `..` and links; it is **not** an OS sandbox or a general
authorization policy. The native folder picker intentionally allows the user to
choose any accessible folder. Same-account attackers able to mutate selected
directories remain outside a complete filesystem confinement guarantee.

## Size and encoding guards

The limit is exactly **50,000,000 bytes**, decimal 50 MB. `fs::metadata` checks size
before opening/reading. The opened handle's metadata is checked again, and the
read is bounded to limit + 1 byte to detect growth. Oversized files are skipped
as subtitles with a warning; the video is still encoded. These limits bound input
processing but cannot guarantee immunity to every possible memory exhaustion.

`encoding_rs` strictly decodes BOM-marked UTF-16 and legacy Shift-JIS/Windows-1252
into UTF-8. UTF-8 is checked directly first. No malformed-sequence replacement
characters are silently accepted. BOM-less legacy encodings remain ambiguous;
guesses are reported, and incorrect guesses require saving the original as
UTF-8. This normalization now works across platforms and replaces the earlier
Windows code-page FFI helper.

Header and timed-cue checks distinguish structural syntax errors from unavailable
subtitles. ASS/SSA require script/event headers; SRT/VTT timestamps and ASS/SSA
dialogue times must parse and have non-reversed ranges. This is a lightweight
structural check, not a complete ASS styling or HTML-tag parser. Some unusual
subtitle syntax may require correction. FFmpeg still performs full parsing.

## Isolated temporary storage and lifetime

`tempfile::TempDir` and `NamedTempFile` own the UTF-8 copy. The application adds
independent 128-bit OS-random prefixes to directory and file names with
`getrandom`; it does not assume `tempfile`'s default PRNG is cryptographic.
Unix directories explicitly use mode 0700; named files use tempfile's private
defaults. Windows uses its per-user temporary location and inherited ACLs.

FFmpeg's child working directory is this isolated directory. Only a whitelisted
ASCII temporary basename enters the fixed subtitle filter; no user path enters
filter syntax. Video input and MP4 output remain separate absolute native argv
values. Original subtitles are never rewritten.

The `ProcessLease` retains temporary owners through process exit/reaping. An
error, worker panic, or worker abortion transfers them to the reaper, preventing
premature deletion while FFmpeg reads. The reaper cleans temporary resources
before releasing the process slot. `tempfile` also supplies automatic Drop
cleanup. As with any destructor-based scheme, forced process termination or OS
cleanup failures can leave files; this is not a crash-proof deletion promise.

## Missing files, parser errors, and the UI

- Missing, oversized, unreadable, binary, or unsupported-encoding subtitles:
  retained warning, no subtitle filter for that iteration, encode the video.
- Structurally malformed subtitles: retained syntax error, skip that video,
  continue with the next item.
- FFmpeg nonzero exit/crash while burning subtitles: retain the bounded stderr
  tail, report a subtitle/filter-or-media failure, clean up, continue the queue.
- Successful FFmpeg exit with error-level stderr: retain a warning rather than
  silently discarding diagnostics.

`diagnostics.rs` drains stderr in fixed-size chunks, keeps at most 24 bounded
lines, and truncates oversized lines. A malicious log line cannot cause the old
unbounded `lines()` allocation. The frontend's existing sequenced listener
retains warnings/errors as text and now opens the diagnostics console when new
entries arrive. Rendering never uses `innerHTML` for filenames or stderr.

## Command-injection scope

The runtime audit found no `cmd.exe`, PowerShell, `sh -c`, or other shell launcher
in the FFmpeg/ffprobe paths. Only bundled executable paths reach `Command::new`.
All options are fixed and all user paths are native argument values. Random
subtitle basenames are ASCII-whitelisted before insertion into filter syntax.

This prevents **shell metacharacter command injection through these invocation
paths**. It does not prove that FFmpeg, libass, fonts, codecs, or the application
have no exploitable bugs. No blanket "cannot be exploited" guarantee is made.

## Validation status and dependency blocker

Completed:

- TypeScript/Vite production build and browser diagnostic-rendering checks.
- Six isolated component tests using the actual staging/process/diagnostic
  definitions: normal/error/panic/abort process cleanup, SSA/VTT temp-file reads
  by real FFmpeg, and bounded long stderr lines.
- Rust formatting and source audit of runtime subprocess launches.

Pending: full application compilation, decoder tests, and the integrated batch
regression suite. `encoding_rs` was not cached, and the execution environment's
crates.io DNS lookup failed. The two application lockfiles must be refreshed by
Cargo once dependencies are available; they were not hand-edited or presented
as current locks.

From the project root, in a network-connected terminal:

```powershell
cargo fetch --manifest-path src-tauri/crates/encoder-core/Cargo.toml
cargo fetch --manifest-path src-tauri/Cargo.toml

$env:KERORO_TEST_FFMPEG = (Resolve-Path 'src-tauri/binaries/ffmpeg-x86_64-pc-windows-msvc.exe').Path
$env:KERORO_TEST_FFPROBE = (Resolve-Path 'src-tauri/binaries/ffprobe-x86_64-pc-windows-msvc.exe').Path
cargo test --manifest-path src-tauri/crates/encoder-core/Cargo.toml -- --include-ignored
cargo test --manifest-path src-tauri/Cargo.toml --lib tests::
npm run build
```

New integrated tests cover SSA/VTT burn-in, canonical folder normalization,
malformed timing followed by a valid episode, oversized subtitles, scope escape,
and temporary-file ownership during worker panic/abort. These full-suite tests
have been added but could not run without the decoder dependency.

No release executable or portable archive was rebuilt. The existing clean-first
release hook remains required after validation.

References: [encoding_rs](https://docs.rs/encoding_rs/0.8.35/encoding_rs/),
[tempfile security/lifetime notes](https://docs.rs/tempfile/3.27.0/tempfile/).
