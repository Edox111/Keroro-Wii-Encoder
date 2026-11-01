# Wave 4: Resource management and environment checks

Current dependency resolution and native compilation now pass. See the latest
verification in [UI_LIFECYCLE.md](UI_LIFECYCLE.md); older blocker notes below are
historical, and no final release installer has been rebuilt.

Changes are applied to the existing project. Tauri command orchestration remains
in `src-tauri/src/lib.rs`; independently testable media helpers live in
`src-tauri/crates/encoder-core/src/`. The complete source listings and Cargo
manifests are in [V1_1_CODE.md](V1_1_CODE.md).

## CPU budget

The encoder uses `num_cpus` and a saturating subtraction:

```rust
let threads = progress::thread_budget(num_cpus::get()).to_string();
// thread_budget = logical_cpus.saturating_sub(2).max(1)
command.args(["-threads", &threads, "-filter_threads", &threads]);
// After selecting the input, output encoding also receives -threads <budget>.
```

Subtraction is safe on one- and two-logical-CPU systems. Decoder, filter, and
encoder thread options receive the budget. Only one media job runs at a time.
This is a thread-pool budget, not CPU affinity: it cannot reserve exactly two
cores, cap the total OS thread count, or guarantee an idle CPU percentage.

## Sleep prevention and background operation

`power::WakeLock` owns a Windows power-request handle via `OwnedHandle`.
`PowerCreateRequest` and `PowerSetRequest(PowerRequestSystemRequired)` prevent
idle system sleep while allowing the screen to turn off. No dedicated keeper
thread or polling loop is needed. The handle can move between async workers.

`start_task` acquires the guard after reserving a job, before preflight. Failures
in tool resolution/preparation drop it automatically. The supervisor retains it
while the media worker runs and while process reaping completes, then explicitly
drops it before final status. Panic unwinding also drops it.
`Drop` clears the request; `OwnedHandle` closes the kernel handle afterward.
If acquiring the sleep request fails, the job is rejected with an error.

Batch and preview both use the guard. Minimize to tray still keeps jobs running.
Manual sleep, shutdown, and OS power policies can override requests; this is not
a guarantee that an application can prevent all suspension.

## Timing accuracy, bounded reading, and event throttling

Before each encode, the existing ffprobe command reads the input container's
duration. Invalid/unavailable duration remains `None`. Preview duration is capped
at ten seconds. Duration metadata can be approximate or unavailable, especially
with malformed/VFR media; no exact-duration guarantee is made.

The implementation retains FFmpeg's structured `-progress pipe:1` timing:
`out_time_us`, with parsed `out_time=HH:MM:SS.fraction` as a fallback.
`BufReader::lines()` consumes that structured stdout stream; each line's local
string drops after its iteration. The bounded stderr monitor also parses human
Duration/time/speed lines as fallback telemetry while retaining the last 20 lines
for diagnostics. It handles newline and carriage-return stats, caps each line at
2048 bytes, and skips parsing truncated lines. Structured stdout and ffprobe stay
primary; see UI_LIFECYCLE.md for the live ETA update.

`progress::ProgressGate` permits a timing event only when the current file's
integer percentage or rounded ETA second changes. The 0.25-second FFmpeg reporting
period bounds timing updates to about four per second; identical percent/ETA
values are suppressed. Unknown-duration progress stays indeterminate. The gate
resets for every file. Tray tooltip updates retain their integer-only gate.
Warnings, errors, file transitions, and terminal events always emit so important
state is never hidden by percentage throttling. Duplicate timing snapshots still
update local values but do not clone/emit a frontend event.

100% remains reserved for successful process exit and finalized output. Progress
before that is capped at 99.9%. Existing job IDs, sequence checks, frontend
listener cleanup, and indeterminate-bar behavior remain in effect. Elapsed/FPS
labels update with emitted snapshots, not with a separate high-frequency stream.

## Output canary and permission recovery

`preflight::canary` exclusively creates a uniquely named zero-byte file in the
chosen output folder, closes it without writing, and immediately deletes it.
It never truncates or deletes a pre-existing `.keroro_access_test.tmp` file.
The stronger write/flush/sync probe and minimum free-space checks then run too.

Create/delete/write errors classified as `PermissionDenied` use the stable
`OUTPUT_PERMISSION_DENIED` code. Permission failures during output-directory
creation/canonicalization use that same classification. `main.ts` displays an
actionable notice and restores editable controls when the backend rejects the
job before acceptance. The user can select a different writable directory or
review the app's Windows Controlled Folder Access permissions. No antivirus
settings are changed or bypassed.

The canary checks current file access. It cannot identify every security product,
guarantee antivirus acceptance, or prevent permissions changing later.

## Startup binary checks

The new `check_environment` command resolves both bundled FFmpeg and ffprobe
paths and requires metadata reporting a nonempty regular file. The frontend
awaits this before enabling encoding. Missing/inaccessible binaries leave the
form disabled with an `ENVIRONMENT_UNAVAILABLE` message. Every new job repeats
the checks, covering removal after startup.

Metadata checks presence, not cryptographic integrity or authenticity. An altered
nonempty executable is not detected by this check. No hash/signature validation
or antivirus-compliance claim is implied.

## Verification and current dependency blocker

Completed:

- TypeScript/Vite production build and browser checks for startup validation,
  missing-binary UI locking, permission-error recovery, diagnostics, and progress.
- Thirteen isolated Rust component tests using the actual helper modules,
  including ten native cross-thread wake-lock acquire/drop cycles with no process
  handle-count growth, zero-byte canary cleanup, preservation of existing files,
  metadata rejection, timestamp parsing, and integer gating.
- The gate reduced 10,000 synthetic percentage samples to 100 timing events.
- Existing isolated real-FFmpeg SSA/VTT temp-file and child-reaping checks passed.

Pending: full application compilation and integrated media tests. `encoding_rs`
from Wave 3 and the newly requested `num_cpus` crate are not cached. Dependency
fetching failed because this execution environment cannot resolve crates.io.
The component checks do not compile the complete app or validate num_cpus runtime
detection. They are not evidence of universal zero leaks or guaranteed UI latency.

Run in a network-connected terminal from the project root:

```powershell
cargo fetch --manifest-path src-tauri/crates/encoder-core/Cargo.toml
cargo fetch --manifest-path src-tauri/Cargo.toml

$env:KERORO_TEST_FFMPEG = (Resolve-Path 'src-tauri/binaries/ffmpeg-x86_64-pc-windows-msvc.exe').Path
$env:KERORO_TEST_FFPROBE = (Resolve-Path 'src-tauri/binaries/ffprobe-x86_64-pc-windows-msvc.exe').Path
cargo test --manifest-path src-tauri/crates/encoder-core/Cargo.toml -- --include-ignored --test-threads=1
cargo test --manifest-path src-tauri/Cargo.toml --lib tests::
npm run build
```

The native handle-count test is ignored by default because it measures
process-wide state; run it alone or with `--test-threads=1`, as above. Do not
query already-closed handles to test cleanup.

Both application lockfiles need Cargo refresh after dependency fetching. No
release executable/installer/archive was rebuilt. Once verified, use
`npm run build:windows` so the existing cleanup hook remains enforced.

References:
[PowerCreateRequest](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-powercreaterequest),
[num_cpus](https://docs.rs/num_cpus/1.17.0/num_cpus/).
