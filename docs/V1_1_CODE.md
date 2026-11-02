> Historical implementation snapshot. Current encoding parameters and components are defined by the source tree and [WiiMC profile](WIIMC_PROFILE.md); older code blocks below are not drop-in replacements.

# Keroro Wii Encoder implementation listings

Current source snapshot includes Wave 5, UI/lifecycle, progress/ETA, Cancel, and smart skip. Native Tauri compilation, 51 core unit tests, 9 media integration tests, and 3 Tauri state tests pass. See [UI_LIFECYCLE.md](UI_LIFECYCLE.md) for validation limits. The final installer has not been rebuilt. The HTML entry is root `index.html`; apply modules and configuration together. Source files remain authoritative.

## index.html

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Keroro Wii Encoder</title>
    <link rel="icon" type="image/png" href="/Icon.png" />
  </head>
  <body data-theme="keroro">
    <div id="drag-overlay" hidden aria-hidden="true" role="status" aria-live="polite">
      <h2 id="drag-overlay-text">Drop Media Here</h2>
    </div>
    <div class="container">
      <header>
        <div class="header-heading">
          <h1>Keroro Wii Encoder</h1>
        </div>
        <p class="subtitle">Downscale your anime for Nintendo Wii playback (640x480) | F11 full screen</p>
        
        <div class="platoon-bar" role="group" aria-label="Character theme">
          <button type="button" class="platoon-btn" data-theme="keroro" aria-pressed="true" title="Sergeant Keroro">
            <span class="icon-wrapper" aria-hidden="true"><img id="icon-keroro" width="26" height="26" alt="" /></span>
            <span>Keroro</span>
          </button>
          <button type="button" class="platoon-btn" data-theme="tamama" aria-pressed="false" title="Private Tamama">
            <span class="icon-wrapper" aria-hidden="true"><img id="icon-tamama" width="26" height="26" alt="" /></span>
            <span>Tamama</span>
          </button>
          <button type="button" class="platoon-btn" data-theme="giroro" aria-pressed="false" title="Corporal Giroro">
            <span class="icon-wrapper" aria-hidden="true"><img id="icon-giroro" width="26" height="26" alt="" /></span>
            <span>Giroro</span>
          </button>
          <button type="button" class="platoon-btn" data-theme="kururu" aria-pressed="false" title="Sergeant Major Kururu">
            <span class="icon-wrapper" aria-hidden="true"><img id="icon-kururu" width="26" height="26" alt="" /></span>
            <span>Kururu</span>
          </button>
          <button type="button" class="platoon-btn" data-theme="dororo" aria-pressed="false" title="Assassin Dororo">
            <span class="icon-wrapper" aria-hidden="true"><img id="icon-dororo" width="26" height="26" alt="" /></span>
            <span>Dororo</span>
          </button>
        </div>
      </header>

      <main class="card">
        <div class="input-group">
          <label for="inputFolder">Input Anime Folder:</label>
          <div class="input-row">
            <input type="text" id="inputFolder" placeholder="C:\Users\YourName\Videos\RawAnime" />
            <button id="browseInputBtn" class="secondary-btn">Browse</button>
          </div>
          <p class="field-hint">Supported video containers: MP4, MKV, AVI, MOV, WMV, FLV, WebM. Output: MP4.</p>
        </div>

        <div class="input-group">
          <label for="subtitleFolder">Input Subtitle Folder (Default):</label>
          <div class="input-row">
            <input type="text" id="subtitleFolder" placeholder="Follows the anime folder unless overridden" aria-describedby="subtitleHint" />
            <button type="button" id="browseSubtitleBtn" class="secondary-btn">Browse</button>
          </div>
          <div class="subtitle-options">
            <p id="subtitleHint" class="field-hint">Matches SRT, ASS, SSA, or VTT by filename or episode. Clear to skip subtitles.</p>
            <button type="button" id="resetSubtitleBtn" class="secondary-btn">Use anime folder</button>
          </div>
        </div>

        <div class="input-group">
          <label for="outputFolder">Output Wii Folder:</label>
          <div class="input-row">
            <input type="text" id="outputFolder" placeholder="C:\Users\YourName\Videos\WiiOutput" />
            <button id="browseOutputBtn" class="secondary-btn">Browse</button>
          </div>
        </div>

        <div class="input-group">
          <label for="animeTitle">Anime Title / Output Name Pattern:</label>
          <input type="text" id="animeTitle" placeholder="Sgt Keroro (output: Sgt Keroro - Ep01 [Wii].mp4)" aria-describedby="outputNameHint" />
          <p id="outputNameHint" class="field-hint" aria-live="polite">Original filename_Wii.mp4</p>
        </div>

        <div class="settings-grid">
          <div class="input-group quality-setting">
            <label for="crfSelect">Quality (CRF): <span id="crfValue">23</span></label>
            <select id="crfSelect">
              <option value="20">20 - High Quality (Larger files)</option>
              <option value="23" selected>23 - Standard Wii Balance (Recommended)</option>
              <option value="26">26 - Compressed (Smallest files for SD)</option>
            </select>
          </div>

          <p class="field-hint">Audio: AAC stereo, 128 kbps, 44.1 kHz. Original volume is retained.</p>
        </div>
        <div class="action-row">
          <button id="convertBtn" class="primary-btn" disabled>Start Platoon Downscale</button>
          <button type="button" id="previewBtn" class="secondary-btn" disabled>Preview First 10 Seconds</button>
        </div>

        <section id="encodingProgress" class="live-encoding-progress" hidden aria-label="Current encoding progress and ETA">
          <div id="encodingProgressTrack" class="progress-container" role="progressbar" aria-label="Encoding progress" aria-valuemin="0" aria-valuemax="100" aria-valuenow="0">
            <div id="encodingProgressFill" class="progress-bar"></div>
          </div>
          <span id="encodingProgressText" role="status" aria-live="polite"></span>
        </section>

        <div class="preferences-row">
          <label><input type="checkbox" id="uiSounds" checked /> UI sounds</label>
          <label title="When enabled, closing hides the app and keeps encoding. Use the tray menu to exit."><input type="checkbox" id="minimizeOnClose" checked disabled /> Minimize to System Tray</label>
          <button type="button" id="recoverBatchBtn" class="secondary-btn" hidden>Review interrupted batch</button>
        </div>

        <section class="status-box" aria-label="Encoding status and statistics">
          <div id="status" role="status" aria-live="polite" aria-atomic="true">Ready - Waiting for folder selection</div>
          <div id="fileName" class="file-name">No active video</div>
          <div id="subtitleStatus" class="file-name">External subtitles off</div>
          <div class="progress-label"><span id="fileLabel">Current file</span><span id="filePercent">0%</span></div>
          <div id="fileProgress" class="progress-container" role="progressbar" aria-label="Current file progress" aria-valuemin="0" aria-valuemax="100" aria-valuenow="0">
            <div id="fileProgressBar" class="progress-bar"></div>
          </div>
          <div class="progress-label"><span>Overall batch</span><span id="batchPercent">0%</span></div>
          <div id="batchProgress" class="progress-container" role="progressbar" aria-label="Overall batch progress" aria-valuemin="0" aria-valuemax="100" aria-valuenow="0">
            <div id="progressBar" class="progress-bar"></div>
          </div>
          <dl class="encoding-stats">
            <div><dt>Encoded</dt><dd id="successCount">0</dd></div>
            <div><dt>Skipped</dt><dd id="skippedCount">0</dd></div>
            <div><dt>Failed</dt><dd id="failedCount">0</dd></div>
            <div><dt>Elapsed</dt><dd id="elapsed">0:00</dd></div>
            <div><dt>Speed</dt><dd id="speed">—</dd></div>
            <div><dt>FPS</dt><dd id="fps">—</dd></div>
          </dl>
          <details id="errorDetails" hidden><summary>Warnings and skipped files</summary><ul id="errorList" aria-label="Persistent file warnings and errors"></ul></details>
          <p id="notice" class="notice" role="status" aria-live="polite"></p>
        </section>
      </main>
    </div>

    <dialog id="previewModal" class="preview-modal" aria-labelledby="previewHeading">
      <div class="preview-heading">
        <h2 id="previewHeading">10-Second Encoding Preview</h2>
        <button type="button" id="closePreviewBtn" class="secondary-btn" aria-label="Close preview">Close</button>
      </div>
      <video id="previewVideo" controls playsinline preload="metadata" aria-label="First ten seconds of the encoded video"></video>
      <p class="field-hint">Check picture quality, aspect ratio, audio, and subtitle alignment before starting the full batch.</p>
      <p id="previewPlaybackStatus" role="status" aria-live="polite"></p>
    </dialog>

    <script type="module" src="/src/main.ts"></script>
    <dialog id="errorModal" class="preview-modal" aria-labelledby="errorHeading">
      <h2 id="errorHeading">Task needs attention</h2>
      <pre id="errorMessage" class="dialog-message"></pre>
      <button type="button" id="closeErrorBtn" class="secondary-btn">Close</button>
    </dialog>
    <dialog id="recoveryModal" class="preview-modal" aria-labelledby="recoveryHeading">
      <h2 id="recoveryHeading">Interrupted batch recovery</h2>
      <p id="recoveryMessage" class="dialog-message"></p>
      <p class="field-hint">Resume keeps the original queue and names. Discard removes the checkpoint, preserving completed videos.</p>
      <div class="recovery-actions">
        <button type="button" id="resumeBatchBtn" class="primary-btn">Resume batch</button>
        <button type="button" id="laterRecoveryBtn" class="secondary-btn">Later</button>
        <button type="button" id="discardRecoveryBtn" class="secondary-btn">Discard checkpoint</button>
      </div>
    </dialog>
  </body>
</html>
```

## src/styles.css

```css
:root {
  --bg-image: url("/assets/keroro.png");
  --accent-color: #22c55e;
  --accent-hover: #16a34a;
  --glass-bg: rgba(22, 33, 62, 0.75);
  --glass-border: rgba(74, 222, 128, 0.3);
  --text-main: #f8fafc;
}

*,
*::before,
*::after {
  box-sizing: border-box;
}

body[data-theme="keroro"] {
  --bg-image: url("/assets/keroro.png");
  --accent-color: #22c55e;
  --accent-hover: #16a34a;
  --glass-border: rgba(74, 222, 128, 0.4);
}
body[data-theme="tamama"] {
  --bg-image: url("/assets/tamama.png");
  --accent-color: #3b82f6;
  --accent-hover: #2563eb;
  --glass-border: rgba(59, 130, 246, 0.4);
}
body[data-theme="giroro"] {
  --bg-image: url("/assets/giroro.png");
  --accent-color: #ef4444;
  --accent-hover: #dc2626;
  --glass-border: rgba(248, 113, 113, 0.4);
}
body[data-theme="kururu"] {
  --bg-image: url("/assets/kururu.png");
  --accent-color: #f59e0b;
  --accent-hover: #d97706;
  --glass-border: rgba(245, 158, 11, 0.4);
}
body[data-theme="dororo"] {
  --bg-image: url("/assets/dororo.png");
  --accent-color: #06b6d4;
  --accent-hover: #0891b2;
  --glass-border: rgba(6, 182, 212, 0.4);
}

html,
body {
  width: 100vw;
  height: 100vh;
  max-width: 100vw;
  max-height: 100vh;
  margin: 0;
  overflow: hidden;
  box-sizing: border-box;
}

body {
  background-color: #0a0f1e;
  position: relative;
  isolation: isolate;
  font-family: 'Segoe UI', system-ui, -apple-system, sans-serif;
  background-image: var(--bg-image);
  background-size: cover;
  background-position: center;
  background-attachment: fixed;
  color: var(--text-main);
  margin: 0;
  padding: 10px 16px;
  display: flex;
  justify-content: center;
  align-items: stretch;
  box-sizing: border-box;
}

body::before {
  content: '';
  position: fixed;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  pointer-events: none;
  background: rgba(10, 15, 30, 0.65);
  backdrop-filter: blur(8px);
  z-index: -1;
}

.container {
  width: 100%;
  max-width: 760px;
  height: 100%;
  min-height: 0;
  min-width: 0;
  display: flex;
  flex-direction: column;
}

header {
  text-align: center;
  margin-bottom: 10px;
  flex: 0 0 auto;
}

.header-heading { display: flex; align-items: center; justify-content: center; gap: 12px; }
.header-heading h1 { min-width: 0; }

h1 {
  color: var(--accent-color);
  margin: 0 0 4px;
  font-size: 1.65rem;
  text-shadow: 0 2px 10px rgba(0, 0, 0, 0.5);
  transition: color 0.3s ease;
}

.subtitle {
  color: #cbd5e1;
  margin: 4px 0;
  font-size: 0.8rem;
  text-shadow: 0 1px 4px rgba(0, 0, 0, 0.5);
}

.platoon-bar {
  display: flex;
  justify-content: center;
  gap: 8px;
  margin-top: 8px;
  flex-wrap: wrap;
}

.platoon-btn {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  min-height: 40px;
  padding: 6px 14px 6px 6px;
  background: rgba(15, 23, 42, 0.8);
  border: 1px solid rgba(255, 255, 255, 0.15);
  color: #cbd5e1;
  border-radius: 30px;
  font-size: 0.85rem;
  font-weight: 500;
  cursor: pointer;
  transition: background-color 0.2s ease, border-color 0.2s ease,
    color 0.2s ease, transform 0.2s ease;
  white-space: nowrap;
}

.platoon-btn > .icon-wrapper {
  display: block;
  flex: 0 0 26px;
  width: 26px;
  height: 26px;
  min-width: 26px;
  min-height: 26px;
  max-width: 26px;
  max-height: 26px;
  border-radius: 50%;
  background: rgba(255, 255, 255, 0.15);
  border: 1px solid rgba(255, 255, 255, 0.25);
  overflow: hidden;
}

.platoon-btn > .icon-wrapper > img {
  width: 100%;
  height: 100%;
  max-width: 100%;
  max-height: 100%;
  object-fit: contain;
  object-position: center;
  display: block;
}

.platoon-btn:hover {
  border-color: var(--accent-color);
  color: #fff;
  transform: translateY(-1px);
}

.platoon-btn[aria-pressed="true"] {
  background: var(--accent-color);
  color: #0f172a;
  font-weight: bold;
  border-color: var(--accent-color);
  box-shadow: 0 0 15px var(--accent-color);
}

.card {
  background: var(--glass-bg);
  backdrop-filter: blur(16px);
  -webkit-backdrop-filter: blur(16px);
  display: flex;
  flex-direction: column;
  flex: 1 1 0;
  min-height: 0;
  min-width: 0;
  overflow: hidden;
  padding: 12px 16px;
  border-radius: 16px;
  border: 1px solid var(--glass-border);
  box-shadow: 0 16px 32px rgba(0, 0, 0, 0.5);
  transition: border-color 0.3s ease;
}

.input-group {
  margin-bottom: 7px;
  flex: 0 0 auto;
}

.input-row {
  display: flex;
  gap: 8px;
}

.input-row input {
  flex: 1;
  min-width: 0;
}

.quality-setting {
  margin-bottom: 0;
}

.field-hint {
  color: #cbd5e1;
  font-size: 0.75rem;
  line-height: 1.3;
  margin: 3px 0 0;
  overflow-wrap: anywhere;
}

.action-row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 10px;
}

.preview-modal {
  width: min(900px, calc(100vw - 32px));
  max-height: calc(100dvh - 32px);
  overflow-y: auto;
  padding: 20px;
  color: var(--text-main);
  background: rgba(15, 23, 42, 0.94);
  border: 1px solid var(--glass-border);
  border-radius: 16px;
  box-shadow: 0 24px 80px rgba(0, 0, 0, 0.65);
  backdrop-filter: blur(20px);
}

.preview-modal::backdrop {
  background: rgba(3, 7, 18, 0.75);
  backdrop-filter: blur(8px);
}

.preview-heading {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 16px;
  margin-bottom: 16px;
}

.preview-heading h2 { color: var(--accent-color); font-size: 1.1rem; margin: 0; }
.preview-modal video { display: block; width: 100%; max-height: 65dvh; background: #000; border-radius: 8px; object-fit: contain; }
.dialog-message { white-space: pre-wrap; overflow-wrap: anywhere; max-height: 55dvh; overflow-y: auto; font: inherit; line-height: 1.6; }
.recovery-actions { display: flex; flex-wrap: wrap; gap: 10px; margin-top: 16px; }
.recovery-actions .primary-btn { width: auto; }
#previewPlaybackStatus { color: #ffb0a7; font-size: 0.8rem; }
.secondary-btn:disabled { cursor: not-allowed; opacity: 0.55; }
button:disabled { cursor: not-allowed; opacity: 0.55; }
.platoon-btn:disabled { transform: none; }

label {
  display: block;
  margin-bottom: 3px;
  font-size: 0.8rem;
  color: #e2e8f0;
  font-weight: 500;
}

input[type="text"] {
  width: 100%;
  padding: 7px 10px;
  background: rgba(15, 23, 42, 0.6);
  border: 1px solid rgba(255, 255, 255, 0.15);
  border-radius: 8px;
  color: #fff;
  box-sizing: border-box;
  font-size: 0.9rem;
  transition: border-color 0.2s;
}

input[type="text"]:focus {
  outline: none;
  border-color: var(--accent-color);
  box-shadow: 0 0 8px var(--accent-color);
}

.secondary-btn {
  padding: 7px 12px;
  background: rgba(51, 65, 85, 0.8);
  color: #fff;
  border: 1px solid rgba(255, 255, 255, 0.15);
  border-radius: 8px;
  font-weight: 600;
  cursor: pointer;
  transition: background-color 0.2s ease, border-color 0.2s ease;
}

.secondary-btn:hover {
  background: rgba(71, 85, 105, 0.9);
  border-color: var(--accent-color);
}

.settings-grid {
  background: rgba(15, 23, 42, 0.5);
  padding: 8px 10px;
  border-radius: 10px;
  margin-bottom: 8px;
  flex: 0 0 auto;
  border: 1px solid rgba(255, 255, 255, 0.1);
}

select {
  width: 100%;
  padding: 7px;
  background: rgba(22, 33, 62, 0.8);
  border: 1px solid rgba(255, 255, 255, 0.15);
  border-radius: 8px;
  color: #fff;
  box-sizing: border-box;
  font-size: 0.9rem;
}

select:focus {
  outline: none;
  border-color: var(--accent-color);
}

.subtitle-options { display: flex; align-items: center; flex-wrap: wrap; gap: 4px 8px; margin-top: 3px; }
.subtitle-options .field-hint { flex: 1 1 240px; margin: 0; }
.subtitle-options .secondary-btn { padding: 5px 9px; font-size: 0.75rem; }

.primary-btn {
  width: 100%;
  padding: 10px;
  background: var(--accent-color);
  color: #0f172a;
  border: none;
  border-radius: 8px;
  font-weight: bold;
  font-size: 0.95rem;
  cursor: pointer;
  transition: background-color 0.2s ease, transform 0.2s ease,
    box-shadow 0.2s ease;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
}

.primary-btn:hover {
  background: var(--accent-hover);
  transform: translateY(-1px);
  box-shadow: 0 6px 16px var(--accent-color);
}

.primary-btn:disabled {
  background: #64748b;
  cursor: not-allowed;
  box-shadow: none;
  transform: none;
}

.progress-container {
  margin-top: 6px;
  width: 100%;
  height: 8px;
  background: rgba(15, 23, 42, 0.8);
  border-radius: 4px;
  overflow: hidden;
  border: 1px solid rgba(255, 255, 255, 0.1);
}

.progress-bar {
  width: 0%;
  height: 100%;
  background: var(--accent-color);
  box-shadow: 0 0 10px var(--accent-color);
  transition: width 0.3s ease, background-color 0.3s ease;
}

.status-box {
  margin-top: 12px;
  padding: 12px;
  background: rgba(15, 23, 42, 0.7);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 8px;
  font-family: 'Consolas', monospace;
  font-size: 0.82rem;
  flex: 1 1 0;
  min-height: 56px;
  min-width: 0;
  overflow-x: hidden;
  overflow-y: auto;
  scrollbar-gutter: stable;
  overscroll-behavior: contain;
  overflow-wrap: anywhere;
}

button:focus-visible,
select:focus-visible,
input[type="checkbox"]:focus-visible {
  outline: 2px solid var(--accent-color);
  outline-offset: 3px;
}

.preferences-row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px 12px;
  margin-top: 8px;
  flex: 0 0 auto;
}

.preferences-row label {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  margin: 0;
}

input[type="checkbox"] {
  accent-color: var(--accent-color);
}

.preferences-row .secondary-btn {
  margin-left: auto;
  font-size: 0.8rem;
}

.progress-label {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  margin-top: 12px;
  font: 500 0.8rem 'Segoe UI', system-ui, sans-serif;
}

.file-name {
  color: #cbd5e1;
  font-size: 0.75rem;
  margin-top: 6px;
  overflow-wrap: anywhere;
}

.encoding-stats {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(70px, 1fr));
  gap: 12px;
  margin: 12px 0 0;
}

.encoding-stats dt { color: #cbd5e1; font-size: 0.7rem; }
.encoding-stats dd { margin: 4px 0 0; color: var(--accent-color); font-size: 0.9rem; }

#status[data-tone="error"], .notice { color: #ffb0a7; }
#status[data-tone="success"] { color: var(--accent-color); }
#status[data-tone="warning"], #errorList [data-level="Warning"] { color: #fcd34d; }
.notice:empty { display: none; }
.notice { font-size: 0.7rem; line-height: 1.3; margin: 6px 0 0; flex: 0 0 auto; }
#errorDetails { margin-top: 14px; color: #ffb0a7; }
#errorDetails summary { cursor: pointer; }
#errorList { padding-left: 20px; }
#errorList li + li { margin-top: 8px; }

.progress-container.indeterminate .progress-bar {
  width: 100% !important;
  opacity: 0.65;
  background: repeating-linear-gradient(100deg, var(--accent-color) 0 12px, transparent 12px 24px);
  animation: progress-pulse 1.2s ease-in-out infinite alternate;
}

@keyframes progress-pulse { to { opacity: 0.25; } }

@media (max-width: 480px) {
  .action-row { grid-template-columns: minmax(0, 1fr); }
  body {
    padding: 12px;
  }

  .card {
    padding: 8px;
  }

  .input-row {
    flex-wrap: wrap;
  }

  .input-row input {
    flex: 1 1 100%;
  }
}

@media (prefers-reduced-motion: reduce) {
  *,
  *::before,
  *::after {
    transition: none !important;
    animation: none !important;
  }
}

.card > .action-row { flex: 0 0 auto; }
.card > .input-group, .settings-grid, .action-row, .preferences-row { min-width: 0; }
[hidden] { display: none !important; }

@media (max-height: 740px) {
  body { padding: 8px 12px; }
  header { margin-bottom: 7px; }
  h1 { font-size: 1.4rem; }
  .subtitle { font-size: 0.75rem; }
  .platoon-btn { min-height: 36px; padding-top: 4px; padding-bottom: 4px; }
  .card { padding: 9px 12px; }
  .card > .input-group { margin-bottom: 3px; }
  label { margin-bottom: 2px; }
  input[type="text"] { padding-top: 6px; padding-bottom: 6px; }
  .quality-setting { display: grid; grid-template-columns: auto minmax(0, 1fr); align-items: center; gap: 8px; }
  .quality-setting label { margin: 0; white-space: nowrap; }
  .quality-setting select { min-width: 0; }
  .field-hint { font-size: 0.7rem; }
  .settings-grid { padding: 6px 8px; margin-bottom: 6px; }
  .preferences-row { margin-top: 6px; }
  .preferences-row label { font-size: 0.75rem; }
  .preferences-row .secondary-btn { padding: 5px 8px; }
  .status-box { margin-top: 7px; padding: 8px; }
}

#drag-overlay {
  display: none;
  position: fixed;
  inset: 0;
  z-index: 10000;
  align-items: center;
  justify-content: center;
  padding: 24px;
  background: rgba(0, 0, 0, 0.8);
  border: 4px dashed var(--accent-color);
  color: var(--text-main);
  text-align: center;
  pointer-events: none;
}
#drag-overlay.is-visible { display: flex; }
#drag-overlay h2 { margin: 0; max-width: 100%; overflow-wrap: anywhere; }

.live-encoding-progress {
  display: none;
  flex: 0 0 auto;
  min-width: 0;
  margin-top: 8px;
  gap: 4px;
  font-size: 0.75rem;
  overflow-wrap: anywhere;
}
.live-encoding-progress.is-visible { display: flex; flex-direction: column; }
.live-encoding-progress .progress-container { margin-top: 0; }

.primary-btn.cancel-btn { background: #dc2626; color: #fff; box-shadow: 0 0 12px rgba(239, 68, 68, 0.35); }
.primary-btn.cancel-btn:hover { background: #b91c1c; box-shadow: 0 0 15px rgba(239, 68, 68, 0.5); }
.primary-btn.cancel-btn:disabled { background: #7f1d1d; box-shadow: none; }
```

## src/main.ts

```typescript
import "./styles.css";
import { getPreferences, savePreferences } from "./preferences";
import { cleanAnimeTitle, outputNameExample } from "./naming";
import { initializeThemes } from "./themes";
import { isTerminal, type BatchProgress, type EncodingProgress } from "./batch";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

const audioUrls = import.meta.glob<string>(["./assets/*.mp3", "./assets/sfx/*.mp3", "!./assets/sfx/click.mp3", "!./assets/sfx/complete.mp3"], {
  eager: true, query: "?url", import: "default",
});

function createAudioManager() {
  const players = new Map<string, HTMLAudioElement>();
  function stop(): void {
    players.forEach((player) => {
      try { player.pause(); player.currentTime = 0; } catch { /* Audio cannot block a task. */ }
    });
  }
  function play(asset: string): void {
    const checkbox = document.querySelector<HTMLInputElement>("#uiSounds");
    if (!checkbox?.checked || checkbox.disabled || getPreferences().isMuted) return;
    try {
      const url = audioUrls[asset] ?? asset;
      stop();
      let player = players.get(url);
      if (!player) { player = new Audio(url); player.preload = "auto"; players.set(url, player); }
      player.volume = 0.4;
      // Missing placeholder files or rejected autoplay must not trigger another error sound.
      void player.play().catch(() => {});
    } catch { /* Unsupported audio must not affect encoding. */ }
  }
  return { play, stop };
}
const audioManager = createAudioManager();
function playClick(): void { audioManager.play("./assets/click.mp3"); }
function playSuccess(): void { audioManager.play("./assets/success.mp3"); }
function playError(): void { audioManager.play("./assets/error.mp3"); }

function onUiClick(event: MouseEvent): void {
  if (!(event.target instanceof Element)) return;
  const control = event.target.closest("button, input[type='checkbox'], input[type='radio'], summary");
  if (!control || control.matches(":disabled") || control.id === "uiSounds") return;
  playClick();
}
function onUiChange(event: Event): void {
  if (event.target instanceof Element && event.target.matches("select:not(:disabled), input[type='range']:not(:disabled)")) playClick();
}
document.addEventListener("click", onUiClick, true);
document.addEventListener("change", onUiChange, true);
function disposeAudio(): void {
  document.removeEventListener("click", onUiClick, true);
  document.removeEventListener("change", onUiChange, true);
  audioManager.stop();
}
window.addEventListener("pagehide", disposeAudio, { once: true });
import.meta.hot?.dispose(disposeAudio);

function formatEta(seconds: number | null | undefined): string {
  if (typeof seconds !== "number" || !Number.isFinite(seconds) || seconds < 0 || seconds > Number.MAX_SAFE_INTEGER) return "--";
  const total = Math.ceil(seconds);
  const clock = `${String(Math.floor(total / 60) % 60).padStart(2, "0")}m ${String(total % 60).padStart(2, "0")}s`;
  return total >= 3600 ? `${String(Math.floor(total / 3600)).padStart(2, "0")}h ${clock}` : clock;
}

function requireElement<T extends HTMLElement>(selector: string, type: { new (): T }): T {
  const element = document.querySelector(selector);
  if (!(element instanceof type)) throw new Error(`Missing or invalid UI element: ${selector}`);
  return element;
}

interface DropAnalysis { media_path: string | null; sub_path: string | null; output_path: string | null }
interface NativeDragPayload { paths: string[] }

interface RecoveryInfo {
  id: string; nextEpisode: number | null; totalFiles: number; completedFiles: number;
  canResume: boolean; problem: string | null;
  settings: { inputDir: string; outputDir: string; subtitleDir: string | null; animeTitle: string; crf: number } | null;
}

async function initialize(): Promise<void> {
  // Persist migrated settings before deleting their legacy keys.
  if (savePreferences({})) {
    try {
      for (const key of ["keroro.notifications", "keroro.uiSounds", "keroro.theme"]) {
        localStorage.removeItem(key);
      }
    } catch { /* Storage restrictions must not block startup. */ }
  }
  initializeThemes((theme) => {
    const asset = `./assets/sfx/${theme}.mp3`;
    if (audioUrls[asset]) audioManager.play(asset);
  });
  const overlay = requireElement("#drag-overlay", HTMLDivElement);
  const overlayText = requireElement("#drag-overlay-text", HTMLHeadingElement);
  const input = requireElement("#inputFolder", HTMLInputElement);
  const output = requireElement("#outputFolder", HTMLInputElement);
  const subtitles = requireElement("#subtitleFolder", HTMLInputElement);
  const browseSubtitles = requireElement("#browseSubtitleBtn", HTMLButtonElement);
  const resetSubtitles = requireElement("#resetSubtitleBtn", HTMLButtonElement);
  const animeTitle = requireElement("#animeTitle", HTMLInputElement);
  const outputHint = requireElement("#outputNameHint", HTMLElement);
  const preview = requireElement("#previewBtn", HTMLButtonElement);
  const modal = requireElement("#previewModal", HTMLDialogElement);
  const video = requireElement("#previewVideo", HTMLVideoElement);
  const closePreview = requireElement("#closePreviewBtn", HTMLButtonElement);
  const playbackStatus = requireElement("#previewPlaybackStatus", HTMLElement);
  const subtitleStatus = requireElement("#subtitleStatus", HTMLElement);
  const browseInput = requireElement("#browseInputBtn", HTMLButtonElement);
  const browseOutput = requireElement("#browseOutputBtn", HTMLButtonElement);
  const quality = requireElement("#crfSelect", HTMLSelectElement);
  const qualityValue = requireElement("#crfValue", HTMLElement);
  const sounds = requireElement("#uiSounds", HTMLInputElement);
  const soundObserver = new MutationObserver(() => { if (sounds.disabled || !sounds.checked) audioManager.stop(); });
  soundObserver.observe(sounds, { attributes: true, attributeFilter: ["disabled"] });
  const minimizeOnClose = requireElement("#minimizeOnClose", HTMLInputElement);
  const convert = requireElement("#convertBtn", HTMLButtonElement);
  const liveProgress = requireElement("#encodingProgress", HTMLElement);
  const liveTrack = requireElement("#encodingProgressTrack", HTMLElement);
  const liveFill = requireElement("#encodingProgressFill", HTMLElement);
  const liveText = requireElement("#encodingProgressText", HTMLElement);
  const status = requireElement("#status", HTMLElement);
  const notice = requireElement("#notice", HTMLElement);
  const fileName = requireElement("#fileName", HTMLElement);
  const fileLabel = requireElement("#fileLabel", HTMLElement);
  const filePercent = requireElement("#filePercent", HTMLElement);
  const batchPercent = requireElement("#batchPercent", HTMLElement);
  const fileProgress = requireElement("#fileProgress", HTMLElement);
  const batchProgress = requireElement("#batchProgress", HTMLElement);
  const fileBar = requireElement("#fileProgressBar", HTMLElement);
  const batchBar = requireElement("#progressBar", HTMLElement);
  const successCount = requireElement("#successCount", HTMLElement);
  const skippedCount = requireElement("#skippedCount", HTMLElement);
  const failedCount = requireElement("#failedCount", HTMLElement);
  const elapsed = requireElement("#elapsed", HTMLElement);
  const speed = requireElement("#speed", HTMLElement);
  const fps = requireElement("#fps", HTMLElement);
  const errorDetails = requireElement("#errorDetails", HTMLDetailsElement);
  const errorList = requireElement("#errorList", HTMLUListElement);
  const errorModal = requireElement("#errorModal", HTMLDialogElement);
  const errorMessage = requireElement("#errorMessage", HTMLElement);
  const closeError = requireElement("#closeErrorBtn", HTMLButtonElement);
  const recoveryModal = requireElement("#recoveryModal", HTMLDialogElement);
  const recoveryMessage = requireElement("#recoveryMessage", HTMLElement);
  const resumeBatch = requireElement("#resumeBatchBtn", HTMLButtonElement);
  const laterRecovery = requireElement("#laterRecoveryBtn", HTMLButtonElement);
  const discardRecovery = requireElement("#discardRecoveryBtn", HTMLButtonElement);
  const reviewRecovery = requireElement("#recoverBatchBtn", HTMLButtonElement);
  const controls = [...document.querySelectorAll<HTMLButtonElement>(".platoon-btn"), input, output, subtitles, browseInput, browseOutput, browseSubtitles, resetSubtitles, animeTitle, quality, preview];
  const unlisteners: UnlistenFn[] = [];
  const buffered = new Map<string, BatchProgress>();
  const completedSounds = new Set<string>();
  let busy = false;
  let activeKind: "batch" | "preview" = "batch";
  let closing = false;
  let previewLoading = false;
  let previewResource: { jobId: string; url: string } | null = null;
  let previewCleanup: Promise<void> = Promise.resolve();
  const shownPreviews = new Set<string>();
  let ready = false;
  let syncing = true;
  let starting = false;
  let cancelPending = false;
  let dropScanning = false;
  let dragVersion = 0;
  let disposed = false;
  let currentJob: string | null = null;
  let sequence = -1;
  let diagnosticsKey = "";
  let encodingJob: string | null = null;
  let encodingSequence = -1;
  let recoveryPending: RecoveryInfo | null = null;
  let recoveryAction = false;
  let recoveryLoading = true;
  let recoveryRequest = 0;
  const reportedFailures = new Set<string>();
  function showError(message: string): void {
    playError();
    errorMessage.textContent = message;
    if (!disposed && !errorModal.open) errorModal.showModal();
  }
  closeError.addEventListener("click", () => errorModal.close());
  function showRecovery(): void {
    if (!recoveryPending || disposed || closing) return;
    const info = recoveryPending;
    recoveryMessage.textContent = info.problem ?? (info.nextEpisode === null
      ? "All recorded episodes completed. Discard this leftover checkpoint to begin a new batch."
      : `An interrupted batch queue was detected. Would you like to resume from episode ${info.nextEpisode}? ${info.completedFiles} of ${info.totalFiles} completed outputs will be preserved.`);
    resumeBatch.disabled = !info.canResume || recoveryAction;
    discardRecovery.disabled = recoveryAction;
    if (!recoveryModal.open) recoveryModal.showModal();
  }
  reviewRecovery.addEventListener("click", showRecovery);
  laterRecovery.addEventListener("click", () => { if (!recoveryAction) recoveryModal.close(); });
  recoveryModal.addEventListener("cancel", (event) => { if (recoveryAction) event.preventDefault(); });
  async function refreshRecovery(show = false): Promise<void> {
    const request = ++recoveryRequest;
    recoveryLoading = true; setEncodingState(busy);
    try {
      const pending = await invoke<RecoveryInfo | null>("get_recovery");
      if (request !== recoveryRequest || disposed) return;
      recoveryPending = pending;
      reviewRecovery.hidden = recoveryPending === null;
      if (show) showRecovery();
    } finally { if (request === recoveryRequest) { recoveryLoading = false; setEncodingState(busy); } }
  }

  const preferences = getPreferences();
  minimizeOnClose.checked = preferences.minimizeToTrayOnClose;
  let subtitleFolderOverridden = preferences.subtitleFolderOverridden;
  input.value = preferences.lastInputFolder;
  output.value = preferences.lastOutputFolder;
  subtitles.value = preferences.lastSubtitleFolder;
  animeTitle.value = preferences.animeTitle;
  quality.value = String(preferences.defaultCrf);
  qualityValue.textContent = quality.value;
  outputHint.textContent = outputNameExample(animeTitle.value);
  function saveForm(): void {
    const crf = Number(quality.value);
    savePreferences({
      lastInputFolder: input.value, lastOutputFolder: output.value,
      lastSubtitleFolder: subtitles.value, animeTitle: animeTitle.value,
      subtitleFolderOverridden,
      ...([20, 23, 26].includes(crf) ? { defaultCrf: crf as 20 | 23 | 26 } : {}),
    });
  }
  function syncSubtitles(): void {
    if (!subtitleFolderOverridden) subtitles.value = input.value;
  }
  input.addEventListener("input", () => { syncSubtitles(); saveForm(); });
  subtitles.addEventListener("input", () => { subtitleFolderOverridden = true; saveForm(); });
  [output, animeTitle].forEach((field) => field.addEventListener("input", saveForm));
  quality.addEventListener("change", saveForm);
  resetSubtitles.addEventListener("click", () => {
    subtitleFolderOverridden = false;
    syncSubtitles();
    saveForm();
  });

  function hideDropOverlay(): void {
    dragVersion += 1;
    overlay.hidden = true;
    overlay.classList.remove("is-visible");
    overlay.setAttribute("aria-hidden", "true");
  }
  function canAcceptDrop(): boolean {
    return ready && !busy && !starting && !cancelPending && !dropScanning && !previewLoading && !closing && !disposed
      && !recoveryAction && !recoveryLoading && !browseInput.disabled && !browseOutput.disabled
      && !browseSubtitles.disabled && !document.querySelector("dialog[open]");
  }
  function dragPaths(payload: NativeDragPayload): string[] {
    return Array.isArray(payload?.paths) ? payload.paths.filter((path): path is string => typeof path === "string") : [];
  }
  function showDropOverlay(payload: NativeDragPayload): void {
    const paths = dragPaths(payload);
    if (!canAcceptDrop() || paths.length === 0) { hideDropOverlay(); return; }
    overlayText.textContent = paths.length > 1 ? "Drop Media & Subtitles"
      : /\.(ass|ssa|srt|vtt)$/i.test(paths[0]) ? "Drop Subtitle Here" : "Drop Media Here";
    overlay.hidden = false;
    overlay.classList.add("is-visible");
    overlay.setAttribute("aria-hidden", "false");
    const version = ++dragVersion;
    if (paths.length === 1) {
      void invoke<DropAnalysis>("analyze_dropped_paths", { paths }).then((analysis) => {
        if (version === dragVersion && !overlay.hidden && canAcceptDrop() && analysis?.output_path) {
          overlayText.textContent = "Drop Folder to set Output";
        }
      }).catch(() => { /* Drop execution reports errors; hover never changes the form. */ });
    }
  }
  async function applyDroppedPaths(payload: NativeDragPayload): Promise<void> {
    hideDropOverlay();
    if (!canAcceptDrop()) return;
    const paths = dragPaths(payload);
    if (!paths.length) return;
    dropScanning = true;
    setEncodingState(busy);
    try {
      const analysis = await invoke<DropAnalysis>("analyze_dropped_paths", { paths });
      if (disposed || closing || busy || starting) return;
      if (!analysis || ![analysis.media_path, analysis.sub_path, analysis.output_path].every((path) => path === null || typeof path === "string")) {
        throw new Error("The dropped-path scanner returned an invalid response.");
      }
      if (!analysis.media_path && !analysis.sub_path && !analysis.output_path) throw new Error("No supported media, subtitles, or empty output folder found.");
      if (analysis.output_path) output.value = analysis.output_path;
      if (analysis.media_path) input.value = analysis.media_path;
      const explicitSubtitle = paths.some((path) => /\.(ass|ssa|srt|vtt)$/i.test(path));
      if (analysis.sub_path && (!analysis.media_path || analysis.sub_path !== analysis.media_path || !subtitleFolderOverridden || explicitSubtitle)) {
        subtitles.value = analysis.sub_path;
        subtitleFolderOverridden = !analysis.media_path || analysis.sub_path !== analysis.media_path || explicitSubtitle;
      } else if (analysis.media_path) {
        syncSubtitles();
      }
      saveForm();
      playClick();
      notice.textContent = "Dropped folders applied. Review the paths before starting.";
    } catch (error) {
      if (!disposed && !closing) showError(`Cannot use dropped paths: ${String(error)}`);
    } finally {
      dropScanning = false;
      setEncodingState(busy);
    }
  }

  function showStatus(message: string, tone = "normal"): void {
    if (status.textContent !== message) status.textContent = message;
    status.dataset.tone = tone;
  }
  function resetEncodingProgress(): void {
    liveProgress.hidden = true;
    liveProgress.classList.remove("is-visible");
    liveText.textContent = "";
    setBar(liveTrack, liveFill, 0);
  }
  function renderEncodingProgress(payload: EncodingProgress): void {
    if (disposed || cancelPending || payload.jobId !== currentJob || activeKind !== "batch") return;
    if (!Number.isSafeInteger(payload.sequence) || payload.sequence < sequence) return;
    if (encodingJob !== payload.jobId) { encodingJob = payload.jobId; encodingSequence = -1; }
    if (payload.sequence <= encodingSequence) return;
    encodingSequence = payload.sequence;
    if (!busy || isTerminal(payload.phase)) { resetEncodingProgress(); return; }
    if (payload.phase !== "running" && payload.phase !== "queued") return;
    const percentage = typeof payload.percentage === "number" && Number.isFinite(payload.percentage)
      ? Math.min(100, Math.max(0, payload.percentage)) : null;
    liveProgress.hidden = false;
    liveProgress.classList.add("is-visible");
    setBar(liveTrack, liveFill, percentage);
    const eta = typeof payload.eta === "string" ? payload.eta.slice(0, 64) : "--";
    const file = typeof payload.file === "string" ? payload.file.slice(0, 128) : "video";
    liveText.textContent = payload.phase === "queued" ? "Preparing downscale... (ETA: --)"
      : `Encoding ${file}... ${percentage === null ? "Measuring..." : `${Math.floor(percentage)}%`} (ETA: ${eta})`;
  }

  function setEncodingState(isEncoding: boolean): void {
    busy = isEncoding;
    if (isEncoding && activeKind === "batch" && !cancelPending && !closing && !disposed) {
      liveProgress.hidden = false;
      liveProgress.classList.add("is-visible");
      if (!liveText.textContent) liveText.textContent = "Preparing downscale... (ETA: --)";
    } else { resetEncodingProgress(); }
    if (isEncoding || starting || dropScanning || closing || disposed) hideDropOverlay();
    controls.forEach((control) => { control.disabled = isEncoding || starting || cancelPending || dropScanning || disposed || !ready || previewLoading || closing || recoveryAction || recoveryLoading; });
    document.querySelector("main.card")?.setAttribute("aria-busy", String(isEncoding || starting || cancelPending || dropScanning || previewLoading || closing));
    const cancelling = cancelPending && activeKind === "batch";
    const cancellable = isEncoding && activeKind === "batch";
    convert.classList.toggle("cancel-btn", cancellable || cancelling);
    convert.textContent = cancelling ? "Canceling Downscale..." : cancellable ? "Cancel Downscale" : "Start Platoon Downscale";
    convert.disabled = !ready || starting || cancelPending || dropScanning || disposed || previewLoading || closing
      || recoveryAction || recoveryLoading || (isEncoding && activeKind !== "batch");
    preview.textContent = isEncoding && activeKind === "preview" ? "Generating preview..." : "Preview First 10 Seconds";
    if (recoveryPending && !isEncoding) convert.disabled = true;
    convert.title = recoveryPending ? "Resume or discard the interrupted checkpoint before starting another batch." : "";
    reviewRecovery.disabled = isEncoding || cancelPending || dropScanning || !ready || closing || recoveryAction;
  }
  function setBar(container: HTMLElement, bar: HTMLElement, value: number | null): void {
    container.classList.toggle("indeterminate", value === null);
    if (value === null) { container.removeAttribute("aria-valuenow"); return; }
    const percent = Math.max(0, Math.min(100, Number.isFinite(value) ? value : 0));
    container.setAttribute("aria-valuenow", String(Math.round(percent)));
    bar.style.width = `${percent}%`;
  }
  function render(progress: BatchProgress, effects: boolean): void {
    if (progress.jobId !== currentJob || progress.sequence <= sequence) return;
    sequence = progress.sequence;
    const terminal = isTerminal(progress.phase);
    activeKind = progress.kind ?? "batch";
    setEncodingState(!terminal);
    if (!effects || starting) {
      renderEncodingProgress({
        jobId: progress.jobId, sequence: progress.sequence, phase: progress.phase,
        file: `Ep${String(Math.max(1, progress.currentFile)).padStart(2, "0")}`,
        percentage: progress.filePercent, eta: formatEta(progress.etaSeconds),
      });
    }
    const warnings = progress.warnings ?? [];
    showStatus(cancelPending && !terminal ? "Canceling downscale and cleaning partial output..." : progress.message, progress.failed > 0 ? "error" : warnings.length > 0 ? "warning" : progress.phase === "succeeded" ? "success" : "normal");
    fileName.textContent = progress.fileName ?? "Preparing task...";
    subtitleStatus.textContent = progress.subtitleName ? `Burning subtitles: ${progress.subtitleName}` : "No external subtitle matched / subtitles off";
    fileLabel.textContent = progress.currentFile > 0
      ? activeKind === "preview" ? "Generating first 10 seconds" : `Encoding file ${progress.currentFile} of ${progress.totalFiles}`
      : `Queued ${progress.totalFiles} files`;
    if (!cancelPending) {
      filePercent.textContent = progress.filePercent === null ? (terminal ? "—" : "Measuring…") : `${progress.filePercent.toFixed(1)}%`;
      batchPercent.textContent = `${progress.batchPercent.toFixed(1)}%`;
      setBar(fileProgress, fileBar, progress.filePercent === null && terminal ? 0 : progress.filePercent);
      setBar(batchProgress, batchBar, progress.batchPercent);
    }
    successCount.textContent = String(progress.succeeded);
    skippedCount.textContent = String(progress.skipped ?? 0);
    failedCount.textContent = String(progress.failed);
    const seconds = Math.max(0, Math.floor(progress.elapsedSeconds));
    elapsed.textContent = `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
    speed.textContent = progress.speed ?? "—";
    fps.textContent = progress.fps === null ? "—" : progress.fps.toFixed(1);
    const diagnostics = [
      ...progress.errors.map((entry) => ({ ...entry, level: "Error" })),
      ...warnings.map((entry) => ({ ...entry, level: "Warning" })),
    ];
    errorDetails.hidden = diagnostics.length === 0;
    const newDiagnostics = diagnostics.length > errorList.childElementCount;
    const nextKey = `${progress.jobId}:${JSON.stringify(diagnostics)}`;
    if (diagnosticsKey !== nextKey) {
      diagnosticsKey = nextKey;
      errorList.replaceChildren(...diagnostics.map((error) => {
        const item = document.createElement("li");
        item.dataset.level = error.level;
        item.textContent = `[${error.level}] ${error.fileName}: ${error.reason}${error.level === "Error" ? " Skipped." : ""}`;
        return item;
      }));
      if (newDiagnostics) errorDetails.open = true;
    }
    if (activeKind === "preview" && progress.phase === "succeeded") void openPreview(progress.jobId);
    if (activeKind === "batch" && effects && progress.phase === "succeeded" && !completedSounds.has(progress.jobId)) {
      completedSounds.add(progress.jobId);
      playSuccess();
    }
    if (effects && (progress.phase === "failed" || progress.phase === "completed_with_errors") && !reportedFailures.has(progress.jobId)) {
      reportedFailures.add(progress.jobId);
      showError(`${progress.message}\n\n${progress.errors.map((entry) => `${entry.fileName}: ${entry.reason}`).join("\n")}\n\nDetails are recorded locally in keroro-encoder.log.`);
    }
    if (terminal && activeKind === "batch" && effects) void refreshRecovery().catch((error: unknown) => { playError(); notice.textContent = `Cannot read recovery state: ${String(error)}`; });
  }
  function receive(progress: BatchProgress): void {
    if (syncing || starting) {
      const previous = buffered.get(progress.jobId);
      if (!previous || previous.sequence < progress.sequence) buffered.set(progress.jobId, progress);
      return;
    }
    if (progress.phase === "queued" && currentJob !== progress.jobId) { currentJob = progress.jobId; sequence = -1; }
    render(progress, true);
  }
  function clearPreview(): Promise<void> {
    const resource = previewResource;
    previewResource = null;
    video.pause();
    video.removeAttribute("src");
    video.load();
    if (resource) {
      URL.revokeObjectURL(resource.url);
      previewCleanup = previewCleanup.then(async () => {
        if (isTauri()) await invoke("release_preview", { jobId: resource.jobId });
      }).catch((error: unknown) => { playError(); notice.textContent = `Preview cleanup needs attention: ${String(error)}`; });
    }
    return previewCleanup;
  }
  async function openPreview(jobId: string): Promise<void> {
    if (shownPreviews.has(jobId) || disposed || closing) return;
    shownPreviews.add(jobId);
    previewLoading = true;
    setEncodingState(busy);
    try {
      const bytes = await invoke<ArrayBuffer>("read_preview", { jobId });
      if (disposed || closing || currentJob !== jobId) { await invoke("release_preview", { jobId }); return; }
      const url = URL.createObjectURL(new Blob([bytes], { type: "video/mp4" }));
      previewResource = { jobId, url };
      playbackStatus.textContent = "";
      video.src = url;
      modal.showModal();
      video.load();
    } catch (error) {
      playError();
      notice.textContent = `Cannot open preview: ${String(error)}`;
      await invoke("release_preview", { jobId }).catch(() => {});
    } finally {
      previewLoading = false;
      setEncodingState(busy);
    }
  }
  closePreview.addEventListener("click", () => modal.close());
  modal.addEventListener("close", () => { void clearPreview(); });
  video.addEventListener("error", () => { playError(); playbackStatus.textContent = "The preview could not be played. Close it and generate a new preview."; });
  animeTitle.addEventListener("input", () => {
    outputHint.textContent = outputNameExample(animeTitle.value);
  });
  function dispose(): void {
    disposed = true;
    resetEncodingProgress();
    hideDropOverlay();
    soundObserver.disconnect();
    window.removeEventListener("keydown", onKeyDown);
    unlisteners.splice(0).forEach((unlisten) => { void Promise.resolve(unlisten()).catch(() => {}); });
    void clearPreview();
  }
  window.addEventListener("pagehide", dispose, { once: true });
  import.meta.hot?.dispose(dispose);

  sounds.checked = !getPreferences().isMuted;
  sounds.addEventListener("change", () => {
    savePreferences({ isMuted: !sounds.checked });
    if (sounds.checked) playClick(); else audioManager.stop();
  });

  let fullscreenPending = false;
  function onKeyDown(event: KeyboardEvent): void {
    if (event.key !== "F11" || event.ctrlKey || event.altKey || event.metaKey) return;
    // Prevent browser/WebView default fullscreen, including held-key repeats.
    event.preventDefault();
    if (event.repeat || fullscreenPending || disposed || closing || !isTauri()) return;
    fullscreenPending = true;
    const desktopWindow = getCurrentWindow();
    void desktopWindow.isFullscreen()
      .then((fullscreen) => desktopWindow.setFullscreen(!fullscreen))
      .catch((error: unknown) => { playError(); notice.textContent = `Cannot toggle full screen: ${String(error)}`; })
      .finally(() => { fullscreenPending = false; });
  }
  window.addEventListener("keydown", onKeyDown);
  quality.addEventListener("change", () => { qualityValue.textContent = quality.value; });

  if (!isTauri()) {
    [browseInput, browseOutput, browseSubtitles, convert, preview, minimizeOnClose].forEach((control) => { control.disabled = true; });
    showStatus("UI preview — use the desktop application to encode and access native features.");
    return;
  }

  minimizeOnClose.addEventListener("change", async () => {
    const previous = getPreferences().minimizeToTrayOnClose;
    minimizeOnClose.disabled = true;
    try {
      await invoke("set_close_preferences", { minimizeOnClose: minimizeOnClose.checked });
      savePreferences({ minimizeToTrayOnClose: minimizeOnClose.checked });
    } catch (error) {
      minimizeOnClose.checked = previous;
      showError(`Cannot save close behavior: ${String(error)}`);
    } finally { minimizeOnClose.disabled = disposed || closing; }
  });

  setEncodingState(false);
  try {
    await invoke("set_close_preferences", { minimizeOnClose: minimizeOnClose.checked });
    minimizeOnClose.disabled = false;
    await invoke("check_environment");
    unlisteners.push(await listen<NativeDragPayload>("tauri://drag-enter", ({ payload }) => showDropOverlay(payload)));
    for (const event of ["tauri://drag-drop", "tauri://drop"]) {
      unlisteners.push(await listen<NativeDragPayload>(event, ({ payload }) => { void applyDroppedPaths(payload); }));
    }
    for (const event of ["tauri://drag-leave", "tauri://drag-cancel"]) {
      unlisteners.push(await listen(event, hideDropOverlay));
    }
    unlisteners.push(await listen<BatchProgress>("wii-progress", ({ payload }) => receive(payload)));
    unlisteners.push(await listen<EncodingProgress>("encoding-progress", ({ payload }) => {
      if (!starting && !syncing) renderEncodingProgress(payload);
    }));
    unlisteners.push(await listen<string>("task-cancelling", ({ payload }) => {
      closing = true;
      setEncodingState(true);
      showStatus(payload);
    }));
    const snapshot = await invoke<BatchProgress | null>("get_batch_state");
    if (disposed) { dispose(); return; }
    if (snapshot) { currentJob = snapshot.jobId; render(snapshot, false); }
    else setEncodingState(false);
    syncing = false;
    if (currentJob) {
      const latest = buffered.get(currentJob);
      if (latest) render(latest, false);
    }
    buffered.clear();
  } catch (error) {
    dispose();
    showStatus(`Cannot initialize encoding events: ${String(error)}`, "error");
    playError();
    errorMessage.textContent = `Cannot initialize the desktop backend: ${String(error)}`;
    if (!errorModal.open) errorModal.showModal();
    return;
  }

  async function selectFolder(target: HTMLInputElement, button: HTMLButtonElement): Promise<void> {
    button.disabled = true;
    try {
      const selected = await open({ directory: true, multiple: false, defaultPath: target.value || undefined, title: target === input ? "Select Input Anime Folder" : target === subtitles ? "Select Subtitle Folder" : "Select Output Wii Folder" });
      if (typeof selected === "string") {
        target.value = selected;
        if (target === subtitles) subtitleFolderOverridden = true;
        if (target === input) syncSubtitles();
        saveForm();
      }
    } catch (error) { playError(); notice.textContent = `Cannot select folder: ${String(error)}`; }
    finally { button.disabled = busy || !ready || previewLoading || closing; }
  }
  browseInput.addEventListener("click", () => { void selectFolder(input, browseInput); });
  browseOutput.addEventListener("click", () => { void selectFolder(output, browseOutput); });
  browseSubtitles.addEventListener("click", () => { void selectFolder(subtitles, browseSubtitles); });
  async function startEncoding(kind: "batch" | "preview"): Promise<void> {
    if (!ready || busy || starting || cancelPending || dropScanning || previewLoading || closing || disposed || recoveryAction || recoveryLoading) return;
    if (kind === "batch" && recoveryPending) { showRecovery(); return; }
    saveForm();
    const inputDir = input.value.trim();
    const outputDir = output.value.trim();
    const crf = Number(quality.value);
    const title = cleanAnimeTitle(animeTitle.value);
    if (!inputDir || (kind === "batch" && !outputDir) || ![20, 23, 26].includes(crf)) {
      playError();
      showStatus(kind === "preview" ? "Choose an input folder and quality preset." : "Choose input/output folders and a supported quality preset.", "error");
      return;
    }
    if ((animeTitle.value.trim() && !title) || Array.from(title).length > 100) {
      playError();
      showStatus("Use an anime title with 1–100 filename-safe characters, or leave it blank.", "error");
      return;
    }
    starting = true;
    encodingJob = null; encodingSequence = -1;
    activeKind = kind;
    buffered.clear();
    notice.textContent = "";
    setEncodingState(true);
    let acceptedJob = false;
    try {
      if (modal.open) modal.close();
      await clearPreview();
      showStatus(kind === "preview" ? "Preparing 10-second preview..." : "Preparing batch...");
      setBar(fileProgress, fileBar, null);
      setBar(batchProgress, batchBar, 0);
      filePercent.textContent = "Measuring...";
      batchPercent.textContent = "0%";
      const args = {
        settings: { inputDir, outputDir, crf, subtitleDir: subtitles.value.trim() || null, animeTitle: title },
      };
      const jobId = await invoke<string>(kind === "preview" ? "start_preview" : "start_batch", args);
      acceptedJob = true;
      if (disposed) return;
      currentJob = jobId;
      sequence = -1;
      const latest = buffered.get(jobId) ?? await invoke<BatchProgress | null>("get_batch_state");
      if (latest) render(latest, true);
    } catch (error) {
      setEncodingState(acceptedJob);
      if (String(error).includes("OUTPUT_PERMISSION_DENIED")) {
        notice.textContent = "Windows denied output-folder access. Choose a writable folder or review the app's Controlled Folder Access permissions.";
      }
      setBar(fileProgress, fileBar, 0);
      showStatus(acceptedJob ? `Task started; waiting for progress. Status refresh failed: ${String(error)}` : `Cannot start ${kind}: ${String(error)}`, "error");
      showError(String(error));
      filePercent.textContent = "0%";
    } finally {
      starting = false;
      if (currentJob) { const latest = buffered.get(currentJob); if (latest) render(latest, true); }
      buffered.clear();
      setEncodingState(busy);
    }
  }
  async function cancelDownscale(): Promise<void> {
    if (!busy || activeKind !== "batch" || starting || cancelPending || closing || disposed || !currentJob) return;
    const jobId = currentJob;
    cancelPending = true;
    resetEncodingProgress();
    setBar(fileProgress, fileBar, 0); setBar(batchProgress, batchBar, 0);
    filePercent.textContent = "0%"; batchPercent.textContent = "0%";
    setEncodingState(busy);
    showStatus("Canceling downscale and cleaning partial output...");
    try {
      await invoke("cancel_downscale", { jobId });
      const snapshot = await invoke<BatchProgress | null>("get_batch_state");
      if (snapshot?.jobId === jobId) render(snapshot, true);
      await refreshRecovery();
    } catch (error) {
      showError(`Cannot complete cancellation: ${String(error)}`);
      await refreshRecovery().catch(() => {});
    } finally {
      cancelPending = false;
      setEncodingState(busy);
    }
  }
  convert.addEventListener("click", () => {
    if (busy) void cancelDownscale(); else void startEncoding("batch");
  });
  preview.addEventListener("click", () => { void startEncoding("preview"); });
  discardRecovery.addEventListener("click", async () => {
    if (!recoveryPending || recoveryAction || busy) return;
    recoveryAction = true; discardRecovery.disabled = true; resumeBatch.disabled = true;
    try {
      await invoke("discard_recovery", { recoveryId: recoveryPending.id });
      recoveryPending = null; reviewRecovery.hidden = true; recoveryModal.close();
    } catch (error) { showError(`Cannot discard checkpoint: ${String(error)}`); }
    finally { recoveryAction = false; setEncodingState(busy); if (recoveryModal.open) showRecovery(); }
  });
  resumeBatch.addEventListener("click", async () => {
    if (!recoveryPending?.canResume || recoveryAction || busy || starting || cancelPending || dropScanning || closing) return;
    const saved = recoveryPending.settings;
    if (saved) {
      input.value = saved.inputDir; output.value = saved.outputDir; subtitles.value = saved.subtitleDir ?? "";
      animeTitle.value = saved.animeTitle; quality.value = String(saved.crf); qualityValue.textContent = quality.value;
      subtitleFolderOverridden = saved.subtitleDir !== saved.inputDir;
      outputHint.textContent = outputNameExample(animeTitle.value); saveForm();
    }
    encodingJob = null; encodingSequence = -1;
    recoveryAction = true; starting = true; activeKind = "batch"; buffered.clear(); setEncodingState(true);
    resumeBatch.disabled = true; discardRecovery.disabled = true;
    let accepted = false;
    try {
      const jobId = await invoke<string>("resume_batch", { recoveryId: recoveryPending.id });
      accepted = true; recoveryPending = null; reviewRecovery.hidden = true; recoveryModal.close();
      currentJob = jobId; sequence = -1;
      const latest = buffered.get(jobId) ?? await invoke<BatchProgress | null>("get_batch_state");
      if (latest) render(latest, true);
    } catch (error) {
      setEncodingState(accepted); showError(`Cannot resume batch: ${String(error)}`);
      if (!accepted) await refreshRecovery().catch(() => { playError(); });
    } finally {
      starting = false; recoveryAction = false;
      if (currentJob) { const latest = buffered.get(currentJob); if (latest) render(latest, true); }
      buffered.clear(); setEncodingState(busy); if (recoveryModal.open) showRecovery();
    }
  });
  ready = true;
  setEncodingState(busy);
  try { await refreshRecovery(true); } catch (error) { showError(`Cannot load interrupted batch state: ${String(error)}`); }
}

function start(): void {
  void initialize().catch((error: unknown) => {
    playError();
    const status = document.querySelector<HTMLElement>("#status");
    if (status) { status.textContent = `Cannot initialize application: ${String(error)}`; status.dataset.tone = "error"; }
    const convert = document.querySelector<HTMLButtonElement>("#convertBtn");
    if (convert) convert.disabled = true;
  });
}
if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", start, { once: true });
else start();
```

## src/batch.ts

```typescript
export type Phase = "queued" | "running" | "succeeded" | "completed_with_errors" | "failed" | "cancelled";

export interface BatchProgress {
  kind: "batch" | "preview";
  jobId: string;
  sequence: number;
  phase: Phase;
  currentFile: number;
  totalFiles: number;
  fileName: string | null;
  subtitleName: string | null;
  filePercent: number | null;
  batchPercent: number;
  succeeded: number;
  skipped?: number;
  failed: number;
  elapsedSeconds: number;
  fps: number | null;
  speed: string | null;
  etaSeconds?: number | null;
  message: string;
  errors: Array<{ fileName: string; reason: string }>;
  warnings: Array<{ fileName: string; reason: string }>;
}

export function isTerminal(phase: Phase): boolean {
  return phase === "succeeded" || phase === "completed_with_errors" || phase === "failed" || phase === "cancelled";
}

export interface EncodingProgress {
  file: string;
  percentage: number | null;
  eta: string;
  jobId: string;
  sequence: number;
  phase: Phase;
}
```

## src/preferences.ts

```typescript
export const themes = ["keroro", "tamama", "giroro", "kururu", "dororo"] as const;
export type Theme = typeof themes[number];
export function isTheme(value: unknown): value is Theme {
  return typeof value === "string" && themes.some((theme) => theme === value);
}
export interface Preferences {
  selectedTheme: Theme;
  isMuted: boolean;
  minimizeToTrayOnClose: boolean;
  defaultCrf: 20 | 23 | 26;
  subtitleFolderOverridden: boolean;
  lastInputFolder: string;
  lastOutputFolder: string;
  lastSubtitleFolder: string;
  animeTitle: string;
}
const key = "keroro.preferences.v1";
const defaults: Preferences = {
  selectedTheme: "keroro", isMuted: false, minimizeToTrayOnClose: true, defaultCrf: 23, subtitleFolderOverridden: false,
  lastInputFolder: "", lastOutputFolder: "", lastSubtitleFolder: "", animeTitle: "",
};
function load(): Preferences {
  const result = { ...defaults };
  try {
    const legacyTheme = localStorage.getItem("keroro.theme");
    if (isTheme(legacyTheme)) result.selectedTheme = legacyTheme;
    result.isMuted = localStorage.getItem("keroro.uiSounds") === "false";
    const raw: unknown = JSON.parse(localStorage.getItem(key) ?? "null");
    if (!raw || typeof raw !== "object" || Array.isArray(raw)) return result;
    const value = raw as Record<string, unknown>;
    if (isTheme(value.selectedTheme)) result.selectedTheme = value.selectedTheme;
    if (typeof value.isMuted === "boolean") result.isMuted = value.isMuted;
    if (typeof value.minimizeToTrayOnClose === "boolean") result.minimizeToTrayOnClose = value.minimizeToTrayOnClose;
    if (value.defaultCrf === 20 || value.defaultCrf === 23 || value.defaultCrf === 26) result.defaultCrf = value.defaultCrf;
    for (const field of ["lastInputFolder", "lastOutputFolder", "lastSubtitleFolder", "animeTitle"] as const) {
      if (typeof value[field] === "string" && value[field].length <= 32768) result[field] = value[field];
    }
    // Older versions stored paths only. Preserve a distinct saved subtitle folder.
    result.subtitleFolderOverridden = typeof value.subtitleFolderOverridden === "boolean"
      ? value.subtitleFolderOverridden
      : result.lastSubtitleFolder !== "" && result.lastSubtitleFolder !== result.lastInputFolder;
    if (!result.subtitleFolderOverridden) result.lastSubtitleFolder = result.lastInputFolder;
  } catch { /* Malformed or unavailable storage uses safe defaults. */ }
  return result;
}
let preferences = load();
export function getPreferences(): Readonly<Preferences> { return { ...preferences }; }
export function savePreferences(patch: Partial<Preferences>): boolean {
  preferences = { ...preferences, ...patch };
  try { localStorage.setItem(key, JSON.stringify(preferences)); return true; }
  catch { return false; }
}
```

## src/themes.ts

```typescript
import keroro from "./assets/keroro-icon.webp";
import tamama from "./assets/tamama-icon.webp";
import giroro from "./assets/giroro-icon.webp";
import kururu from "./assets/kururu-icon.webp";
import dororo from "./assets/dororo-icon.webp";
import { getPreferences, savePreferences, isTheme, type Theme } from "./preferences";

const icons = { keroro, tamama, giroro, kururu, dororo } as const;

export function initializeThemes(onSelect: (theme: Theme) => void): void {
  const buttons = document.querySelectorAll<HTMLButtonElement>(".platoon-btn[data-theme]");

  function selectTheme(theme: Theme): void {
    document.body.dataset.theme = theme;
    buttons.forEach((button) => {
      const selected = button.dataset.theme === theme;
      button.setAttribute("aria-pressed", String(selected));
    });
  }

  buttons.forEach((button) => {
    const theme = button.dataset.theme;
    const image = button.querySelector<HTMLImageElement>(".icon-wrapper > img");
    if (!isTheme(theme) || !image) {
      throw new Error("Invalid character theme button markup.");
    }
    image.src = icons[theme];
    button.addEventListener("click", () => {
      selectTheme(theme);
      onSelect(theme);
      savePreferences({ selectedTheme: theme });
    });
  });

  selectTheme(getPreferences().selectedTheme);
}
```

## package.json

```json
{
  "name": "keroro-wii-encoder",
  "private": true,
  "version": "1.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview",
    "tauri": "tauri",
    "clean:release": "powershell -NoProfile -ExecutionPolicy Bypass -File scripts/clean-release.ps1",
    "test:encoder": "cargo test --manifest-path src-tauri/crates/encoder-core/Cargo.toml",
    "build:windows": "tauri build --bundles nsis",
    "clean": "node scripts/clean.mjs",
    "build:release": "tauri build && node extract-release.js"
  },
  "dependencies": {
    "@tauri-apps/api": "^2",
    "@tauri-apps/plugin-dialog": "^2.8.4"
  },
  "devDependencies": {
    "@tauri-apps/cli": "^2",
    "typescript": "~6.0.3",
    "vite": "^8.0.16"
  }
}
```

## src-tauri/src/lib.rs

```rust
pub use keroro_encoder_core::ActiveProcess;
use keroro_encoder_core::{
    BatchPlan, Cancellation, ItemFailure, Phase, Progress, Settings, TaskKind, Tools,
    run_batch_with_session,
};
use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{
    AppHandle, Emitter, Manager, State, WebviewWindow, WindowEvent,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tokio::sync::watch;

#[derive(Clone)]
struct ActiveTask {
    job_id: String,
    cancellation: Cancellation,
    finished: watch::Receiver<bool>,
}
#[derive(Clone)]
struct PreviewFile {
    job_id: String,
    path: PathBuf,
}
struct RuntimeState {
    active: Option<ActiveTask>,
    progress: Option<Progress>,
    preview: Option<PreviewFile>,
    close_prompt_open: bool,
    cancel_job: Option<String>,
    exiting: bool,
    minimize_on_close: bool,
    tray_percent: Option<u8>,
}
impl Default for RuntimeState {
    fn default() -> Self {
        Self {
            active: None,
            progress: None,
            preview: None,
            close_prompt_open: false,
            cancel_job: None,
            exiting: false,
            minimize_on_close: true,
            tray_percent: None,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClosePreferences {
    minimize_on_close: bool,
}

#[derive(Clone, Default)]
struct BatchManager(Arc<Mutex<RuntimeState>>);
struct JobReservation {
    manager: BatchManager,
    job_id: String,
    finished: watch::Sender<bool>,
}
impl BatchManager {
    fn reserve(
        &self,
        job_id: String,
        cancellation: Cancellation,
    ) -> Result<JobReservation, String> {
        let (finished, receiver) = watch::channel(false);
        {
            let mut state = self.0.lock().map_err(|e| e.to_string())?;
            if state.active.is_some()
                || state.cancel_job.is_some()
                || state.close_prompt_open
                || state.exiting
            {
                return Err("An encoding task or close confirmation is already active.".into());
            }
            state.active = Some(ActiveTask {
                job_id: job_id.clone(),
                cancellation,
                finished: receiver,
            });
        }
        Ok(JobReservation {
            manager: self.clone(),
            job_id,
            finished,
        })
    }
}

impl Drop for JobReservation {
    fn drop(&mut self) {
        if let Ok(mut state) = self.manager.0.lock() {
            if state
                .active
                .as_ref()
                .is_some_and(|active| active.job_id == self.job_id)
            {
                state.active = None;
            }
        }
        self.finished.send_replace(true);
    }
}

struct CancelReservation {
    manager: BatchManager,
    job_id: String,
}
impl Drop for CancelReservation {
    fn drop(&mut self) {
        if let Ok(mut state) = self.manager.0.lock() {
            if state.cancel_job.as_deref() == Some(self.job_id.as_str()) {
                state.cancel_job = None;
            }
        }
    }
}

fn show_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if let Err(error) = window
            .show()
            .and_then(|_| window.unminimize())
            .and_then(|_| window.set_focus())
        {
            log::error!("Cannot restore window: {error}");
        }
    }
}

fn publish(manager: &BatchManager, window: &WebviewWindow, progress: Progress) {
    let tooltip = match manager.0.lock() {
        Ok(mut state) => {
            if !state
                .active
                .as_ref()
                .is_some_and(|active| active.job_id == progress.job_id)
            {
                return;
            }
            state.progress = Some(progress.clone());
            if progress.phase.is_terminal() {
                state.active = None;
            }
            let next = if !progress.phase.is_terminal() && progress.kind == TaskKind::Batch {
                Some(if progress.batch_percent.is_finite() {
                    progress.batch_percent.clamp(0.0, 100.0).floor() as u8
                } else {
                    0
                })
            } else {
                None
            };
            // Reuse the core's progress publication; never refresh per stderr line or timer.
            // Lifecycle/warning events with unchanged integer progress do no OS work.
            if next == state.tray_percent {
                None
            } else {
                state.tray_percent = next;
                Some(next.map_or_else(
                    || "Keroro Wii Encoder".into(),
                    |percent| format!("Keroro Wii Encoder - {percent}%"),
                ))
            }
        }
        Err(error) => {
            log::error!("Cannot update task state: {error}");
            return;
        }
    };
    if let Some(tooltip) = tooltip {
        if let Some(tray) = window.app_handle().tray_by_id("keroro-tray") {
            if let Err(error) = tray.set_tooltip(Some(tooltip)) {
                log::warn!("Cannot update tray tooltip: {error}");
            }
        }
    }
    if let Err(error) = window.emit("wii-progress", &progress) {
        log::error!("Cannot deliver progress: {error}");
    }
    if progress.kind == TaskKind::Batch {
        let payload = serde_json::json!({
            "file": format!("Ep{:02}", progress.current_file.max(1)),
            "percentage": progress.file_percent,
            "eta": keroro_encoder_core::progress::format_eta(progress.eta_seconds),
            "jobId": progress.job_id,
            "sequence": progress.sequence,
            "phase": progress.phase,
        });
        if let Err(error) = window.emit("encoding-progress", payload) {
            log::warn!("Cannot deliver ETA progress: {error}");
        }
    }
}

async fn resolve_tools() -> Result<Tools, String> {
    async fn locate(name: &str) -> Result<PathBuf, String> {
        #[cfg(debug_assertions)]
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("binaries")
            .join(format!("{name}-x86_64-pc-windows-msvc.exe"));
        #[cfg(not(debug_assertions))]
        let path = std::env::current_exe()
            .map_err(|e| format!("Cannot locate application: {e}"))?
            .with_file_name(format!("{name}.exe"));
        keroro_encoder_core::environment::validate_binary(&path, name).await?;
        Ok(path)
    }
    Ok(Tools {
        ffmpeg: locate("ffmpeg").await?,
        ffprobe: locate("ffprobe").await?,
    })
}

async fn remove_preview(file: PreviewFile) -> Result<(), String> {
    match tokio::fs::remove_file(&file.path).await {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(format!("Cannot remove temporary preview: {error}")),
    }
    let _ = keroro_encoder_core::artifacts::forget(&file.path).await;
    if let Some(parent) = file.path.parent() {
        match tokio::fs::remove_dir(parent).await {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(format!("Cannot remove preview directory: {error}")),
        }
    }
    Ok(())
}

async fn cleanup_saved_preview(manager: &BatchManager) {
    let file = match manager.0.lock() {
        Ok(mut state) => state.preview.take(),
        Err(error) => {
            log::error!("Cannot access preview cleanup state: {error}");
            None
        }
    };
    if let Some(file) = file {
        if let Err(error) = remove_preview(file).await {
            log::error!("{error}");
        }
    }
}

async fn start_task(
    app: AppHandle,
    window: WebviewWindow,
    manager: BatchManager,
    settings: Settings,
    kind: TaskKind,
    resume_id: Option<String>,
) -> Result<String, String> {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let job_id = format!("{timestamp}-{}", COUNTER.fetch_add(1, Ordering::Relaxed));
    let cancellation = Cancellation::new();
    let reservation = manager.reserve(job_id.clone(), cancellation.clone())?;
    let wake_lock = keroro_encoder_core::power::WakeLock::acquire()?;
    let tools = resolve_tools().await?;
    let sessions = app
        .state::<Arc<keroro_encoder_core::session::SessionStore>>()
        .inner()
        .clone();
    let is_resume = resume_id.is_some();
    let plan = if let Some(resume_id) = resume_id {
        let store = sessions.clone();
        let id = job_id.clone();
        let stop = cancellation.clone();
        let plan = keroro_encoder_core::session::blocking(move || {
            store.resume_checked(&resume_id, &id, &|| stop.is_cancelled())
        })
        .await?;
        keroro_encoder_core::preflight::output(std::path::Path::new(&plan.settings().output_dir))
            .await?;
        plan
    } else if kind == TaskKind::Preview {
        let old = manager.0.lock().map_err(|e| e.to_string())?.preview.take();
        if let Some(old) = old {
            remove_preview(old).await?;
        }
        let directory = app
            .path()
            .app_cache_dir()
            .map_err(|e| e.to_string())?
            .join("previews")
            .join(&job_id);
        match BatchPlan::prepare_preview(job_id.clone(), settings, &directory).await {
            Ok(plan) => plan,
            Err(error) => {
                let _ = tokio::fs::remove_dir(&directory).await;
                return Err(error);
            }
        }
    } else {
        BatchPlan::prepare(job_id.clone(), settings).await?
    };
    if kind == TaskKind::Batch && !is_resume {
        let store = sessions.clone();
        let saved_plan = plan.clone();
        keroro_encoder_core::session::blocking(move || store.begin(&saved_plan)).await?;
    }
    let checkpoint = (kind == TaskKind::Batch).then_some(sessions);
    let preview_path = plan.preview_path();
    let initial = plan.initial_progress();
    let processes = app.state::<Arc<ActiveProcess>>().inner().clone();
    publish(&manager, &window, plan.initial_progress());
    tauri::async_runtime::spawn(async move {
        let _reservation = reservation;
        let wake_lock = wake_lock;
        // The terminal event is held until preview bookkeeping and cleanup finish.
        let worker_manager = manager.clone();
        let worker_window = window.clone();
        let worker_processes = processes.clone();
        // JoinError catches an unwinding panic instead of leaving a stale running snapshot.
        let worker = tokio::spawn(async move {
            run_batch_with_session(
                plan,
                tools,
                cancellation,
                worker_processes,
                checkpoint,
                |progress| {
                    if !progress.phase.is_terminal() {
                        publish(&worker_manager, &worker_window, progress);
                    }
                },
            )
            .await
        });
        let outcome = worker.await;
        processes.wait_until_idle().await;
        drop(wake_lock);
        let mut result = match outcome {
            Ok(result) => result,
            Err(error) => {
                log::error!("Media worker stopped unexpectedly: {error}");
                let mut progress = manager
                    .0
                    .lock()
                    .ok()
                    .and_then(|state| state.progress.clone())
                    .filter(|progress| progress.job_id == initial.job_id)
                    .unwrap_or(initial);
                progress.sequence += 1;
                progress.phase = Phase::Failed;
                progress.file_percent = None;
                progress.eta_seconds = None;
                progress.message = "Encoding worker stopped unexpectedly. Partial files may need removal; retry the task.".into();
                if progress.succeeded + progress.skipped + progress.failed < progress.total_files {
                    progress.failed += 1;
                }
                progress.errors.push(ItemFailure {
                    file_name: progress
                        .file_name
                        .clone()
                        .unwrap_or_else(|| "Background task".into()),
                    reason: "The media worker failed unexpectedly; its child process was stopped."
                        .into(),
                });
                progress
            }
        };
        if let Some(path) = preview_path {
            let file = PreviewFile {
                job_id: result.job_id.clone(),
                path,
            };
            if result.phase == Phase::Succeeded {
                if let Err(error) = keroro_encoder_core::artifacts::record(&file.path).await {
                    log::warn!("Cannot journal preview: {error}");
                }
                if let Ok(mut state) = manager.0.lock() {
                    state.preview = Some(file);
                }
                result.message =
                    "Preview ready — first 10 seconds (or the full video if shorter).".into();
            } else if let Err(error) = remove_preview(file).await {
                log::error!("{error}");
            }
        }
        publish(&manager, &window, result.clone());
    });
    Ok(job_id)
}

#[tauri::command]
async fn cancel_downscale(
    app: AppHandle,
    manager: State<'_, BatchManager>,
    processes: State<'_, Arc<ActiveProcess>>,
    job_id: String,
) -> Result<(), String> {
    let active = {
        let mut state = manager.inner().0.lock().map_err(|e| e.to_string())?;
        let active = match state.active.as_ref() {
            Some(active) if active.job_id == job_id => Some(active.clone()),
            None if state.progress.as_ref().is_some_and(|progress| {
                progress.job_id == job_id && progress.phase.is_terminal()
            }) =>
            {
                None
            }
            _ => return Err("The active job changed; refresh its status before canceling.".into()),
        };
        if active.is_some() {
            if state.cancel_job.is_some() {
                return Err("Cancellation is already in progress.".into());
            }
            state.cancel_job = Some(job_id.clone());
        }
        active
    };
    let Some(active) = active else {
        return Ok(());
    };
    let _cancel_reservation = CancelReservation {
        manager: manager.inner().clone(),
        job_id: job_id.clone(),
    };
    log::info!("User requested cancellation of {}", active.job_id);
    active.cancellation.cancel();
    let mut finished = active.finished;
    loop {
        if *finished.borrow_and_update() {
            break;
        }
        if finished.changed().await.is_err() {
            break;
        }
    }
    processes.inner().wait_until_idle().await;
    let cancelled = manager
        .inner()
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .progress
        .as_ref()
        .is_some_and(|progress| {
            progress.job_id == job_id
                && progress.kind == TaskKind::Batch
                && progress.phase == Phase::Cancelled
        });
    if cancelled {
        let store = app
            .state::<Arc<keroro_encoder_core::session::SessionStore>>()
            .inner()
            .clone();
        keroro_encoder_core::session::blocking(move || store.finish(&job_id))
            .await
            .map_err(|error| {
                format!("Encoding stopped, but the saved queue could not be cleared: {error}")
            })?;
    }
    Ok(())
}

#[tauri::command]
async fn analyze_dropped_paths(paths: Vec<String>) -> Result<serde_json::Value, String> {
    keroro_encoder_core::session::blocking(move || keroro_encoder_core::drop_scan::analyze(paths))
        .await
}

#[tauri::command]
async fn get_recovery(
    app: AppHandle,
    manager: State<'_, BatchManager>,
) -> Result<Option<keroro_encoder_core::session::RecoveryInfo>, String> {
    let unavailable = {
        let state = manager.inner().0.lock().map_err(|e| e.to_string())?;
        state.active.is_some() || state.cancel_job.is_some()
    };
    if unavailable {
        return Ok(None);
    }
    let store = app
        .state::<Arc<keroro_encoder_core::session::SessionStore>>()
        .inner()
        .clone();
    keroro_encoder_core::session::blocking(move || store.info()).await
}
#[tauri::command]
async fn discard_recovery(
    app: AppHandle,
    manager: State<'_, BatchManager>,
    recovery_id: String,
) -> Result<(), String> {
    let reservation = manager.reserve("discard-recovery".into(), Cancellation::new())?;
    let store = app
        .state::<Arc<keroro_encoder_core::session::SessionStore>>()
        .inner()
        .clone();
    let result = keroro_encoder_core::session::blocking(move || store.discard(&recovery_id)).await;
    drop(reservation);
    result
}
#[tauri::command]
async fn resume_batch(
    app: AppHandle,
    window: WebviewWindow,
    manager: State<'_, BatchManager>,
    recovery_id: String,
) -> Result<String, String> {
    let store = app
        .state::<Arc<keroro_encoder_core::session::SessionStore>>()
        .inner()
        .clone();
    let info = keroro_encoder_core::session::blocking(move || store.info())
        .await?
        .ok_or("No interrupted batch exists.")?;
    let settings = info
        .settings
        .ok_or("Recovery checkpoint cannot be resumed; review its error.")?;
    start_task(
        app,
        window,
        manager.inner().clone(),
        settings,
        TaskKind::Batch,
        Some(recovery_id),
    )
    .await
}

#[tauri::command]
async fn check_environment() -> Result<(), String> {
    resolve_tools().await.map(|_| ())
}

#[tauri::command]
async fn start_batch(
    app: AppHandle,
    window: WebviewWindow,
    manager: State<'_, BatchManager>,
    settings: Settings,
) -> Result<String, String> {
    start_task(
        app,
        window,
        manager.inner().clone(),
        settings,
        TaskKind::Batch,
        None,
    )
    .await
}
#[tauri::command]
async fn start_preview(
    app: AppHandle,
    window: WebviewWindow,
    manager: State<'_, BatchManager>,
    settings: Settings,
) -> Result<String, String> {
    start_task(
        app,
        window,
        manager.inner().clone(),
        settings,
        TaskKind::Preview,
        None,
    )
    .await
}
#[tauri::command]
fn get_batch_state(manager: State<'_, BatchManager>) -> Result<Option<Progress>, String> {
    manager
        .inner()
        .0
        .lock()
        .map(|state| state.progress.clone())
        .map_err(|e| e.to_string())
}
#[tauri::command]
async fn read_preview(
    job_id: String,
    manager: State<'_, BatchManager>,
) -> Result<tauri::ipc::Response, String> {
    let file = manager
        .inner()
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .preview
        .clone()
        .filter(|file| file.job_id == job_id)
        .ok_or("This preview is no longer available.")?;
    const LIMIT: u64 = 32 * 1024 * 1024;
    if tokio::fs::metadata(&file.path)
        .await
        .map_err(|e| e.to_string())?
        .len()
        > LIMIT
    {
        return Err("The preview exceeds the 32 MiB playback limit.".into());
    }
    let bytes = tokio::fs::read(&file.path)
        .await
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > LIMIT {
        return Err("The preview exceeds the playback limit.".into());
    }
    Ok(tauri::ipc::Response::new(bytes))
}
#[tauri::command]
async fn release_preview(job_id: String, manager: State<'_, BatchManager>) -> Result<(), String> {
    let file = {
        let mut state = manager.inner().0.lock().map_err(|e| e.to_string())?;
        if state
            .preview
            .as_ref()
            .is_some_and(|file| file.job_id == job_id)
        {
            let file = state.preview.take();
            if state.progress.as_ref().is_some_and(|progress| {
                progress.kind == TaskKind::Preview && progress.job_id == job_id
            }) {
                state.progress = None;
            }
            file
        } else {
            None
        }
    };
    if let Some(file) = file {
        remove_preview(file).await?;
    }
    Ok(())
}
#[tauri::command]
async fn set_close_preferences(
    app: AppHandle,
    manager: State<'_, BatchManager>,
    minimize_on_close: bool,
) -> Result<(), String> {
    let path = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("window_preferences.json");
    let preferences = ClosePreferences { minimize_on_close };
    keroro_encoder_core::session::blocking(move || {
        keroro_encoder_core::atomic_file::json(&path, &preferences)
    })
    .await?;
    manager
        .inner()
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .minimize_on_close = minimize_on_close;
    Ok(())
}

fn request_close(app: &AppHandle, force_exit: bool) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let manager = app.state::<BatchManager>().inner().clone();
    let active = match manager.0.lock() {
        Ok(mut state) => {
            if state.close_prompt_open || state.exiting {
                return;
            }
            if !force_exit && state.minimize_on_close {
                // Close-to-tray never cancels or confirms an active media task.
                drop(state);
                if let Err(error) = window.hide() {
                    log::error!("Cannot hide window: {error}");
                }
                return;
            }
            let active = state.active.clone();
            if active.is_some() {
                state.close_prompt_open = true;
            } else {
                state.exiting = true;
            }
            active
        }
        Err(error) => {
            log::error!("Cannot inspect active task: {error}");
            return;
        }
    };
    let Some(active) = active else {
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            cleanup_saved_preview(&manager).await;
            app.exit(0);
        });
        return;
    };
    show_main(app);
    let app = app.clone();
    let dialog = app
        .dialog()
        .message("Conversion is currently ongoing! Are you sure you want to cancel and exit?")
        .title("Cancel conversion?")
        .parent(&window)
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::OkCancelCustom(
            "Cancel and exit".into(),
            "Keep encoding".into(),
        ));
    dialog.show(move |confirmed| {
        if let Ok(mut state) = manager.0.lock() {
            state.close_prompt_open = false;
            if confirmed {
                state.exiting = true;
            }
        }
        if !confirmed {
            return;
        }
        active.cancellation.cancel();
        let _ = window.emit(
            "task-cancelling",
            "Cancelling conversion and cleaning temporary files before exit...",
        );
        tauri::async_runtime::spawn(async move {
            let mut finished = active.finished;
            loop {
                if *finished.borrow_and_update() {
                    break;
                }
                if finished.changed().await.is_err() {
                    break;
                }
            }
            // The worker has stopped/reaped FFmpeg and completed its cleanup.
            cleanup_saved_preview(&manager).await;
            app.exit(0);
        });
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // The single-instance plugin must run before all other plugins and setup locks.
    let result = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main(app);
        }))
        .manage(BatchManager::default())
        .manage(Arc::new(ActiveProcess::default()))
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            cancel_downscale,
            analyze_dropped_paths,
            get_recovery,
            resume_batch,
            discard_recovery,
            check_environment,
            start_batch,
            start_preview,
            get_batch_state,
            read_preview,
            release_preview,
            set_close_preferences
        ])
        .setup(|app| {
            // First setup operations: all following failures/panics have a local sink.
            let log_directory = app.path().app_log_dir()?;
            keroro_encoder_core::local_log::install(&log_directory)
                .map_err(std::io::Error::other)?;
            let data = app.path().app_data_dir()?;
            let sessions = keroro_encoder_core::session::SessionStore::open(&data)
                .map_err(std::io::Error::other)?;
            app.manage(sessions);
            let preferences_path = data.join("window_preferences.json");
            if preferences_path.exists() {
                keroro_encoder_core::artifacts::no_links(&preferences_path)
                    .map_err(std::io::Error::other)?;
                let metadata = std::fs::metadata(&preferences_path)?;
                if metadata.len() <= 1024 {
                    match std::fs::read(&preferences_path)
                        .map_err(|e| e.to_string())
                        .and_then(|bytes| {
                            serde_json::from_slice::<ClosePreferences>(&bytes)
                                .map_err(|e| e.to_string())
                        }) {
                        Ok(preferences) => {
                            app.state::<BatchManager>()
                                .inner()
                                .0
                                .lock()
                                .map_err(|e| std::io::Error::other(e.to_string()))?
                                .minimize_on_close = preferences.minimize_on_close
                        }
                        Err(error) => log::warn!(
                            "Cannot restore close preference; using close-to-tray: {error}"
                        ),
                    }
                } else {
                    log::warn!("Ignoring oversized window preference file.");
                }
            }
            let cache = app.path().app_cache_dir()?;
            let artifacts = keroro_encoder_core::artifacts::ArtifactStore::open(&data, &cache)
                .map_err(std::io::Error::other)?;
            std::fs::create_dir_all(artifacts.workspace())?;
            keroro_encoder_core::artifacts::no_links(&artifacts.workspace())
                .map_err(std::io::Error::other)?;
            for warning in artifacts.sweep().map_err(std::io::Error::other)? {
                log::warn!("Startup cleanup: {warning}");
            }
            keroro_encoder_core::artifacts::install(artifacts).map_err(std::io::Error::other)?;

            let show =
                MenuItem::with_id(app, "show", "Show Keroro Wii Encoder", true, None::<&str>)?;
            let exit = MenuItem::with_id(app, "exit", "Exit…", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &exit])?;
            let icon = app
                .default_window_icon()
                .cloned()
                .ok_or_else(|| std::io::Error::other("Application icon is missing."))?;
            TrayIconBuilder::with_id("keroro-tray")
                .icon(icon)
                .tooltip("Keroro Wii Encoder — click to restore")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => show_main(app),
                    "exit" => request_close(app, true),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if matches!(
                        event,
                        TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        }
                    ) {
                        show_main(tray.app_handle());
                    }
                })
                .build(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() != "main" {
                    return;
                }
                // Hold native close while hiding or running orderly exit/cancellation.
                api.prevent_close();
                request_close(window.app_handle(), false);
            }
        })
        .run(tauri::generate_context!());
    if let Err(error) = result {
        log::error!("Keroro Wii Encoder could not start: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Barrier;

    #[test]
    fn simultaneous_starts_reserve_exactly_one_job() {
        let manager = BatchManager::default();
        let barrier = Arc::new(Barrier::new(2));
        let results = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..2)
                .map(|number| {
                    let manager = manager.clone();
                    let barrier = barrier.clone();
                    scope.spawn(move || {
                        barrier.wait();
                        let reservation =
                            manager.reserve(format!("job-{number}"), Cancellation::new());
                        barrier.wait(); // Keep the winner reserved until both attempts finish.
                        reservation.is_ok()
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|handle| handle.join().expect("join starter"))
                .collect::<Vec<_>>()
        });
        assert_eq!(results.into_iter().filter(|success| *success).count(), 1);
        assert!(manager.0.lock().expect("state").active.is_none());
        let reservation = manager
            .reserve("retry".into(), Cancellation::new())
            .expect("reusable");
        let finished = manager
            .0
            .lock()
            .expect("state")
            .active
            .as_ref()
            .expect("active")
            .finished
            .clone();
        drop(reservation);
        assert!(*finished.borrow());
    }

    #[test]
    fn cancellation_cleanup_blocks_new_starts_until_its_guard_drops() {
        let manager = BatchManager::default();
        manager.0.lock().expect("state").cancel_job = Some("cancelled-job".into());
        let cleanup = CancelReservation {
            manager: manager.clone(),
            job_id: "cancelled-job".into(),
        };
        assert!(
            manager
                .reserve("blocked".into(), Cancellation::new())
                .is_err()
        );
        drop(cleanup);
        let reservation = manager
            .reserve("next".into(), Cancellation::new())
            .expect("cleanup released");
        drop(reservation);
    }

    #[test]
    fn prompt_and_exit_block_new_jobs_and_decline_preserves_cancellation() {
        let manager = BatchManager::default();
        let cancellation = Cancellation::new();
        let reservation = manager
            .reserve("active".into(), cancellation.clone())
            .expect("reserve");
        manager.0.lock().expect("state").close_prompt_open = true;
        assert!(
            manager
                .reserve("blocked".into(), Cancellation::new())
                .is_err()
        );
        manager.0.lock().expect("state").close_prompt_open = false;
        assert!(!cancellation.is_cancelled());
        drop(reservation);
        manager.0.lock().expect("state").exiting = true;
        assert!(
            manager
                .reserve("during-exit".into(), Cancellation::new())
                .is_err()
        );
    }
}
```

## src-tauri/src/main.rs

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    keroro_wii_encoder_lib::run()
}
```

## src-tauri/Cargo.toml

```toml
[package]
name = "keroro-wii-encoder"
version = "1.1.0"
description = "Batch anime encoder for Nintendo Wii playback"
authors = ["Edox"]
edition = "2024"

[lib]
name = "keroro_wii_encoder_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri-plugin-dialog = "2"
tokio = { version = "1", features = ["full"] }
tauri = { version = "2", features = ["tray-icon"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
log = { version = "0.4.34", features = ["std"] }
tauri-plugin-single-instance = "2"
keroro-encoder-core = { path = "crates/encoder-core" }

# Read the optimization guideline for more details: https://tauri.app/concept/size/#cargo-configuration
[profile.release]
codegen-units = 1
lto = true
opt-level = "s"
panic = "unwind"
strip = true
```

## src-tauri/tauri.conf.json

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "Keroro Wii Encoder",
  "version": "1.1.0",
  "identifier": "com.keroro.wii-encoder",
  "build": {
    "beforeDevCommand": "npm run dev",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "npm run clean:release && npm run build",
    "frontendDist": "../dist"
  },
  "app": {
    "withGlobalTauri": true,
    "windows": [
      {
        "title": "Keroro Wii Encoder",
        "width": 800,
        "height": 900,
        "minWidth": 650,
        "minHeight": 800,
        "dragDropEnabled": true
      }
    ],
    "security": {
      "csp": null
    }
  },
  "bundle": {
    "active": true,
    "targets": [
      "nsis"
    ],
    "externalBin": [
      "binaries/ffmpeg",
      "binaries/ffprobe"
    ],
    "resources": [
      "binaries/FFmpeg-LICENSE.txt"
    ],
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ]
  }
}
```

## src-tauri/capabilities/default.json

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Capability for the main window",
  "windows": [
    "main"
  ],
  "permissions": [
    "core:default",
    "dialog:allow-open",
    "core:window:allow-is-fullscreen",
    "core:window:allow-set-fullscreen"
  ]
}
```

## src-tauri/crates/encoder-core/Cargo.toml

```toml
[package]
name = "keroro-encoder-core"
version = "1.1.0"
edition = "2024"

[dependencies]
serde = { version = "1", features = ["derive"] }
regex = "1"
encoding_rs = "0.8.35"
tempfile = "3.27.0"
getrandom = "0.4.3"
num_cpus = "1.17.0"
log = { version = "0.4.34", features = ["std"] }
serde_json = "1"
sha2 = "0.10.9"
tokio = { version = "1", features = ["fs", "process", "io-util", "rt-multi-thread", "macros", "time", "sync"] }

[target.'cfg(windows)'.dependencies]
windows-sys = { version = "0.61.2", features = ["Win32_Storage_FileSystem", "Win32_Foundation", "Win32_System_Power", "Win32_System_Threading", "Win32_System_JobObjects", "Win32_Security"] }
```

## src-tauri/crates/encoder-core/src/artifacts.rs

```rust
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock},
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct Identity {
    volume: u64,
    index: u64,
}
fn identity(path: &Path) -> Result<Identity, String> {
    no_links(path)?;
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    #[cfg(windows)]
    {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::Storage::FileSystem::{
            BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle,
        };
        let mut info = BY_HANDLE_FILE_INFORMATION::default();
        // SAFETY: live file handle and correctly sized writable output.
        if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) } == 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        Ok(Identity {
            volume: u64::from(info.dwVolumeSerialNumber),
            index: (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow),
        })
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let info = file.metadata().map_err(|e| e.to_string())?;
        Ok(Identity {
            volume: info.dev(),
            index: info.ino(),
        })
    }
    #[cfg(not(any(windows, unix)))]
    {
        let _ = file;
        Err("Stable artifact identity is unsupported on this platform.".into())
    }
}
pub fn no_links(path: &Path) -> Result<(), String> {
    let components = path.ancestors().collect::<Vec<_>>();
    for component in components.into_iter().rev() {
        let metadata = fs::symlink_metadata(component).map_err(|e| e.to_string())?;
        if metadata.file_type().is_symlink() {
            return Err("Cleanup refuses symbolic links.".into());
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if metadata.file_attributes() & 0x400 != 0 {
                return Err("Cleanup refuses reparse points/junctions.".into());
            }
        }
    }
    Ok(())
}
#[derive(Clone, Serialize, Deserialize)]
struct Artifact {
    path: PathBuf,
    identity: Identity,
}
pub struct ArtifactStore {
    ledger: PathBuf,
    cache: PathBuf,
    entries: Mutex<Vec<Artifact>>,
}
static STORE: OnceLock<Arc<ArtifactStore>> = OnceLock::new();

impl ArtifactStore {
    pub fn open(data: &Path, cache: &Path) -> Result<Arc<Self>, String> {
        crate::offline::local_path(data)?;
        crate::offline::local_path(cache)?;
        fs::create_dir_all(cache).map_err(|e| e.to_string())?;
        let cache = fs::canonicalize(cache).map_err(|e| e.to_string())?;
        no_links(&cache)?;
        let ledger = data.join("artifacts.json");
        let entries = match fs::metadata(&ledger) {
            Ok(meta) if meta.len() <= 8 * 1024 * 1024 => {
                no_links(&ledger)?;
                serde_json::from_slice(&fs::read(&ledger).map_err(|e| e.to_string())?)
                    .map_err(|e| format!("Artifact ledger needs manual review: {e}"))?
            }
            Ok(_) => return Err("Artifact ledger exceeds its 8 MiB limit.".into()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(error) => return Err(error.to_string()),
        };
        Ok(Arc::new(Self {
            ledger,
            cache,
            entries: Mutex::new(entries),
        }))
    }
    pub fn workspace(&self) -> PathBuf {
        self.cache.join("subtitle-work")
    }
    fn allowed(&self, path: &Path) -> bool {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        let private_output = name.starts_with('.')
            && (name.ends_with(".part.mp4")
                || name.starts_with(".keroro_access_test-")
                || name.starts_with(".keroro-write-check-"));
        let subtitle = path.starts_with(self.workspace()) && name.starts_with("subtitle-");
        let preview = path.starts_with(self.cache.join("previews")) && name == "preview_temp.mp4";
        private_output || name.ends_with(".mp4.part") || subtitle || preview
    }
    pub fn record(&self, path: &Path) -> Result<(), String> {
        let parent = fs::canonicalize(path.parent().ok_or("Artifact has no parent.")?)
            .map_err(|e| e.to_string())?;
        let path = parent.join(path.file_name().ok_or("Artifact has no filename.")?);
        if !self.allowed(&path) {
            return Err("Refusing to journal a non-private artifact.".into());
        }
        let artifact = Artifact {
            identity: identity(&path)?,
            path,
        };
        let mut entries = self.entries.lock().map_err(|e| e.to_string())?;
        entries.retain(|entry| entry.path != artifact.path);
        entries.push(artifact);
        crate::atomic_file::json(&self.ledger, &*entries)
    }
    pub fn forget(&self, path: &Path) -> Result<(), String> {
        let mut entries = self.entries.lock().map_err(|e| e.to_string())?;
        entries.retain(|entry| entry.path != path);
        crate::atomic_file::json(&self.ledger, &*entries)
    }
    pub fn sweep(&self) -> Result<Vec<String>, String> {
        let mut entries = self.entries.lock().map_err(|e| e.to_string())?;
        let mut retained = Vec::new();
        let mut warnings = Vec::new();
        for entry in entries.iter() {
            match fs::symlink_metadata(&entry.path) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => {
                    warnings.push(format!(
                        "Cannot inspect artifact {}: {error}",
                        entry.path.display()
                    ));
                    retained.push(entry.clone());
                    continue;
                }
                Ok(_) => {}
            }
            if !self.allowed(&entry.path) || identity(&entry.path).as_ref() != Ok(&entry.identity) {
                warnings.push(format!(
                    "Preserved unrecognized/replaced artifact {}",
                    entry.path.display()
                ));
                retained.push(entry.clone());
                continue;
            }
            #[cfg(windows)]
            {
                use std::os::windows::fs::OpenOptionsExt;
                // Refuse cleanup while an old encoder or another process holds it open.
                match fs::OpenOptions::new()
                    .read(true)
                    .write(true)
                    .share_mode(0)
                    .open(&entry.path)
                {
                    Ok(file) => drop(file),
                    Err(error) => {
                        warnings.push(format!(
                            "Owned artifact is still locked: {} ({error})",
                            entry.path.display()
                        ));
                        retained.push(entry.clone());
                        continue;
                    }
                }
            }
            if let Err(error) = fs::remove_file(&entry.path) {
                warnings.push(format!(
                    "Cannot remove owned artifact {}: {error}",
                    entry.path.display()
                ));
                retained.push(entry.clone());
                continue;
            }
            log::info!("Removed owned orphan {}", entry.path.display());
            if entry.path.starts_with(self.workspace()) {
                if let Some(parent) = entry.path.parent() {
                    let _ = fs::remove_dir(parent);
                } // Empty only; never recursive.
            }
        }
        *entries = retained;
        crate::atomic_file::json(&self.ledger, &*entries)?;
        Ok(warnings)
    }
}
pub fn install(store: Arc<ArtifactStore>) -> Result<(), String> {
    STORE
        .set(store)
        .map_err(|_| "Artifact store already configured.".into())
}
pub fn workspace() -> Option<PathBuf> {
    STORE.get().map(|store| store.workspace())
}
pub fn forget_sync(path: &Path) -> Result<(), String> {
    if let Some(store) = STORE.get() {
        store.forget(path)?;
    }
    Ok(())
}
pub fn record_sync(path: &Path) -> Result<(), String> {
    if let Some(store) = STORE.get() {
        store.record(path)?;
    }
    Ok(())
}
pub async fn record(path: &Path) -> Result<(), String> {
    let Some(store) = STORE.get().cloned() else {
        return Ok(());
    };
    let path = path.to_path_buf();
    tokio::task::spawn_blocking(move || store.record(&path))
        .await
        .map_err(|e| e.to_string())?
}
pub async fn forget(path: &Path) -> Result<(), String> {
    let Some(store) = STORE.get().cloned() else {
        return Ok(());
    };
    let path = path.to_path_buf();
    tokio::task::spawn_blocking(move || store.forget(&path))
        .await
        .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sweeper_removes_only_recorded_private_files_with_matching_identity() {
        let root = tempfile::tempdir().expect("fixture");
        let cache = root.path().join("cache");
        let store = ArtifactStore::open(root.path(), &cache).expect("store");
        let own = root.path().join(".episode.test.part.mp4");
        let user = root.path().join("preview_temp.mp4");
        fs::write(&own, "partial").expect("own");
        fs::write(&user, "user media").expect("user");
        store.record(&own).expect("journal");
        assert!(store.record(&user).is_err());
        assert!(store.sweep().expect("sweep").is_empty());
        assert!(!own.exists());
        assert!(user.exists());
    }
}
```

## src-tauri/crates/encoder-core/src/atomic_file.rs

```rust
#[cfg(any(not(windows), test))]
use std::fs;
use std::{io::Write, path::Path};

/// Same-directory rename. Final media never becomes visible through a partial copy.
pub fn rename(source: &Path, destination: &Path, replace: bool) -> Result<(), String> {
    if source.parent() != destination.parent() {
        return Err("Atomic commit requires the same parent directory.".into());
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::{
            MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
        };
        let wide = |path: &Path| -> Result<Vec<u16>, String> {
            let mut value = path.as_os_str().encode_wide().collect::<Vec<_>>();
            if value.contains(&0) {
                return Err("NUL in path.".into());
            }
            value.push(0);
            Ok(value)
        };
        // Canonical parent paths preserve long-path and redirected-temp semantics
        // for native Win32 calls, while retaining the destination basename.
        let parent = std::fs::canonicalize(source.parent().ok_or("Missing source parent.")?)
            .map_err(|e| e.to_string())?;
        let source = wide(&parent.join(source.file_name().ok_or("Missing source basename.")?))?;
        let destination =
            wide(&parent.join(destination.file_name().ok_or("Missing target basename.")?))?;
        // SAFETY: live terminated UTF-16 paths. No cross-volume copy flag; media never replaces an existing file.
        let ok = unsafe {
            MoveFileExW(
                source.as_ptr(),
                destination.as_ptr(),
                MOVEFILE_WRITE_THROUGH
                    | if replace {
                        MOVEFILE_REPLACE_EXISTING
                    } else {
                        0
                    },
            )
        };
        if ok == 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
    }
    #[cfg(not(windows))]
    {
        if replace {
            fs::rename(source, destination).map_err(|e| e.to_string())?;
        } else {
            fs::hard_link(source, destination).map_err(|e| e.to_string())?;
            fs::remove_file(source).map_err(|e| e.to_string())?;
        }
        if let Some(parent) = destination.parent() {
            fs::File::open(parent)
                .and_then(|file| file.sync_all())
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

pub fn json(path: &Path, value: &impl serde::Serialize) -> Result<(), String> {
    let parent = path.parent().ok_or("Missing checkpoint parent.")?;
    let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    struct Limited<'a> {
        file: &'a mut std::fs::File,
        remaining: usize,
    }
    impl Write for Limited<'_> {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if bytes.len() > self.remaining {
                return Err(std::io::Error::other(
                    "Local state exceeds the 8 MiB limit.",
                ));
            }
            let written = self.file.write(bytes)?;
            self.remaining -= written;
            Ok(written)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            self.file.flush()
        }
    }
    serde_json::to_writer(
        Limited {
            file: temp.as_file_mut(),
            remaining: 8 * 1024 * 1024,
        },
        value,
    )
    .map_err(|e| e.to_string())?;
    temp.flush()
        .and_then(|_| temp.as_file().sync_all())
        .map_err(|e| e.to_string())?;
    rename(temp.path(), path, true)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn oversized_state_preserves_the_previous_checkpoint() {
        let root = tempfile::tempdir().expect("fixture");
        let path = root.path().join("state.json");
        json(&path, &vec![1]).expect("initial checkpoint");
        assert!(json(&path, &"x".repeat(8 * 1024 * 1024)).is_err());
        assert_eq!(fs::read_to_string(path).expect("read"), "[1]");
        assert_eq!(fs::read_dir(root.path()).expect("list").count(), 1);
    }
    #[test]
    fn commit_never_overwrites_existing_media_and_json_is_replaceable() {
        let root = tempfile::tempdir().expect("fixture");
        let source = root.path().join("temp.mp4");
        let target = root.path().join("final.mp4");
        fs::write(&source, b"complete").expect("temp");
        fs::write(&target, b"original").expect("existing");
        assert!(rename(&source, &target, false).is_err());
        assert_eq!(fs::read(&target).expect("read"), b"original");
        fs::remove_file(&target).expect("remove fixture");
        rename(&source, &target, false).expect("publish");
        assert_eq!(fs::read(target).expect("read"), b"complete");
        let checkpoint = root.path().join("state.json");
        json(&checkpoint, &vec![1]).expect("first");
        json(&checkpoint, &vec![1, 2]).expect("replace");
        assert_eq!(fs::read_to_string(checkpoint).expect("read"), "[1,2]");
    }
}
```

## src-tauri/crates/encoder-core/src/cancel.rs

```rust
use tokio::sync::watch;

#[derive(Clone, Debug)]
pub struct Cancellation(watch::Sender<bool>);

impl Default for Cancellation {
    fn default() -> Self {
        Self::new()
    }
}

impl Cancellation {
    pub fn new() -> Self {
        Self(watch::channel(false).0)
    }
    pub fn cancel(&self) {
        self.0.send_replace(true);
    }
    pub fn is_cancelled(&self) -> bool {
        *self.0.borrow()
    }
    pub async fn cancelled(&self) {
        let mut receiver = self.0.subscribe();
        loop {
            if *receiver.borrow_and_update() {
                return;
            }
            if receiver.changed().await.is_err() {
                return;
            }
        }
    }
}
```

## src-tauri/crates/encoder-core/src/diagnostics.rs

```rust
use std::collections::VecDeque;
use tokio::io::{AsyncRead, AsyncReadExt};

/// Drain stderr completely without allocating an unbounded String for one hostile line.
pub async fn monitor(
    mut reader: impl AsyncRead + Unpin,
    mut on_line: impl FnMut(&str),
) -> Result<String, String> {
    let mut tail = VecDeque::new();
    let mut line = Vec::new();
    let mut chunk = [0u8; 4096];
    let mut truncated = false;
    let mut previous_cr = false;
    loop {
        let count = reader.read(&mut chunk).await.map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        for byte in &chunk[..count] {
            if *byte == b'\n' || *byte == b'\r' {
                if *byte == b'\n' && previous_cr {
                    previous_cr = false;
                    continue;
                }
                previous_cr = *byte == b'\r';
                push(&mut tail, &line, truncated, &mut on_line);
                line.clear();
                truncated = false;
            } else {
                previous_cr = false;
                if line.len() < 2048 {
                    line.push(*byte);
                } else {
                    truncated = true;
                }
            }
        }
    }
    if !line.is_empty() || truncated {
        push(&mut tail, &line, truncated, &mut on_line);
    }
    Ok(tail.into_iter().collect::<Vec<_>>().join("\n"))
}
fn push(
    tail: &mut VecDeque<String>,
    bytes: &[u8],
    truncated: bool,
    on_line: &mut impl FnMut(&str),
) {
    if tail.len() == 20 {
        tail.pop_front();
    }
    let mut text = String::from_utf8_lossy(bytes).into_owned();
    if !truncated {
        on_line(&text);
    }
    if truncated {
        text.push_str(" [truncated]");
    }
    tail.push_back(text);
}

#[cfg(test)]
async fn tail(reader: impl AsyncRead + Unpin) -> Result<String, String> {
    monitor(reader, |_| {}).await
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn carriage_return_stats_are_parsed_without_crlf_duplicate_lines() {
        let mut parser = crate::progress::StderrProgress::default();
        let input = b"[info] Duration: 00:03:00.00, bitrate: 123\r\ntime=00:00:30.00 speed=2x\rtime=00:01:00.00 speed=1x\r\n";
        let tail = super::monitor(input.as_slice(), |line| {
            parser.update(line);
        })
        .await
        .expect("monitor");
        assert_eq!(tail.lines().count(), 3);
        assert_eq!(parser.duration, Some(180.0));
        assert_eq!(parser.position, Some(60.0));
        assert_eq!(parser.speed, Some(1.0));
    }
    #[tokio::test]
    #[ignore = "Requires KERORO_TEST_FFMPEG pointing to the bundled FFmpeg executable"]
    async fn real_ffmpeg_stderr_provides_duration_time_and_speed() {
        use std::process::Stdio;
        use tokio::process::Command;
        let binary = std::env::var_os("KERORO_TEST_FFMPEG").expect("FFmpeg fixture binary");
        let root = tempfile::tempdir().expect("fixture");
        let input = root.path().join("\u{30b1}\u{30ed}\u{30ed} [test]' .mp4");
        let created = Command::new(&binary)
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-f",
                "lavfi",
                "-i",
                "color=c=black:s=64x48:r=10:d=2",
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
            ])
            .arg(&input)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .output()
            .await
            .expect("make video");
        assert!(
            created.status.success(),
            "{}",
            String::from_utf8_lossy(&created.stderr)
        );
        let mut child = Command::new(&binary)
            .args([
                "-hide_banner",
                "-loglevel",
                "repeat+level+info",
                "-stats",
                "-stats_period",
                "0.1",
                "-i",
            ])
            .arg(&input)
            .args(["-f", "null", "-"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .expect("run");
        let mut parser = crate::progress::StderrProgress::default();
        let tail = super::monitor(child.stderr.take().expect("stderr"), |line| {
            parser.update(line);
        })
        .await
        .expect("drain");
        assert!(child.wait().await.expect("wait").success(), "{tail}");
        assert_eq!(parser.duration, Some(2.0));
        assert!(parser.position.is_some_and(|position| position >= 1.9));
        assert!(parser.speed.is_some_and(|speed| speed > 0.0));
        assert!(
            crate::progress::eta_seconds(parser.position, parser.duration, parser.speed).is_some()
        );
    }
    #[tokio::test]
    async fn last_twenty_normal_lines_are_preserved_exactly() {
        let input = (1..=25)
            .map(|number| format!("diagnostic {number}\n"))
            .collect::<String>();
        let actual = super::tail(input.as_bytes()).await.expect("tail");
        let expected = (6..=25)
            .map(|number| format!("diagnostic {number}"))
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(actual, expected);
    }
    #[tokio::test]
    async fn diagnostics_cap_long_lines_and_preserve_final_errors() {
        let mut bytes = vec![b'x'; 200_000];
        bytes.extend_from_slice(b"\nSubtitle parser failed\n");
        let text = super::tail(bytes.as_slice()).await.expect("drain");
        assert!(text.len() < 2200);
        assert!(text.contains("[truncated]") && text.contains("Subtitle parser failed"));
    }
}
```

## src-tauri/crates/encoder-core/src/drop_scan.rs

```rust
use std::{collections::HashSet, fs, path::PathBuf};

const MAX_PATHS: usize = 64;
const MAX_ENTRIES: usize = 512;

fn clean_windows_prefix(value: &str) -> String {
    if let Some(rest) = value.strip_prefix(r"\\?\") {
        let bytes = rest.as_bytes();
        if bytes.first().is_some_and(u8::is_ascii_alphabetic)
            && bytes.get(1) == Some(&b':')
            && matches!(bytes.get(2), Some(b'\\' | b'/'))
        {
            return rest.to_owned();
        }
    }
    // Keep UNC/device namespaces intact so the existing offline guard rejects them.
    value.to_owned()
}

pub fn analyze(paths: Vec<String>) -> Result<serde_json::Value, String> {
    if paths.is_empty() || paths.len() > MAX_PATHS {
        return Err("Drop between 1 and 64 local files or folders.".into());
    }
    if paths
        .iter()
        .any(|path| path.is_empty() || path.len() > 32_768)
    {
        return Err("A dropped path is empty or too long.".into());
    }
    let mut seen = HashSet::new();
    let mut media: Option<PathBuf> = None;
    let mut output: Option<PathBuf> = None;
    let mut subtitles = Vec::new();
    for value in paths {
        let path = PathBuf::from(clean_windows_prefix(&value));
        crate::offline::local_path(&path)?;
        crate::artifacts::no_links(&path)?;
        let path = fs::canonicalize(&path)
            .map_err(|e| format!("Cannot inspect dropped path {}: {e}", path.display()))?;
        crate::offline::local_path(&path)?;
        if !seen.insert(path.clone()) {
            continue;
        }
        let metadata = fs::metadata(&path).map_err(|e| e.to_string())?;
        let (folder, has_video, has_subtitle) = if metadata.is_dir() {
            let mut has_video = false;
            let mut has_subtitle = false;
            let mut entries_seen = 0usize;
            let entries = fs::read_dir(&path)
                .map_err(|e| format!("Cannot read dropped folder {}: {e}", path.display()))?;
            for entry in entries.take(MAX_ENTRIES) {
                let entry = entry.map_err(|e| e.to_string())?;
                entries_seen += 1;
                // Shallow metadata only: do not follow links, recurse, or read media contents.
                if !entry.file_type().map_err(|e| e.to_string())?.is_file() {
                    continue;
                }
                let file = entry.path();
                has_video |= crate::is_video_path(&file);
                has_subtitle |= crate::subtitles::extension(&file).is_some();
                if has_video && has_subtitle {
                    break;
                }
            }
            if entries_seen == 0 && output.is_none() {
                output = Some(path.clone());
            }
            (path, has_video, has_subtitle)
        } else if metadata.is_file() {
            let folder = path
                .parent()
                .ok_or("Dropped file has no parent folder.")?
                .to_path_buf();
            (
                folder,
                crate::is_video_path(&path),
                crate::subtitles::extension(&path).is_some(),
            )
        } else {
            continue;
        };
        if has_video && media.is_none() {
            media = Some(folder.clone());
        }
        if has_subtitle {
            subtitles.push((folder, has_video));
        }
    }
    let subtitle = subtitles
        .iter()
        .find(|(_, has_video)| !has_video)
        .or_else(|| subtitles.first())
        .map(|(folder, _)| folder);
    if media.is_none() && subtitle.is_none() && output.is_none() {
        return Err(format!(
            "No supported media, subtitles, or completely empty output folder found in the first {MAX_ENTRIES} entries per folder. Nested folders are not scanned."
        ));
    }
    Ok(serde_json::json!({
        "media_path": media.as_ref().map(|path| clean_windows_prefix(&path.to_string_lossy())),
        "sub_path": subtitle.map(|path| clean_windows_prefix(&path.to_string_lossy())),
        "output_path": output.as_ref().map(|path| clean_windows_prefix(&path.to_string_lossy())),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn extended_drive_prefixes_are_cleaned_without_reinterpreting_network_namespaces() {
        assert_eq!(clean_windows_prefix(r"\\?\D:\Anime"), r"D:\Anime");
        assert_eq!(clean_windows_prefix(r"D:\Anime"), r"D:\Anime");
        assert_eq!(
            clean_windows_prefix(r"\\?\UNC\server\share"),
            r"\\?\UNC\server\share"
        );
        assert_eq!(
            clean_windows_prefix(r"\\?\GLOBALROOT\Device"),
            r"\\?\GLOBALROOT\Device"
        );
    }
    #[test]
    fn only_truly_empty_directories_are_output_candidates() {
        let root = tempfile::tempdir().expect("fixture");
        let canonical = fs::canonicalize(root.path()).expect("canonical");
        let result = analyze(vec![canonical.to_string_lossy().into()]).expect("empty folder");
        assert!(result["media_path"].is_null() && result["sub_path"].is_null());
        assert_eq!(
            result["output_path"],
            clean_windows_prefix(&canonical.to_string_lossy())
        );
        fs::write(root.path().join(".keep"), []).expect("hidden file");
        assert!(analyze(vec![canonical.to_string_lossy().into()]).is_err());
        fs::remove_file(root.path().join(".keep")).expect("remove fixture file");
        fs::create_dir(root.path().join("nested")).expect("subdirectory");
        assert!(analyze(vec![canonical.to_string_lossy().into()]).is_err());
    }
    #[test]
    fn multiple_folders_prefer_dedicated_subtitles_and_accept_unicode() {
        let root = tempfile::tempdir().expect("fixture");
        let media = root.path().join("\u{30b1}\u{30ed}\u{30ed} & anime");
        let subtitles = root.path().join("Subtitles");
        let output = root.path().join("Output");
        fs::create_dir(&media).expect("media");
        fs::create_dir(&subtitles).expect("subtitles");
        fs::create_dir(&output).expect("output");
        fs::write(media.join("Episode01.MKV"), []).expect("video");
        fs::write(media.join("Episode01.srt"), []).expect("mixed subtitle");
        fs::write(subtitles.join("Episode01.ASS"), []).expect("dedicated subtitle");
        let result = analyze(vec![
            media.to_string_lossy().into(),
            subtitles.to_string_lossy().into(),
            output.to_string_lossy().into(),
        ])
        .expect("analyze");
        assert_eq!(
            result["media_path"],
            clean_windows_prefix(
                &fs::canonicalize(media)
                    .expect("canonical")
                    .to_string_lossy()
            )
        );
        assert_eq!(
            result["sub_path"],
            clean_windows_prefix(
                &fs::canonicalize(subtitles)
                    .expect("canonical")
                    .to_string_lossy()
            )
        );
        assert_eq!(
            result["output_path"],
            clean_windows_prefix(
                &fs::canonicalize(output)
                    .expect("canonical")
                    .to_string_lossy()
            )
        );
    }
    #[test]
    fn file_drops_use_parents_and_nested_directories_are_not_scanned() {
        let root = tempfile::tempdir().expect("fixture");
        let nested = root.path().join("nested");
        fs::create_dir(&nested).expect("nested");
        let video = nested.join("Episode01.webm");
        let subtitle = nested.join("Episode01.VTT");
        fs::write(&video, []).expect("video");
        fs::write(&subtitle, []).expect("subtitle");
        assert!(analyze(vec![root.path().to_string_lossy().into()]).is_err());
        let result = analyze(vec![
            video.to_string_lossy().into(),
            subtitle.to_string_lossy().into(),
        ])
        .expect("files");
        assert_eq!(result["media_path"], result["sub_path"]);
        assert_eq!(
            result["media_path"],
            clean_windows_prefix(
                &fs::canonicalize(nested)
                    .expect("canonical")
                    .to_string_lossy()
            )
        );
    }
    #[test]
    fn subtitle_only_and_unsupported_drops_have_clear_results() {
        let root = tempfile::tempdir().expect("fixture");
        let subtitle = root.path().join("Episode01.SSA");
        fs::write(&subtitle, []).expect("subtitle");
        let result = analyze(vec![subtitle.to_string_lossy().into()]).expect("subtitle only");
        assert!(result["media_path"].is_null());
        let unsupported = root.path().join("readme.txt");
        fs::write(&unsupported, []).expect("other file");
        assert!(analyze(vec![unsupported.to_string_lossy().into()]).is_err());
        assert!(analyze(Vec::new()).is_err());
        assert!(analyze(vec![String::from("x"); MAX_PATHS + 1]).is_err());
    }
}
```

## src-tauri/crates/encoder-core/src/environment.rs

```rust
use std::path::Path;

/// Presence/access check only. Metadata does not verify a cryptographic signature.
pub async fn validate_binary(path: &Path, name: &str) -> Result<(), String> {
    let metadata = tokio::fs::metadata(path).await.map_err(|error|
        format!("ENVIRONMENT_UNAVAILABLE: Bundled {name} is missing or inaccessible: {} ({error}). Restore the complete application package.", path.display()))?;
    if !metadata.is_file() || metadata.len() == 0 {
        return Err(format!(
            "ENVIRONMENT_UNAVAILABLE: Bundled {name} is not a nonempty regular file: {}. Restore the complete application package.",
            path.display()
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn missing_empty_and_directory_binaries_are_rejected() {
        let root = tempfile::tempdir().expect("fixture");
        let binary = root.path().join("ffmpeg.exe");
        assert!(
            validate_binary(&binary, "ffmpeg")
                .await
                .err()
                .expect("missing")
                .contains("ENVIRONMENT_UNAVAILABLE")
        );
        tokio::fs::write(&binary, []).await.expect("empty");
        assert!(validate_binary(&binary, "ffmpeg").await.is_err());
        assert!(validate_binary(root.path(), "ffmpeg").await.is_err());
        tokio::fs::write(&binary, b"MZfixture")
            .await
            .expect("nonempty");
        assert!(validate_binary(&binary, "ffmpeg").await.is_ok());
    }
}
```

## src-tauri/crates/encoder-core/src/job_fence.rs

```rust
/// Parent-owned Windows job: assigned media children are terminated by the OS
/// when the app is force-killed and its final job handle closes.
#[cfg(windows)]
pub fn assign(child: &tokio::process::Child) -> Result<(), String> {
    use std::{
        os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
        sync::OnceLock,
    };
    use windows_sys::Win32::System::JobObjects::*;
    static JOB: OnceLock<Result<OwnedHandle, String>> = OnceLock::new();
    let job = JOB
        .get_or_init(|| {
            // SAFETY: unnamed, non-inheritable job with default security attributes.
            let raw = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
            if raw.is_null() {
                return Err(std::io::Error::last_os_error().to_string());
            }
            // SAFETY: unique valid owned job handle.
            let handle = unsafe { OwnedHandle::from_raw_handle(raw) };
            let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            // SAFETY: correctly typed initialized structure, valid job handle.
            if unsafe {
                SetInformationJobObject(
                    raw,
                    JobObjectExtendedLimitInformation,
                    &limits as *const _ as *const _,
                    std::mem::size_of_val(&limits) as u32,
                )
            } == 0
            {
                return Err(std::io::Error::last_os_error().to_string());
            }
            Ok(handle)
        })
        .as_ref()
        .map_err(|error| format!("Cannot create child-process safety job: {error}"))?;
    let raw_child = child
        .raw_handle()
        .ok_or("Child exited before job assignment.")?;
    // SAFETY: both handles remain owned/live. Assignment is before async processing.
    if unsafe { AssignProcessToJobObject(job.as_raw_handle(), raw_child) } == 0 {
        return Err(format!(
            "Cannot assign media child to safety job: {}",
            std::io::Error::last_os_error()
        ));
    }
    Ok(())
}
#[cfg(not(windows))]
pub fn assign(_: &tokio::process::Child) -> Result<(), String> {
    Ok(())
}
```

## src-tauri/crates/encoder-core/src/lib.rs

```rust
use serde::{Deserialize, Serialize};
pub mod artifacts;
pub mod atomic_file;
mod cancel;
mod diagnostics;
pub mod drop_scan;
pub mod environment;
mod job_fence;
pub mod local_log;
pub mod offline;
mod output;
pub mod power;
pub mod preflight;
mod process_state;
pub mod progress;
pub mod session;
pub mod subtitles;
pub use process_state::ActiveProcess;
pub mod media;
pub use cancel::Cancellation;
use std::{
    collections::HashSet,
    ffi::OsString,
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{
    fs,
    io::{AsyncBufReadExt, BufReader},
    process::Command,
};

/// Container candidates; FFmpeg validates the actual streams and installed decoders.
pub const VIDEO_EXTENSIONS: &[&str] = &["mp4", "mkv", "avi", "mov", "wmv", "flv", "webm"];

pub fn is_video_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            VIDEO_EXTENSIONS
                .iter()
                .any(|supported| ext.eq_ignore_ascii_case(supported))
        })
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub input_dir: String,
    pub output_dir: String,
    pub crf: u8,
    #[serde(default)]
    pub subtitle_dir: Option<String>,
    #[serde(default)]
    pub anime_title: String,
}

#[derive(Clone, Debug)]
pub struct Tools {
    pub ffmpeg: PathBuf,
    pub ffprobe: PathBuf,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Item {
    input: PathBuf,
    output: PathBuf,
    subtitle: Option<PathBuf>,
    subtitle_root: Option<PathBuf>,
    validation_error: Option<String>,
}

#[derive(Clone, Debug)]
pub struct BatchPlan {
    pub job_id: String,
    settings: Settings,
    items: Vec<Item>,
    pub kind: TaskKind,
    warnings: Vec<ItemFailure>,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskKind {
    Batch,
    Preview,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Queued,
    Running,
    Succeeded,
    CompletedWithErrors,
    Failed,
    Cancelled,
}

impl Phase {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Succeeded | Self::CompletedWithErrors | Self::Failed | Self::Cancelled
        )
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemFailure {
    pub file_name: String,
    pub reason: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub kind: TaskKind,
    pub job_id: String,
    pub sequence: u64,
    pub phase: Phase,
    pub current_file: usize,
    pub total_files: usize,
    pub file_name: Option<String>,
    pub subtitle_name: Option<String>,
    pub file_percent: Option<f64>,
    pub batch_percent: f64,
    pub succeeded: usize,
    pub skipped: usize,
    pub failed: usize,
    pub elapsed_seconds: f64,
    pub fps: Option<f64>,
    pub speed: Option<String>,
    pub eta_seconds: Option<f64>,
    pub message: String,
    pub errors: Vec<ItemFailure>,
    pub warnings: Vec<ItemFailure>,
}

enum ItemUpdate {
    Progress(Option<f64>, Option<f64>, Option<String>, Option<f64>),
    Warning(String),
    SubtitlesOff,
}

impl BatchPlan {
    pub fn settings(&self) -> &Settings {
        &self.settings
    }
    pub async fn prepare(job_id: String, settings: Settings) -> Result<Self, String> {
        Self::prepare_inner(job_id, settings, TaskKind::Batch).await
    }

    pub async fn prepare_preview(
        job_id: String,
        mut settings: Settings,
        directory: &Path,
    ) -> Result<Self, String> {
        settings.output_dir = directory.to_string_lossy().into_owned();
        Self::prepare_inner(job_id, settings, TaskKind::Preview).await
    }

    async fn prepare_inner(
        job_id: String,
        mut settings: Settings,
        kind: TaskKind,
    ) -> Result<Self, String> {
        offline::local_path(Path::new(&settings.input_dir))?;
        offline::local_path(Path::new(&settings.output_dir))?;
        if let Some(path) = &settings.subtitle_dir {
            if !path.trim().is_empty() {
                offline::local_path(Path::new(path))?;
            }
        }
        if job_id.is_empty()
            || job_id.len() > 128
            || !job_id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_'))
        {
            return Err("Invalid internal job identifier.".into());
        }
        if !matches!(settings.crf, 20 | 23 | 26) {
            return Err("Choose CRF 20, 23, or 26.".into());
        }
        let raw_title = settings.anime_title.trim();
        let title = media::clean_title(raw_title)?;
        if !raw_title.is_empty() && title.is_empty() {
            return Err("The anime title must contain filename-safe characters.".into());
        }
        settings.anime_title = title;
        let input = fs::canonicalize(&settings.input_dir)
            .await
            .map_err(|e| format!("Cannot open input folder: {e}"))?;
        if !fs::metadata(&input)
            .await
            .map_err(|e| e.to_string())?
            .is_dir()
        {
            return Err("Input path must be a folder.".into());
        }
        let mut subtitles = Vec::new();
        let mut subtitle_root = None;
        if let Some(directory) = settings
            .subtitle_dir
            .as_deref()
            .filter(|s| !s.trim().is_empty())
        {
            let directory = fs::canonicalize(directory)
                .await
                .map_err(|e| format!("Cannot canonicalize subtitle folder: {e}"))?;
            if !fs::metadata(&directory)
                .await
                .map_err(|e| e.to_string())?
                .is_dir()
            {
                return Err("Subtitle path must be a directory.".into());
            }
            subtitle_root = Some(directory.clone());
            let mut entries = fs::read_dir(&directory)
                .await
                .map_err(|e| format!("Cannot read subtitle folder: {e}"))?;
            while let Some(entry) = entries.next_entry().await.map_err(|e| e.to_string())? {
                let path = entry.path();
                if entry
                    .file_type()
                    .await
                    .map_err(|e| e.to_string())?
                    .is_file()
                    && subtitles::extension(&path).is_some()
                {
                    // Deletion/unreadability after scanning is handled per item during staging.
                    subtitles.push(path);
                }
            }
        }
        let mut videos = Vec::new();
        let mut entries = fs::read_dir(&input).await.map_err(|e| e.to_string())?;
        while let Some(entry) = entries.next_entry().await.map_err(|e| e.to_string())? {
            let path = entry.path();
            if entry
                .file_type()
                .await
                .map_err(|e| e.to_string())?
                .is_file()
                && is_video_path(&path)
            {
                videos.push(path);
            }
        }
        if videos.is_empty() {
            return Err("No supported videos found in the input folder.".into());
        }
        let (mut videos, mut pairing) = session::blocking(move || {
            videos.sort_by(|a, b| media::compare_videos(a, b));
            let pairing = media::pair_subtitles(&videos, &subtitles);
            Ok((videos, pairing))
        })
        .await?;
        if kind == TaskKind::Preview {
            videos.truncate(1);
            pairing.matches.truncate(1);
        }
        if settings.output_dir.trim().is_empty() {
            return Err("Choose an output folder.".into());
        }
        fs::create_dir_all(&settings.output_dir)
            .await
            .map_err(|e| preflight::access_error("Cannot create output folder", e))?;
        let output = fs::canonicalize(&settings.output_dir)
            .await
            .map_err(|e| preflight::access_error("Cannot open output folder", e))?;
        if input == output {
            return Err(
                "Choose a separate output folder to avoid re-encoding previous outputs.".into(),
            );
        }
        let mut warnings = Vec::new();
        if subtitle_root.is_some() {
            for reason in pairing.warnings {
                log::warn!("Subtitle pairing: {reason}");
                warnings.push(ItemFailure {
                    file_name: "Subtitle pairing".into(),
                    reason,
                });
            }
        }
        if let Some(reason) = preflight::output(&output).await? {
            warnings.push(ItemFailure {
                file_name: "Output folder".into(),
                reason,
            });
        }
        let mut destinations = HashSet::new();
        settings.input_dir = input
            .to_str()
            .ok_or("Input path cannot be saved as Unicode.")?
            .to_owned();
        settings.output_dir = output
            .to_str()
            .ok_or("Output path cannot be saved as Unicode.")?
            .to_owned();
        if let Some(root) = &subtitle_root {
            settings.subtitle_dir = Some(
                root.to_str()
                    .ok_or("Subtitle path cannot be saved as Unicode.")?
                    .to_owned(),
            );
        }
        let mut items = Vec::new();
        for (index, (path, subtitle)) in videos.into_iter().zip(pairing.matches).enumerate() {
            let name = if kind == TaskKind::Preview {
                OsString::from("preview_temp.mp4")
            } else {
                media::output_name(&path, &settings.anime_title, index + 1)?
            };
            if !destinations.insert(name.to_string_lossy().to_lowercase()) {
                return Err(format!(
                    "Multiple inputs produce {}. Rename one input before encoding.",
                    name.to_string_lossy()
                ));
            }
            items.push(Item {
                input: path,
                output: output.join(name),
                subtitle,
                subtitle_root: subtitle_root.clone(),
                validation_error: None,
            });
        }
        Ok(Self {
            job_id,
            settings,
            items,
            kind,
            warnings,
        })
    }

    pub fn preview_path(&self) -> Option<PathBuf> {
        if self.kind == TaskKind::Preview {
            self.items.first().map(|item| item.output.clone())
        } else {
            None
        }
    }

    pub fn initial_progress(&self) -> Progress {
        Progress {
            kind: self.kind,
            subtitle_name: None,
            job_id: self.job_id.clone(),
            sequence: 0,
            phase: Phase::Queued,
            current_file: 0,
            total_files: self.items.len(),
            file_name: None,
            file_percent: None,
            batch_percent: 0.0,
            succeeded: 0,
            skipped: 0,
            failed: 0,
            elapsed_seconds: 0.0,
            fps: None,
            speed: None,
            eta_seconds: None,
            message: format!("Queued {} video(s).", self.items.len()),
            errors: Vec::new(),
            warnings: self.warnings.clone(),
        }
    }
}

fn process(path: &Path) -> Command {
    let mut command = Command::new(path);
    command.stdin(Stdio::null()).kill_on_drop(true);
    #[cfg(windows)]
    command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW: no console flashing per item.
    command
}

async fn duration_seconds(
    tools: &Tools,
    input: &Path,
    cancellation: &Cancellation,
    processes: &Arc<ActiveProcess>,
) -> Option<f64> {
    use tokio::io::AsyncReadExt;
    let mut child = process(&tools.ffprobe)
        .args([
            "-v",
            "error",
            "-protocol_whitelist",
            "file,pipe",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
        ])
        .arg(input)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let mut stdout = child.stdout.take()?;
    let child = processes.track(child).ok()?;
    let reader = tokio::spawn(async move {
        let mut bytes = Vec::new();
        stdout.read_to_end(&mut bytes).await.map(|_| bytes)
    });
    let status = tokio::select! {
        result = child.wait() => result.ok(),
        _ = cancellation.cancelled() => { let _ = child.kill_and_wait().await; None },
        _ = tokio::time::sleep(Duration::from_secs(30)) => { let _ = child.kill_and_wait().await; None },
    };
    if status.is_none() {
        reader.abort();
        let _ = reader.await;
        return None;
    }
    let bytes = reader.await.ok()?.ok()?;
    if !status?.success() {
        return None;
    }
    let seconds = String::from_utf8_lossy(&bytes).trim().parse::<f64>().ok()?;
    (seconds.is_finite() && seconds > 0.0).then_some(seconds)
}

pub fn file_percent(out_time_us: &str, duration: Option<f64>) -> Option<f64> {
    file_percent_seconds(out_time_us.parse::<f64>().ok()? / 1_000_000.0, duration)
}

pub fn file_percent_seconds(seconds: f64, duration: Option<f64>) -> Option<f64> {
    let duration = duration.filter(|d| d.is_finite() && *d > 0.0)?;
    if !seconds.is_finite() {
        return None;
    }
    // 100% is reserved for a successful exit and finalized output.
    Some((seconds / duration * 100.0).clamp(0.0, 99.9))
}

async fn finalize_output(
    temp: &Path,
    output: &Path,
    cancellation: &Cancellation,
    checkpoint: Option<(&Arc<session::SessionStore>, usize)>,
) -> Result<Option<String>, String> {
    if cancellation.is_cancelled() {
        return Err("Encoding cancelled.".into());
    }
    let source = temp.to_path_buf();
    let destination = output.to_path_buf();
    if let Some((store, index)) = checkpoint {
        let store = store.clone();
        let stop = cancellation.clone();
        let result = session::blocking(move || {
            store.commit_checked(index, &source, &destination, &|| stop.is_cancelled())
        })
        .await;
        if result.is_err() && cancellation.is_cancelled() {
            return Err("Encoding cancelled.".into());
        }
        result.map_err(|error| {
            format!("STATE_PERSISTENCE_FAILED: {error}. Any completed output was preserved.")
        })?;
    } else {
        session::blocking(move || {
            let file = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(&source)
                .map_err(|e| e.to_string())?;
            file.sync_all().map_err(|e| e.to_string())?;
            drop(file);
            atomic_file::rename(&source, &destination, false)
        })
        .await?;
    }
    Ok(None)
}

async fn encode_item(
    tools: &Tools,
    settings: &Settings,
    item: &Item,
    kind: TaskKind,
    cancellation: &Cancellation,
    processes: &Arc<ActiveProcess>,
    checkpoint: Option<(&Arc<session::SessionStore>, usize)>,
    mut update: impl FnMut(ItemUpdate),
) -> Result<Option<String>, String> {
    if cancellation.is_cancelled() {
        return Err("Encoding cancelled.".into());
    }
    if let Some(error) = &item.validation_error {
        return Err(error.clone());
    }
    if fs::try_exists(&item.output)
        .await
        .map_err(|e| e.to_string())?
    {
        return Err("Output already exists; it was preserved. Choose another output folder or move the existing output.".into());
    }
    let parent = item.output.parent().ok_or("Invalid output directory.")?;
    if let Some(warning) = preflight::space(parent).await? {
        update(ItemUpdate::Warning(warning));
    }
    let temp = output::partial_path(&item.output)?;
    let mut staged: Option<subtitles::StagedSubtitle> = None;
    let reserved = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .await
        .map_err(|e| format!("Cannot reserve temporary output: {e}"))?;
    drop(reserved);
    if let Err(error) = artifacts::record(&temp).await {
        let _ = fs::remove_file(&temp).await;
        return Err(format!("Cannot journal temporary output: {error}"));
    }
    let result = async {
        let duration = duration_seconds(tools, &item.input, cancellation, processes).await;
        processes.wait_until_idle().await;
        if cancellation.is_cancelled() { return Err("Encoding cancelled.".into()); }
        let duration = if kind == TaskKind::Preview { duration.map(|d| d.min(10.0)) } else { duration };
        let mut filter = String::from("scale=640:480:force_original_aspect_ratio=decrease:force_divisible_by=2,pad=640:480:(ow-iw)/2:(oh-ih)/2,setsar=1");
        if let Some(source) = &item.subtitle {
            match subtitles::read_scoped(source, item.subtitle_root.as_deref()).await {
                Ok(decoded) => {
                    if cancellation.is_cancelled() { return Err("Encoding cancelled.".into()); }
                    if let Some(warning) = decoded.warning { update(ItemUpdate::Warning(warning)); }
                    let ext = subtitles::extension(source).ok_or("Invalid subtitle extension.")?;
                    let owned = subtitles::stage(decoded.text, ext).await?;
                    filter.push_str(",subtitles=filename=");
                    filter.push_str(owned.filename()?);
                    filter.push_str(":charenc=UTF-8");
                    staged = Some(owned);
                },
                Err(subtitles::ReadError::Syntax(reason)) => {
                    return Err(format!("Subtitle syntax error: {reason}"));
                },
                Err(error) => {
                    update(ItemUpdate::Warning(format!("{error} Encoding without external subtitles.")));
                    update(ItemUpdate::SubtitlesOff);
                },
            }
        }
        let has_subtitles = staged.is_some();
        if cancellation.is_cancelled() { return Err("Encoding cancelled.".into()); }
        let mut command = process(&tools.ffmpeg);
        let threads = progress::thread_budget(num_cpus::get()).to_string();
        command.args(["-threads", &threads, "-filter_threads", &threads]);
        command.current_dir(staged.as_ref().map_or(parent, |subtitle| subtitle.directory())).args(["-hide_banner", "-loglevel", "repeat+level+info", "-stdin", "-y", "-progress", "pipe:1", "-stats_period", "0.25", "-stats"]);
        if kind == TaskKind::Preview { command.args(["-ss", "00:00:00"]); }
        command.args(["-protocol_whitelist", "file,pipe"]).arg("-i").arg(&item.input);
        if kind == TaskKind::Preview { command.args(["-t", "10"]); }
        command.args(["-map", "0:v:0", "-map", "0:a:0?", "-sn", "-dn", "-vf"]).arg(&filter)
            .args(["-c:v", "libx264", "-threads", &threads, "-preset", "medium", "-pix_fmt", "yuv420p", "-crf"]).arg(settings.crf.to_string())
            .args(["-c:a", "aac", "-b:a", "128k", "-ac", "2", "-ar", "44100", "-movflags", "+faststart", "-f", "mp4"]);
        let mut child = command.arg(&temp).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().map_err(|e| format!("Cannot start FFmpeg: {e}"))?;
        let stdout = child.stdout.take().ok_or("FFmpeg progress pipe is unavailable.")?;
        let stderr = child.stderr.take().ok_or("FFmpeg diagnostic pipe is unavailable.")?;
        let mut child = processes.track(child)?;
        child.hold_subtitle(staged.take());
        let (stderr_updates, stderr_metrics) = tokio::sync::watch::channel(progress::StderrProgress::default());
        let diagnostics = tokio::spawn(async move {
            let mut telemetry = progress::StderrProgress::default();
            diagnostics::monitor(stderr, move |line| {
                if telemetry.update(line) { stderr_updates.send_replace(telemetry.clone()); }
            }).await
        });
        let mut lines = BufReader::new(stdout).lines();
        let (mut position, mut fps, mut speed, mut read_error) = (None, None, None, None);
        loop {
            let next = tokio::select! {
                _ = cancellation.cancelled() => break,
                next = lines.next_line() => next,
            };
            let line = match next { Ok(Some(line)) => line, Ok(None) => break, Err(error) => { read_error = Some(error); break; } };
            let Some((key, value)) = line.split_once('=') else { continue; };
            match key {
                "out_time_us" => position = value.parse::<f64>().ok().filter(|time| time.is_finite() && *time >= 0.0).map(|time| time / 1_000_000.0),
                "out_time" if position.is_none() => position = progress::timestamp_seconds(value),
                "fps" => fps = value.parse::<f64>().ok().filter(|f| f.is_finite() && *f >= 0.0),
                "speed" => speed = (value != "N/A").then(|| value.chars().take(24).collect::<String>()),
                "progress" => {
                    let human = stderr_metrics.borrow().clone();
                    let total = duration.or(human.duration).map(|duration| if kind == TaskKind::Preview { duration.min(10.0) } else { duration });
                    let current = position.or(human.position);
                    let current_speed = speed.clone().or_else(|| human.speed.map(|speed| format!("{speed:.2}x")));
                    let rate = current_speed.as_deref().and_then(progress::speed_factor);
                    let percent = current.and_then(|current| file_percent_seconds(current, total));
                    let eta = progress::eta_seconds(current, total, rate);
                    update(ItemUpdate::Progress(percent, fps, current_speed, eta));
                },
                _ => {},
            }
        }
        let stop_result = if cancellation.is_cancelled() || read_error.is_some() { child.stop().await } else { Ok(()) };
        let status = tokio::select! {
            result = child.wait() => result.map_err(|e| format!("Cannot wait for FFmpeg: {e}")),
            _ = cancellation.cancelled(), if !cancellation.is_cancelled() => {
                child.stop().await?;
                child.wait().await.map_err(|e| e.to_string())
            },
        };
        let diagnostic = diagnostics.await.map_err(|e| format!("Cannot join FFmpeg diagnostics: {e}"))??;
        stop_result?;
        if cancellation.is_cancelled() { return Err("Encoding cancelled.".into()); }
        if let Some(error) = read_error { return Err(format!("Cannot read FFmpeg progress: {error}")); }
        if !status?.success() {
            log::error!("FFmpeg failure for {}. Last stderr lines:\n{}", item.input.display(), diagnostic);
            let category = if has_subtitles { "FFmpeg subtitle/filter or media parse failure" } else { "FFmpeg failed" };
            return Err(format!("{category}. See keroro-encoder.log for decoder/parser diagnostics."));
        }
        if has_subtitles && stderr_metrics.borrow().has_diagnostics {
            log::warn!("FFmpeg subtitle/filter diagnostics for {}:\n{}", item.input.display(), diagnostic);
            update(ItemUpdate::Warning("FFmpeg reported subtitle/filter diagnostics. See keroro-encoder.log.".into()));
        }
        finalize_output(&temp, &item.output, cancellation, checkpoint).await
    }.await;
    // An early pipe/read/wait error can drop the lease before the child exits.
    // Finish reaping before deleting files or starting the next batch item.
    processes.wait_until_idle().await;
    let mut cleanup_errors = Vec::new();
    if result.is_err() {
        if let Err(error) = fs::remove_file(&temp).await {
            if error.kind() != std::io::ErrorKind::NotFound {
                cleanup_errors.push(format!("temporary output: {error}"));
            }
        }
    }
    if !temp.exists() {
        if let Err(error) = artifacts::forget(&temp).await {
            cleanup_errors.push(error);
        }
    }
    if let Some(staged) = staged {
        if let Err(error) = staged.close() {
            cleanup_errors.push(error);
        }
    }
    if !cleanup_errors.is_empty() {
        let details = cleanup_errors.join("; ");
        return match result {
            Ok(_) => Ok(Some(format!(
                "Output saved; cleanup needs attention: {details}"
            ))),
            Err(error) => Err(format!("{error} Cleanup failed: {details}")),
        };
    }
    result
}

pub async fn run_batch(
    plan: BatchPlan,
    tools: Tools,
    cancellation: Cancellation,
    publish: impl FnMut(Progress),
) -> Progress {
    run_batch_with_session(
        plan,
        tools,
        cancellation,
        Arc::new(ActiveProcess::default()),
        None,
        publish,
    )
    .await
}

pub async fn run_batch_with_session(
    plan: BatchPlan,
    tools: Tools,
    cancellation: Cancellation,
    processes: Arc<ActiveProcess>,
    checkpoint: Option<Arc<session::SessionStore>>,
    mut publish: impl FnMut(Progress),
) -> Progress {
    let start = Instant::now();
    let mut state = plan.initial_progress();
    state.phase = Phase::Running;
    let mut interrupted = false;
    for (index, item) in plan.items.iter().enumerate() {
        if cancellation.is_cancelled() {
            state.phase = Phase::Cancelled;
            break;
        }
        let saved = match checkpoint
            .as_ref()
            .map(|store| store.completed(index))
            .transpose()
        {
            Ok(saved) => saved.unwrap_or(false),
            Err(error) => {
                state.message = error;
                interrupted = true;
                break;
            }
        };
        let existing = if plan.kind == TaskKind::Batch && !saved {
            let path = item.output.clone();
            match session::blocking(move || output::existing(&path)).await {
                Ok(existing) => existing,
                Err(error) => {
                    log::warn!("Cannot smart-skip {}: {error}", item.output.display());
                    false
                }
            }
        } else {
            false
        };
        if saved || existing {
            if existing {
                if let Some(store) = &checkpoint {
                    let store = store.clone();
                    if let Err(error) =
                        session::blocking(move || store.record_existing(index)).await
                    {
                        log::error!("Cannot checkpoint skipped output: {error}");
                        state.message = error;
                        interrupted = true;
                        break;
                    }
                }
            }
            state.current_file = index + 1;
            state.skipped += 1;
            state.file_name = item
                .input
                .file_name()
                .map(|name| name.to_string_lossy().into_owned());
            state.subtitle_name = None;
            state.file_percent = Some(100.0);
            state.eta_seconds = Some(0.0);
            state.fps = None;
            state.speed = None;
            state.batch_percent = (index + 1) as f64 / state.total_files as f64 * 100.0;
            state.elapsed_seconds = start.elapsed().as_secs_f64();
            state.message = format!(
                "Skipped episode {}: final output already exists; preserved {}.",
                index + 1,
                item.output.display()
            );
            log::info!("{}", state.message);
            state.sequence += 1;
            publish(state.clone());
            continue;
        }
        let name = item
            .input
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Video".into());
        log::info!("Encoding episode {}: {}", index + 1, item.input.display());
        state.current_file = index + 1;
        state.file_name = Some(name.clone());
        state.subtitle_name = item
            .subtitle
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned());
        state.file_percent = None;
        state.fps = None;
        state.speed = None;
        state.eta_seconds = None;
        state.batch_percent = index as f64 / state.total_files as f64 * 100.0;
        state.message = format!(
            "Encoding file {} of {}: {name}",
            index + 1,
            state.total_files
        );
        state.sequence += 1;
        publish(state.clone());
        if plan
            .settings
            .subtitle_dir
            .as_deref()
            .is_some_and(|dir| !dir.trim().is_empty())
            && item.subtitle.is_none()
            && item.validation_error.is_none()
        {
            let reason =
                "No matching .srt/.ass/.ssa/.vtt subtitle; encoding without external subtitles."
                    .to_string();
            state.warnings.push(ItemFailure {
                file_name: name.clone(),
                reason: reason.clone(),
            });
            state.message = format!("[Warning] {name}: {reason}");
            state.sequence += 1;
            publish(state.clone());
        }
        let mut progress_gate = progress::ProgressGate::default();
        let result = encode_item(
            &tools,
            &plan.settings,
            item,
            plan.kind,
            &cancellation,
            &processes,
            checkpoint.as_ref().map(|store| (store, index)),
            |update| {
                match update {
                    ItemUpdate::Progress(percent, fps, speed, eta) => {
                        state.file_percent = percent;
                        state.batch_percent = (index as f64 + percent.unwrap_or(0.0) / 100.0)
                            / state.total_files as f64
                            * 100.0;
                        state.fps = fps;
                        state.speed = speed;
                        state.eta_seconds = eta;
                        let percent_changed = progress_gate.should_emit(percent);
                        let eta_changed = progress_gate.should_emit_eta(eta);
                        if !percent_changed && !eta_changed {
                            return;
                        }
                    }
                    ItemUpdate::Warning(reason) => {
                        log::warn!("{}: {}", name, reason);
                        state.message = format!("[Warning] {name}: {reason}");
                        state.warnings.push(ItemFailure {
                            file_name: name.clone(),
                            reason,
                        });
                    }
                    ItemUpdate::SubtitlesOff => state.subtitle_name = None,
                }
                state.elapsed_seconds = start.elapsed().as_secs_f64();
                state.sequence += 1;
                publish(state.clone());
            },
        )
        .await;
        if cancellation.is_cancelled() {
            state.phase = Phase::Cancelled;
            if result.is_ok() {
                state.succeeded += 1;
                state.file_percent = Some(100.0);
                state.eta_seconds = Some(0.0);
            }
            if let Err(reason) = &result {
                if reason != "Encoding cancelled." {
                    state.errors.push(ItemFailure {
                        file_name: name.clone(),
                        reason: reason.clone(),
                    });
                }
            }
            break;
        }
        match result {
            Ok(warning) => {
                state.succeeded += 1;
                state.file_percent = Some(100.0);
                state.eta_seconds = Some(0.0);
                if let Some(reason) = warning {
                    state.message = format!("[Warning] {name}: {reason}");
                    state.warnings.push(ItemFailure {
                        file_name: name.clone(),
                        reason,
                    });
                } else {
                    state.message = format!("Saved {name}.");
                }
            }
            Err(reason) => {
                state.eta_seconds = None;
                log::error!("Failed {}: {}", name, reason);
                interrupted = reason.starts_with("STATE_PERSISTENCE_FAILED:");
                state.failed += 1;
                state.message = format!("[Error] Failed to process {name}: {reason}. Skipping...");
                state.errors.push(ItemFailure {
                    file_name: name,
                    reason,
                });
            }
        }
        state.batch_percent = (index + 1) as f64 / state.total_files as f64 * 100.0;
        state.elapsed_seconds = start.elapsed().as_secs_f64();
        state.sequence += 1;
        publish(state.clone());
        if interrupted {
            break;
        }
    }
    state.phase = if interrupted {
        Phase::Failed
    } else if state.phase == Phase::Cancelled {
        Phase::Cancelled
    } else if state.failed == 0 {
        Phase::Succeeded
    } else if state.succeeded + state.skipped > 0 {
        Phase::CompletedWithErrors
    } else {
        Phase::Failed
    };
    if !interrupted && state.phase != Phase::Cancelled {
        if let Some(store) = &checkpoint {
            let store = store.clone();
            let job_id = plan.job_id.clone();
            if let Err(error) = session::blocking(move || store.finish(&job_id)).await {
                log::error!("Cannot clear finished checkpoint: {error}");
                state.warnings.push(ItemFailure { file_name: "Recovery state".into(), reason: "Completed output was saved, but the finished checkpoint could not be cleared. Review recovery on the next launch.".into() });
            }
        }
    }
    state.message = if interrupted {
        "Encoding paused because recovery state could not be saved. Completed outputs were preserved; review the local log.".into()
    } else if state.phase == Phase::Cancelled {
        format!(
            "Task cancelled. {} completed output(s) preserved; {} skipped.",
            state.succeeded, state.skipped
        )
    } else {
        format!(
            "Batch complete: {} encoded, {} skipped, {} failed, {} total. {} warning(s).",
            state.succeeded,
            state.skipped,
            state.failed,
            state.total_files,
            state.warnings.len()
        )
    };
    state.sequence += 1;
    state.elapsed_seconds = start.elapsed().as_secs_f64();
    publish(state.clone());
    state
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn progress_handles_unknown_duration_and_invalid_timestamps() {
        assert_eq!(file_percent("5000000", Some(10.0)), Some(50.0));
        assert_eq!(file_percent("5000000", None), None);
        assert_eq!(file_percent("N/A", Some(10.0)), None);
        assert_eq!(file_percent("NaN", Some(10.0)), None);
        assert_eq!(file_percent("10000000", Some(0.0)), None);
        assert_eq!(file_percent("12000000", Some(10.0)), Some(99.9));
        assert_eq!(file_percent("-1000000", Some(10.0)), Some(0.0));
    }
}
```

## src-tauri/crates/encoder-core/src/local_log.rs

```rust
use log::{LevelFilter, Log, Metadata, Record};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};

const LIMIT: u64 = 16 * 1024 * 1024;
struct Sink {
    file: Option<File>,
    bytes: u64,
}
struct LocalLogger {
    path: PathBuf,
    sink: Mutex<Sink>,
}
static LOGGER: OnceLock<LocalLogger> = OnceLock::new();

impl LocalLogger {
    fn write(&self, line: &[u8]) {
        let Ok(mut sink) = self.sink.try_lock() else {
            // A panic while another record is being written must not deadlock.
            if let Ok(mut file) = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)
            {
                let _ = file.write_all(line);
                let _ = file.sync_data();
            }
            return;
        };
        let mut rotation_error = None;
        if sink.bytes.saturating_add(line.len() as u64) > LIMIT {
            sink.file.take();
            let backup = self.path.with_extension("log.1");
            rotation_error = fs::rename(&self.path, &backup).err();
            sink.file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)
                .ok();
            sink.bytes = fs::metadata(&self.path).map(|m| m.len()).unwrap_or(0);
        }
        if let Some(file) = sink.file.as_mut() {
            // Logging through log! here would recurse into this locked sink.
            if let Some(error) = rotation_error {
                let _ = writeln!(file, "[WARN] [logger] Cannot rotate local log: {error}");
            }
            if file.write_all(line).and_then(|_| file.flush()).is_ok() {
                sink.bytes = sink.bytes.saturating_add(line.len() as u64);
            }
        }
    }
}
impl Log for LocalLogger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.level() <= log::Level::Info
    }
    fn log(&self, record: &Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|t| t.as_millis())
            .unwrap_or(0);
        self.write(
            format!(
                "{timestamp} [{}] [{}] {}\n",
                record.level(),
                record.target(),
                record.args()
            )
            .as_bytes(),
        );
    }
    fn flush(&self) {
        if let Ok(mut sink) = self.sink.try_lock() {
            if let Some(file) = sink.file.as_mut() {
                let _ = file.sync_data();
            }
        }
    }
}

pub fn install(directory: &Path) -> Result<PathBuf, String> {
    crate::offline::local_path(directory)?;
    fs::create_dir_all(directory).map_err(|e| e.to_string())?;
    crate::artifacts::no_links(directory)?;
    let path = directory.join("keroro-encoder.log");
    if path.exists() {
        crate::artifacts::no_links(&path)?;
    }
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| format!("Cannot open local log: {e}"))?;
    let bytes = file.metadata().map_err(|e| e.to_string())?.len();
    LOGGER
        .set(LocalLogger {
            path: path.clone(),
            sink: Mutex::new(Sink {
                file: Some(file),
                bytes,
            }),
        })
        .map_err(|_| "Local logger already initialized.")?;
    let logger = LOGGER.get().ok_or("Local logger unavailable.")?;
    log::set_logger(logger).map_err(|e| e.to_string())?;
    log::set_max_level(LevelFilter::Info);
    std::panic::set_hook(Box::new(|panic| {
        let trace = std::backtrace::Backtrace::force_capture();
        log::error!(target: "panic", "{panic}\n{trace}");
        log::logger().flush();
    }));
    log::info!("Local logging initialized; no telemetry or network log sink.");
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn file_sink_writes_only_to_its_local_file() {
        let root = tempfile::tempdir().expect("fixture");
        let path = root.path().join("keroro-encoder.log");
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .expect("open");
        let logger = LocalLogger {
            path: path.clone(),
            sink: Mutex::new(Sink {
                file: Some(file),
                bytes: 0,
            }),
        };
        logger.write(b"local diagnostic\n");
        logger.flush();
        assert_eq!(
            fs::read_to_string(path).expect("read"),
            "local diagnostic\n"
        );
    }
}
```

## src-tauri/crates/encoder-core/src/media.rs

```rust
use regex::Regex;
use std::{
    cmp::Ordering,
    collections::HashMap,
    ffi::OsString,
    path::{Path, PathBuf},
    sync::LazyLock,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Episode {
    pub season: Option<u32>,
    pub number: u32,
}

static PATTERNS: LazyLock<Result<Vec<Regex>, regex::Error>> =
    LazyLock::new(|| {
        [
        r"(?i)(?:^|[^a-z0-9])s(?P<season>\d{1,2})e(?P<ep>\d{1,4})(?:v\d+)?(?:$|[^0-9])",
        r"(?i)(?:^|[\s._\[\]-])(?:episode|ep|e)[\s._-]*(?P<ep>\d{1,4})(?:v\d+)?(?:$|[\s._\[\]-])",
        r"(?i)(?:^|\s)-\s*(?P<ep>\d{1,4})(?:v\d+)?(?:$|[\s._\[\]-])",
        r"(?i)^(?P<ep>\d{1,4})(?:v\d+)?$",
        r"(?i)(?:^|[\s._\[-])(?P<ep>\d{1,4})(?:v\d+)?(?:$|[\s._\]-])",
    ].into_iter().map(Regex::new).collect()
    });

pub fn episode_number(stem: &str) -> Option<Episode> {
    let patterns = PATTERNS.as_ref().ok()?;
    for (index, pattern) in patterns.iter().enumerate() {
        let mut offset = 0;
        let mut candidates = Vec::new();
        while let Some(capture) = pattern.captures_at(stem, offset) {
            let Some(number_match) = capture.name("ep") else {
                break;
            };
            offset = number_match.end();
            if let Ok(number) = number_match.as_str().parse::<u32>() {
                if index == 4
                    && ([360, 480, 576, 720, 1080, 1440, 2160, 4320].contains(&number)
                        || (1900..=2099).contains(&number))
                {
                    continue;
                }
                candidates.push(Episode {
                    season: capture.name("season").and_then(|m| m.as_str().parse().ok()),
                    number,
                });
            }
            if candidates.len() > 1 {
                return None;
            }
        }
        if candidates.len() == 1 {
            return candidates.first().copied();
        }
    }
    None
}

pub fn clean_title(title: &str) -> Result<String, String> {
    let title: String = title
        .chars()
        .map(|ch| {
            if ch.is_control() || "<>:\"/\\|?*".contains(ch) {
                ' '
            } else {
                ch
            }
        })
        .collect();
    let title = title.split_whitespace().collect::<Vec<_>>().join(" ");
    let title = title.trim_end_matches(['.', ' ']).to_owned();
    if title.chars().count() > 100 {
        return Err("Use an anime title of at most 100 characters.".into());
    }
    Ok(title)
}

pub fn output_name(input: &Path, title: &str, sequence: usize) -> Result<OsString, String> {
    if !title.is_empty() {
        return Ok(format!("{title} - Ep{sequence:02} [Wii].mp4").into());
    }
    let mut name = OsString::from(input.file_stem().ok_or("Invalid video filename.")?);
    name.push("_Wii.mp4");
    Ok(name)
}

fn natural_cmp(left: &str, right: &str) -> Ordering {
    let mut left = left.chars().peekable();
    let mut right = right.chars().peekable();
    while let (Some(&a), Some(&b)) = (left.peek(), right.peek()) {
        if a.is_ascii_digit() && b.is_ascii_digit() {
            let a: String = std::iter::from_fn(|| left.next_if(|c| c.is_ascii_digit())).collect();
            let b: String = std::iter::from_fn(|| right.next_if(|c| c.is_ascii_digit())).collect();
            let a = a.trim_start_matches('0');
            let b = b.trim_start_matches('0');
            let order = a.len().cmp(&b.len()).then_with(|| a.cmp(b));
            if order != Ordering::Equal {
                return order;
            }
        } else {
            let order = a.cmp(&b);
            if order != Ordering::Equal {
                return order;
            }
            left.next();
            right.next();
        }
    }
    left.count().cmp(&right.count())
}

pub fn compare_videos(left: &Path, right: &Path) -> Ordering {
    let left_stem = left
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    let right_stem = right
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    let left_ep = episode_number(&left_stem);
    let right_ep = episode_number(&right_stem);
    match (left_ep, right_ep) {
        (Some(a), Some(b)) => a.cmp(&b),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
    .then_with(|| compare_filenames(left, right))
}

fn compare_filenames(left: &Path, right: &Path) -> Ordering {
    let left_name = left
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    let right_name = right
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    natural_cmp(&left_name, &right_name).then_with(|| left.cmp(right))
}

struct SubtitleIndex {
    stems: HashMap<String, Vec<usize>>,
    episodes: HashMap<u32, Vec<(usize, Option<u32>)>>,
}
impl SubtitleIndex {
    fn new(subtitles: &[PathBuf]) -> Self {
        let mut index = Self {
            stems: HashMap::new(),
            episodes: HashMap::new(),
        };
        for (position, path) in subtitles.iter().enumerate() {
            let Some(stem) = path.file_stem() else {
                continue;
            };
            let stem = stem.to_string_lossy().to_lowercase();
            index.stems.entry(stem.clone()).or_default().push(position);
            if let Some(episode) = episode_number(&stem) {
                index
                    .episodes
                    .entry(episode.number)
                    .or_default()
                    .push((position, episode.season));
            }
        }
        index
    }
    fn candidate(&self, video: &Path) -> Result<Option<usize>, String> {
        let stem = video
            .file_stem()
            .ok_or("Invalid video filename.")?
            .to_string_lossy()
            .to_lowercase();
        if let Some(exact) = self.stems.get(&stem) {
            return match exact.as_slice() {
                [position] => Ok(Some(*position)),
                _ => Err(format!("Multiple subtitles exactly match {stem}.")),
            };
        }
        let Some(episode) = episode_number(&stem) else {
            return Ok(None);
        };
        let mut candidates = self
            .episodes
            .get(&episode.number)
            .into_iter()
            .flatten()
            .filter(|(_, season)| {
                episode.season.is_none() || season.is_none() || episode.season == *season
            });
        let first = candidates.next().map(|(position, _)| *position);
        if candidates.next().is_some() {
            return Err(format!(
                "Multiple subtitles match episode {} of {stem}.",
                episode.number
            ));
        }
        Ok(first)
    }
}

pub struct SubtitlePairing {
    /// One entry per video in the caller's queue order.
    pub matches: Vec<Option<PathBuf>>,
    pub warnings: Vec<String>,
}

pub fn pair_subtitles(videos: &[PathBuf], subtitles: &[PathBuf]) -> SubtitlePairing {
    if subtitles.is_empty() {
        return SubtitlePairing {
            matches: vec![None; videos.len()],
            warnings: if videos.is_empty() {
                Vec::new()
            } else {
                vec![format!(
                    "Subtitle file-count mismatch: {} videos, 0 subtitles; {} unpaired video(s). Encoding without external subtitles.",
                    videos.len(),
                    videos.len()
                )]
            },
        };
    }
    let index = SubtitleIndex::new(subtitles);
    let mut proposals = Vec::with_capacity(videos.len());
    let mut claims = vec![0usize; subtitles.len()];
    let mut ambiguous = 0;
    for video in videos {
        let candidate = match index.candidate(video) {
            Ok(candidate) => candidate,
            Err(_) => {
                ambiguous += 1;
                None
            }
        };
        if let Some(position) = candidate {
            if let Some(count) = claims.get_mut(position) {
                *count += 1;
            }
        }
        proposals.push(candidate);
    }
    let mut matches = vec![None; videos.len()];
    let mut used = vec![false; subtitles.len()];
    for (video, proposal) in proposals.into_iter().enumerate() {
        if let Some(subtitle) = proposal {
            if claims.get(subtitle) == Some(&1) {
                if let (Some(destination), Some(source), Some(in_use)) = (
                    matches.get_mut(video),
                    subtitles.get(subtitle),
                    used.get_mut(subtitle),
                ) {
                    *destination = Some(source.clone());
                    *in_use = true;
                }
            } else {
                ambiguous += 1;
            }
        }
    }
    // Indices come from these unchanged slices. Keep absolute sorted positions;
    // never shift leftovers or steal a confident match.
    let mut video_order: Vec<_> = (0..videos.len()).collect();
    let mut subtitle_order: Vec<_> = (0..subtitles.len()).collect();
    video_order.sort_by(|a, b| compare_filenames(&videos[*a], &videos[*b]));
    subtitle_order.sort_by(|a, b| compare_filenames(&subtitles[*a], &subtitles[*b]));
    let mut fallback = 0;
    for (rank, video) in video_order.into_iter().enumerate() {
        if matches.get(video).is_none_or(|pair| pair.is_some()) {
            continue;
        }
        let Some(subtitle) = subtitle_order.get(rank).copied() else {
            continue;
        };
        if used.get(subtitle) != Some(&false) {
            continue;
        }
        if let (Some(destination), Some(source), Some(in_use)) = (
            matches.get_mut(video),
            subtitles.get(subtitle),
            used.get_mut(subtitle),
        ) {
            *destination = Some(source.clone());
            *in_use = true;
            fallback += 1;
        }
    }
    let unmatched = matches.iter().filter(|pair| pair.is_none()).count();
    let unused = used.iter().filter(|in_use| !**in_use).count();
    let mut warnings = Vec::new();
    if fallback > 0 {
        warnings.push(format!("Used alphanumeric index fallback for {fallback} video(s) without a unique filename/episode match. Verify the subtitle ordering before a full batch."));
    }
    if ambiguous > 0 {
        warnings.push(format!("{ambiguous} video(s) had ambiguous or competing subtitle matches; only unclaimed same-index subtitles were eligible for fallback."));
    }
    if videos.len() != subtitles.len() && (unmatched > 0 || unused > 0) {
        warnings.push(format!("Subtitle file-count mismatch: {} videos, {} subtitles; {unmatched} unpaired video(s), {unused} unused subtitle(s). Episode matching and strict index fallback could not resolve all orphans. Unpaired videos will encode without external subtitles.", videos.len(), subtitles.len()));
    } else if unmatched > 0 && !subtitles.is_empty() {
        warnings.push(format!("{unmatched} video(s) could not be paired without reusing a subtitle reserved by a confident match; they will encode without external subtitles."));
    }
    SubtitlePairing { matches, warnings }
}

#[cfg(test)]
fn match_subtitle(video: &Path, subtitles: &[PathBuf]) -> Result<Option<PathBuf>, String> {
    SubtitleIndex::new(subtitles)
        .candidate(video)
        .map(|position| position.and_then(|position| subtitles.get(position).cloned()))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn paths(names: &[&str]) -> Vec<PathBuf> {
        names.iter().map(PathBuf::from).collect()
    }
    #[test]
    fn episode_tokens_match_across_names_without_matching_resolution_or_substrings() {
        let videos = paths(&[
            "[Subs] Keroro 01 (1080p).mkv",
            "[Subs] Keroro 10 (1080p).mkv",
        ]);
        let subtitles = paths(&[
            "Different release Ep101.ass",
            "Other title - 10.srt",
            "Fansub episode 01.vtt",
        ]);
        let pairing = pair_subtitles(&videos, &subtitles);
        assert_eq!(
            pairing.matches,
            vec![Some(subtitles[2].clone()), Some(subtitles[1].clone())]
        );
        assert_eq!(episode_number("Unlabelled 01 02"), None);
        assert_eq!(episode_number("Movie 2026 1080"), None);
        assert!(
            pairing
                .warnings
                .iter()
                .any(|warning| warning.contains("2 videos, 3 subtitles"))
        );
    }
    #[test]
    fn fallback_uses_natural_sorted_indices_and_restores_caller_order() {
        let videos = paths(&["Film10.mkv", "Film2.mkv", "Film1.mkv"]);
        let subtitles = paths(&["Track C.ass", "Track A.srt", "Track B.vtt"]);
        let pairing = pair_subtitles(&videos, &subtitles);
        assert_eq!(
            pairing.matches,
            vec![
                Some(subtitles[0].clone()),
                Some(subtitles[2].clone()),
                Some(subtitles[1].clone())
            ]
        );
        assert!(
            pairing
                .warnings
                .iter()
                .any(|warning| warning.contains("index fallback for 3"))
        );
    }
    #[test]
    fn fallback_never_steals_a_confident_match_or_shifts_remaining_indices() {
        let videos = paths(&["Unknown.mkv", "Known E02.mkv"]);
        let subtitles = paths(&["A unnumbered.ass", "Z episode02.srt"]);
        let pairing = pair_subtitles(&videos, &subtitles);
        assert_eq!(pairing.matches, vec![None, Some(subtitles[1].clone())]);
        assert!(
            pairing
                .warnings
                .iter()
                .any(|warning| warning.contains("without reusing"))
        );
    }
    #[test]
    fn count_mismatch_warns_and_leaves_video_only_orphans() {
        let videos = paths(&["Unnumbered B.mkv", "Unnumbered A.mkv"]);
        let subtitles = paths(&["Subtitle.ass"]);
        let pairing = pair_subtitles(&videos, &subtitles);
        assert_eq!(pairing.matches, vec![None, Some(subtitles[0].clone())]);
        assert!(
            pairing
                .warnings
                .iter()
                .any(|warning| warning.contains("2 videos, 1 subtitles"))
        );
        assert!(
            pair_subtitles(&videos, &[])
                .warnings
                .iter()
                .any(|warning| warning.contains("0 subtitles"))
        );
    }
    #[test]
    fn competing_primary_matches_never_reuse_the_same_subtitle() {
        let videos = paths(&["A E01.mkv", "B E01.mkv"]);
        let subtitles = paths(&["Other episode01.srt"]);
        let pairing = pair_subtitles(&videos, &subtitles);
        assert_eq!(
            pairing.matches.iter().filter(|pair| pair.is_some()).count(),
            1
        );
        assert!(
            pairing
                .warnings
                .iter()
                .any(|warning| warning.contains("competing"))
        );
    }
    #[test]
    fn matching_does_not_confuse_seasons_resolutions_or_ambiguous_files() {
        assert_eq!(
            episode_number("[Group] Keroro - 03 [1080p]"),
            Some(Episode {
                season: None,
                number: 3
            })
        );
        assert_eq!(
            episode_number("Keroro S02E03"),
            Some(Episode {
                season: Some(2),
                number: 3
            })
        );
        assert_eq!(episode_number("Movie 2026 1080"), None);
        let files = vec![
            PathBuf::from("Other S01E03.srt"),
            PathBuf::from("Other S02E03.ass"),
        ];
        assert_eq!(
            match_subtitle(Path::new("Keroro S02E03.mkv"), &files).expect("match"),
            Some(files[1].clone())
        );
        assert!(match_subtitle(Path::new("Keroro E03.mkv"), &files).is_err());
        let exact = vec![
            PathBuf::from("Keroro E03.ass"),
            PathBuf::from("Other E03.srt"),
        ];
        assert_eq!(
            match_subtitle(Path::new("Keroro E03.mkv"), &exact).expect("exact"),
            Some(exact[0].clone())
        );
    }
    #[test]
    fn titles_and_natural_order_produce_safe_sequential_names() {
        assert_eq!(
            clean_title("  Sgt: Keroro / ").expect("title"),
            "Sgt Keroro"
        );
        assert_eq!(
            output_name(Path::new("Episode10.mkv"), "Sgt Keroro", 1).expect("name"),
            OsString::from("Sgt Keroro - Ep01 [Wii].mp4")
        );
        assert_eq!(
            output_name(Path::new("Episode10.mkv"), "", 1).expect("name"),
            OsString::from("Episode10_Wii.mp4")
        );
        assert_eq!(
            compare_videos(Path::new("Ep2.mkv"), Path::new("Ep10.mkv")),
            Ordering::Less
        );
    }
}
```

## src-tauri/crates/encoder-core/src/offline.rs

```rust
use std::path::Path;
pub fn local_path(path: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::path::{Component, Prefix};
        let absolute = std::path::absolute(path).map_err(|e| e.to_string())?;
        let drive = match absolute.components().next() {
            Some(Component::Prefix(prefix)) => match prefix.kind() {
                Prefix::Disk(drive) | Prefix::VerbatimDisk(drive) => drive,
                _ => {
                    return Err(
                        "OFFLINE_PATH_REQUIRED: UNC shares and device namespaces are disabled."
                            .into(),
                    );
                }
            },
            _ => return Err("A local drive path is required.".into()),
        };
        let root = [u16::from(drive), 58, 92, 0];
        // SAFETY: fixed terminated drive-root buffer. DRIVE_REMOTE is 4.
        if unsafe { windows_sys::Win32::Storage::FileSystem::GetDriveTypeW(root.as_ptr()) } == 4 {
            return Err("OFFLINE_PATH_REQUIRED: Mapped network drives are disabled.".into());
        }
    }
    #[cfg(not(windows))]
    {
        let _ = path;
    }
    Ok(())
}
#[cfg(all(test, windows))]
mod tests {
    use super::*;
    #[test]
    fn explicit_network_paths_are_rejected_without_opening_them() {
        assert!(local_path(Path::new(r"\\server\share\video")).is_err());
        assert!(local_path(Path::new(r"\\?\UNC\server\share\video")).is_err());
        assert!(local_path(Path::new(r"\\.\pipe\unknown")).is_err());
    }
}
```

## src-tauri/crates/encoder-core/src/output.rs

```rust
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn existing(path: &Path) -> Result<bool, String> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(format!("Cannot inspect output {}: {error}", path.display())),
        Ok(metadata) => {
            crate::artifacts::no_links(path)?;
            if !metadata.is_file() {
                return Err("Existing output is not a regular file; it was preserved.".into());
            }
            Ok(true)
        }
    }
}

pub fn partial_path(output: &Path) -> Result<PathBuf, String> {
    let mut name = output
        .file_name()
        .ok_or("Missing output filename.")?
        .to_os_string();
    name.push(".part");
    Ok(output.with_file_name(name))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    #[ignore = "Requires KERORO_TEST_FFMPEG pointing to the bundled FFmpeg executable"]
    async fn ffmpeg_writes_mp4_part_and_only_rename_publishes_the_final() {
        use std::process::Stdio;
        let binary = std::env::var_os("KERORO_TEST_FFMPEG").expect("FFmpeg");
        let root = tempfile::tempdir().expect("fixture");
        let final_path = root.path().join("Ep01.mp4");
        let part = partial_path(&final_path).expect("part path");
        let result = tokio::process::Command::new(&binary)
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-f",
                "lavfi",
                "-i",
                "color=c=black:s=64x48:r=10:d=1",
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
                "-f",
                "mp4",
            ])
            .arg(&part)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .output()
            .await
            .expect("encode part");
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(!final_path.exists());
        crate::atomic_file::rename(&part, &final_path, false).expect("finalize");
        assert!(!part.exists() && existing(&final_path).expect("final"));
        let decode = tokio::process::Command::new(binary)
            .args(["-hide_banner", "-loglevel", "error", "-i"])
            .arg(&final_path)
            .args(["-f", "null", "-"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .output()
            .await
            .expect("decode final");
        assert!(
            decode.status.success(),
            "{}",
            String::from_utf8_lossy(&decode.stderr)
        );
    }
    #[test]
    fn existing_finals_are_preserved_and_parts_are_not_mistaken_for_finals() {
        let root = tempfile::tempdir().expect("fixture");
        let final_path = root.path().join("Episode01.mp4");
        let part = partial_path(&final_path).expect("part path");
        assert_eq!(part.file_name().expect("filename"), "Episode01.mp4.part");
        fs::write(&part, b"partial").expect("part");
        assert!(!existing(&final_path).expect("missing final"));
        fs::write(&final_path, b"preserve original").expect("final");
        assert!(existing(&final_path).expect("skip final"));
        assert_eq!(fs::read(&final_path).expect("read"), b"preserve original");
        let folder = root.path().join("directory.mp4");
        fs::create_dir(&folder).expect("directory");
        assert!(existing(&folder).is_err());
    }
}
```

## src-tauri/crates/encoder-core/src/power.rs

```rust
#[cfg(windows)]
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};

/// OS power-request ownership is not thread-affine; safe to move between workers.
pub struct WakeLock {
    #[cfg(windows)]
    handle: OwnedHandle,
}
impl WakeLock {
    #[cfg(windows)]
    pub fn acquire() -> Result<Self, String> {
        use windows_sys::Win32::{
            Foundation::INVALID_HANDLE_VALUE,
            System::{
                Power::{PowerCreateRequest, PowerRequestSystemRequired, PowerSetRequest},
                Threading::{REASON_CONTEXT, REASON_CONTEXT_0},
            },
        };
        let mut reason = "Keroro Wii Encoder is processing a media task."
            .encode_utf16()
            .chain(Some(0))
            .collect::<Vec<_>>();
        let context = REASON_CONTEXT {
            Version: 0,
            Flags: 1,
            Reason: REASON_CONTEXT_0 {
                SimpleReasonString: reason.as_mut_ptr(),
            },
        };
        // SAFETY: correctly tagged simple-string union and live NUL-terminated
        // UTF-16 buffer. PowerCreateRequest copies the supplied reason context.
        let raw = unsafe { PowerCreateRequest(&context) };
        if raw == INVALID_HANDLE_VALUE || raw.is_null() {
            return Err(format!(
                "Cannot create sleep-prevention request: {}",
                std::io::Error::last_os_error()
            ));
        }
        // SAFETY: newly created unique valid owned kernel handle, closed by RAII.
        let handle = unsafe { OwnedHandle::from_raw_handle(raw) };
        // SAFETY: valid owned request handle; does not request a display lock.
        if unsafe { PowerSetRequest(handle.as_raw_handle(), PowerRequestSystemRequired) } == 0 {
            return Err(format!(
                "Cannot prevent idle sleep: {}",
                std::io::Error::last_os_error()
            ));
        }
        Ok(Self { handle })
    }
    #[cfg(not(windows))]
    pub fn acquire() -> Result<Self, String> {
        Err("Native sleep prevention is implemented for Windows only.".into())
    }
}
#[cfg(windows)]
impl Drop for WakeLock {
    fn drop(&mut self) {
        use windows_sys::Win32::System::Power::{PowerClearRequest, PowerRequestSystemRequired};
        // SAFETY: handle remains valid until OwnedHandle drops after this method.
        if unsafe { PowerClearRequest(self.handle.as_raw_handle(), PowerRequestSystemRequired) }
            == 0
        {
            log::error!(
                "Cannot clear idle-sleep request: {}",
                std::io::Error::last_os_error()
            );
        }
        // OwnedHandle then closes the power request even if clearing failed.
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetProcessHandleCount};
    fn handles() -> u32 {
        let mut count = 0;
        // SAFETY: valid process pseudo-handle and writable u32 output.
        assert_ne!(
            unsafe { GetProcessHandleCount(GetCurrentProcess(), &mut count) },
            0
        );
        count
    }
    fn exercise() {
        let lock = WakeLock::acquire().expect("native power request");
        std::thread::spawn(move || drop(lock))
            .join()
            .expect("drop on another thread");
    }
    #[test]
    #[ignore = "Process-wide handle measurement; run alone or with --test-threads=1"]
    fn power_request_moves_across_threads_without_leaking_handles() {
        exercise(); // Warm up OS/Rust per-process initialization.
        let before = handles();
        for _ in 0..10 {
            exercise();
        }
        let after = handles();
        assert!(
            after <= before,
            "Power-request handles grew from {before} to {after}"
        );
    }
}
```

## src-tauri/crates/encoder-core/src/preflight.rs

```rust
use std::{
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};
use tokio::{fs, io::AsyncWriteExt};

pub const MIN_FREE_BYTES: u64 = 64 * 1024 * 1024;

pub fn access_error(action: &str, error: std::io::Error) -> String {
    if error.kind() == std::io::ErrorKind::PermissionDenied {
        format!(
            "OUTPUT_PERMISSION_DENIED: {action}: {error}. Choose a writable folder or review Windows Controlled Folder Access permissions."
        )
    } else {
        format!("{action}: {error}")
    }
}

/// A unique zero-byte canary. Never truncate/delete a pre-existing user's file.
pub async fn canary(path: &Path) -> Result<(), String> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let probe = path.join(format!(
        ".keroro_access_test-{}-{stamp}-{}.tmp",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
        .await
        .map_err(|error| access_error("Cannot create output canary", error))?;
    let journal = crate::artifacts::record(&probe).await;
    drop(file); // No write calls: exactly zero bytes.
    let cleanup = fs::remove_file(&probe)
        .await
        .map_err(|error| access_error("Cannot remove output canary", error));
    journal?;
    cleanup?;
    crate::artifacts::forget(&probe).await
}

pub fn validate_space(available: u64) -> Result<(), String> {
    if available < MIN_FREE_BYTES {
        Err(format!(
            "Output drive has only {} MiB available; at least 64 MiB of free reserve is required. Free space and retry.",
            available / 1024 / 1024
        ))
    } else {
        Ok(())
    }
}

#[cfg(windows)]
fn available(path: &Path) -> Result<Option<u64>, String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    let mut wide = path.as_os_str().encode_wide().collect::<Vec<_>>();
    if wide.contains(&0) {
        return Err("Output path contains a NUL character.".into());
    }
    if wide.last() != Some(&92) {
        wide.push(92);
    } // UNC directories also require a trailing slash.
    wide.push(0);
    let mut bytes = 0u64;
    // SAFETY: NUL-terminated live UTF-16 path, valid u64 output, optional unused
    // outputs are null. Use caller-available bytes so disk quotas are respected.
    let success = unsafe {
        GetDiskFreeSpaceExW(
            wide.as_ptr(),
            &mut bytes,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if success == 0 {
        return Err(format!(
            "Cannot query output free space: {}",
            std::io::Error::last_os_error()
        ));
    }
    Ok(Some(bytes))
}

#[cfg(not(windows))]
fn available(_: &Path) -> Result<Option<u64>, String> {
    Ok(None)
}

/// A free-space reserve is a guardrail, not a size prediction for CRF encoding.
pub async fn space(path: &Path) -> Result<Option<String>, String> {
    let owned = path.to_path_buf();
    let query = tokio::task::spawn_blocking(move || available(&owned)).await;
    match query {
        Ok(Ok(Some(bytes))) => {
            validate_space(bytes)?;
            Ok(None)
        }
        Ok(Ok(None)) => Ok(Some(
            "Free-space pre-check is unavailable on this platform.".into(),
        )),
        Ok(Err(error)) => Ok(Some(format!("{error}; free-space pre-check unavailable."))),
        Err(error) => Ok(Some(format!(
            "Free-space query could not complete: {error}"
        ))),
    }
}

pub async fn output(path: &Path) -> Result<Option<String>, String> {
    if !fs::metadata(path)
        .await
        .map_err(|e| access_error("Cannot inspect output folder", e))?
        .is_dir()
    {
        return Err("Output path must be a directory.".into());
    }
    canary(path).await?;
    static NEXT: AtomicU64 = AtomicU64::new(0);
    // Unique create_new prevents overwriting any pre-existing file or symlink.
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let probe = path.join(format!(
        ".keroro-write-check-{}-{stamp}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
        .await
        .map_err(|e| access_error("Output folder is not writable", e))?;
    let journal = crate::artifacts::record(&probe).await;
    let check = async {
        file.write_all(b"Keroro output write validation\n").await?;
        file.flush().await?;
        file.sync_all().await
    }
    .await;
    drop(file);
    let cleanup = fs::remove_file(&probe).await;
    if let Err(error) = cleanup {
        return Err(format!(
            "Cannot remove output write-check file {}: {error}",
            probe.display()
        ));
    }
    journal?;
    crate::artifacts::forget(&probe).await?;
    check.map_err(|e| access_error("Output write validation failed", e))?;
    space(path).await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn low_space_is_rejected_without_filling_a_real_drive() {
        assert!(validate_space(0).is_err());
        assert!(validate_space(MIN_FREE_BYTES - 1).is_err());
        assert!(validate_space(MIN_FREE_BYTES).is_ok());
    }
    #[test]
    fn permission_errors_have_an_actionable_stable_code() {
        let error = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "blocked by OS");
        assert!(
            access_error("Cannot create canary", error).starts_with("OUTPUT_PERMISSION_DENIED:")
        );
    }
    #[tokio::test]
    async fn canary_leaves_no_files_and_preserves_existing_names() {
        let root = tempfile::tempdir().expect("fixture");
        let existing = root.path().join(".keroro_access_test.tmp");
        fs::write(&existing, "keep me").await.expect("existing");
        canary(root.path()).await.expect("canary");
        assert_eq!(
            fs::read_to_string(&existing).await.expect("preserved"),
            "keep me"
        );
        let mut entries = fs::read_dir(root.path()).await.expect("directory");
        let mut count = 0;
        while entries.next_entry().await.expect("entry").is_some() {
            count += 1;
        }
        assert_eq!(count, 1);
    }
}
```

## src-tauri/crates/encoder-core/src/process_state.rs

```rust
use std::{
    process::ExitStatus,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{io::AsyncWriteExt, process::Child};

/// One authoritative FFmpeg/ffprobe child slot, shared with the application.
/// Never hold this synchronous mutex across an await.
#[derive(Default)]
pub struct ActiveProcess(pub Mutex<Option<Child>>);

pub(crate) struct ProcessLease {
    state: Arc<ActiveProcess>,
    subtitle: Option<crate::subtitles::StagedSubtitle>,
}

impl ActiveProcess {
    /// Wait for any unwind/cancellation reaper before releasing the job reservation.
    pub async fn wait_until_idle(&self) {
        loop {
            let occupied = match self.0.lock() {
                Ok(slot) => slot.is_some(),
                Err(poisoned) => poisoned.into_inner().is_some(),
            };
            if !occupied {
                return;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }
    pub(crate) fn track(self: &Arc<Self>, mut child: Child) -> Result<ProcessLease, String> {
        if child.try_wait().map_err(|e| e.to_string())?.is_none() {
            if let Err(error) = crate::job_fence::assign(&child) {
                // Very short probes can exit in the spawn/assignment interval.
                if child.try_wait().map_err(|e| e.to_string())?.is_none() {
                    return Err(error);
                }
            }
        }
        let mut slot = self
            .0
            .lock()
            .map_err(|e| format!("Process state unavailable: {e}"))?;
        if slot.is_some() {
            return Err("Another media process is already active.".into());
        }
        *slot = Some(child);
        Ok(ProcessLease {
            state: self.clone(),
            subtitle: None,
        })
    }
}

impl ProcessLease {
    pub(crate) fn hold_subtitle(&mut self, subtitle: Option<crate::subtitles::StagedSubtitle>) {
        self.subtitle = subtitle;
    }
    pub(crate) async fn wait(&self) -> Result<ExitStatus, String> {
        loop {
            let status = {
                let mut slot = self.state.0.lock().map_err(|e| e.to_string())?;
                slot.as_mut()
                    .ok_or("Media process handle is missing.")?
                    .try_wait()
                    .map_err(|e| format!("Cannot wait for media process: {e}"))?
            };
            if let Some(status) = status {
                return Ok(status);
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }

    pub(crate) async fn kill_and_wait(&self) -> Result<(), String> {
        {
            let mut slot = self.state.0.lock().map_err(|e| e.to_string())?;
            let child = slot.as_mut().ok_or("Media process handle is missing.")?;
            if child.try_wait().map_err(|e| e.to_string())?.is_none() {
                child
                    .start_kill()
                    .map_err(|e| format!("Cannot terminate media process: {e}"))?;
            }
        }
        self.wait().await.map(|_| ())
    }

    pub(crate) async fn stop(&self) -> Result<(), String> {
        let stdin = {
            let mut slot = self.state.0.lock().map_err(|e| e.to_string())?;
            let child = slot.as_mut().ok_or("Media process handle is missing.")?;
            if child.try_wait().map_err(|e| e.to_string())?.is_some() {
                return Ok(());
            }
            child.stdin.take()
        };
        // A stalled stdin must not prevent the bounded graceful shutdown.
        if let Some(mut stdin) = stdin {
            let _ = tokio::time::timeout(Duration::from_millis(250), stdin.write_all(b"q\n")).await;
        }
        if let Ok(status) = tokio::time::timeout(Duration::from_secs(2), self.wait()).await {
            return status.map(|_| ());
        }
        self.kill_and_wait().await
    }
}

impl Drop for ProcessLease {
    fn drop(&mut self) {
        // An unwinding worker must not delete a subtitle while FFmpeg still reads it.
        let subtitle = self.subtitle.take();
        let mut slot = match self.state.0.lock() {
            Ok(slot) => slot,
            // Cleanup must still run after a panic poisoned the slot.
            Err(poisoned) => poisoned.into_inner(),
        };
        let Some(child) = slot.as_mut() else {
            return;
        };
        match child.try_wait() {
            Ok(Some(_)) => {
                slot.take();
                drop(slot);
                if let Some(subtitle) = subtitle {
                    if let Err(error) = subtitle.close() {
                        log::warn!("{error}");
                    }
                }
                return;
            } // Already reaped.
            Ok(None) | Err(_) => {
                if let Err(error) = child.start_kill() {
                    log::error!("Cannot stop dropped media process: {error}");
                }
            }
        }
        // Dropping a worker future or unwinding must never detach a live child.
        // Commands also set kill_on_drop(true) as a fallback without a runtime.
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            let state = self.state.clone();
            drop(slot);
            runtime.spawn(async move {
                loop {
                    let done = {
                        let mut slot = match state.0.lock() {
                            Ok(slot) => slot,
                            Err(poisoned) => poisoned.into_inner(),
                        };
                        match slot.as_mut().map(|child| child.try_wait()) {
                            Some(Ok(None)) => false,
                            Some(Err(error)) => {
                                log::error!("Cannot inspect dropped media process: {error}");
                                // Retain ownership and retry termination; never declare a live child idle.
                                if let Some(child) = slot.as_mut() {
                                    let _ = child.start_kill();
                                }
                                false
                            }
                            Some(Ok(Some(_))) | None => true,
                        }
                    };
                    if done {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(25)).await;
                }
                if let Some(subtitle) = subtitle {
                    if let Err(error) = subtitle.close() {
                        log::error!("{error}");
                    }
                }
                let mut slot = match state.0.lock() {
                    Ok(slot) => slot,
                    Err(poisoned) => poisoned.into_inner(),
                };
                slot.take();
            });
        } else {
            // Runtime teardown: Child's kill_on_drop is the final fallback.
            slot.take();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Stdio;
    use tokio::sync::oneshot;

    fn fixture(live: bool) -> Child {
        let mut command = tokio::process::Command::new(
            std::env::var_os("KERORO_TEST_FFMPEG").expect("FFmpeg fixture path"),
        );
        command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        #[cfg(windows)]
        command.creation_flags(0x0800_0000);
        if live {
            command.args([
                "-re",
                "-f",
                "lavfi",
                "-i",
                "color=size=64x64:rate=10",
                "-t",
                "60",
                "-f",
                "null",
                "-",
            ]);
        } else {
            command.arg("-version");
        }
        command.spawn().expect("spawn fixture")
    }

    #[tokio::test]
    #[ignore = "Requires bundled FFmpeg"]
    async fn normal_exit_reaps_child_and_clears_slot() {
        let state = Arc::new(ActiveProcess::default());
        let lease = state.track(fixture(false)).expect("track");
        assert!(
            state
                .0
                .try_lock()
                .expect("mutex is not held across awaits")
                .is_some()
        );
        assert!(lease.wait().await.expect("wait").success());
        drop(lease);
        assert!(state.0.lock().expect("state").is_none());
    }

    #[tokio::test]
    #[ignore = "Requires bundled FFmpeg"]
    async fn early_error_stops_reaps_and_clears_child() {
        let state = Arc::new(ActiveProcess::default());
        let result: Result<(), String> = async {
            let _lease = state.track(fixture(true))?;
            Err("Injected pipe/read/setup failure".into())
        }
        .await;
        assert!(result.is_err());
        tokio::time::timeout(Duration::from_secs(3), state.wait_until_idle())
            .await
            .expect("error cleanup reaped child");
        assert!(state.0.lock().expect("state").is_none());
    }

    async fn dropped_worker_is_reaped(panic_worker: bool) {
        let state = Arc::new(ActiveProcess::default());
        let worker_state = state.clone();
        let (registered, registration) = oneshot::channel();
        let (trigger, proceed) = oneshot::channel::<()>();
        let worker = tokio::spawn(async move {
            let mut lease = worker_state.track(fixture(true)).expect("track");
            let staged = crate::subtitles::stage("subtitle fixture".into(), "srt")
                .await
                .expect("stage");
            let paths = (
                staged.path().to_path_buf(),
                staged.directory().to_path_buf(),
            );
            lease.hold_subtitle(Some(staged));
            registered.send(paths).expect("registration receiver");
            let _ = proceed.await;
            if panic_worker {
                panic!("Injected worker panic");
            }
            std::future::pending::<()>().await;
        });
        let (subtitle_path, subtitle_directory) = registration.await.expect("registered");
        assert!(state.0.try_lock().expect("short mutex scope").is_some());
        if panic_worker {
            trigger.send(()).expect("trigger panic");
        } else {
            worker.abort();
        }
        let failure = worker.await.expect_err("worker interrupted");
        assert_eq!(failure.is_panic(), panic_worker);
        tokio::time::timeout(Duration::from_secs(3), state.wait_until_idle())
            .await
            .expect("child reaped promptly");
        assert!(
            !subtitle_path.exists() && !subtitle_directory.exists(),
            "Subtitle resources must outlive FFmpeg, then be removed"
        );
        assert!(state.0.lock().expect("state").is_none());
        // The slot remains reusable after unwinding or dropping a future.
        let lease = state.track(fixture(false)).expect("reuse slot");
        assert!(lease.wait().await.expect("wait").success());
        drop(lease);
        assert!(state.0.lock().expect("state").is_none());
    }

    #[tokio::test]
    #[ignore = "Requires bundled FFmpeg"]
    async fn worker_panic_stops_reaps_and_clears_child() {
        dropped_worker_is_reaped(true).await;
    }

    #[tokio::test]
    #[ignore = "Requires bundled FFmpeg"]
    async fn worker_abort_stops_reaps_and_clears_child() {
        dropped_worker_is_reaped(false).await;
    }
}
```

## src-tauri/crates/encoder-core/src/progress.rs

```rust
/// Timing updates require a changed integer percentage or ETA second.
#[derive(Default)]
pub struct ProgressGate {
    last: Option<Option<u8>>,
    last_eta: Option<Option<u64>>,
}
impl ProgressGate {
    pub fn should_emit(&mut self, percentage: Option<f64>) -> bool {
        let integer = percentage
            .filter(|value| value.is_finite())
            .map(|value| value.clamp(0.0, 100.0).floor() as u8);
        if self.last == Some(integer) {
            return false;
        }
        self.last = Some(integer);
        true
    }
    pub fn should_emit_eta(&mut self, seconds: Option<f64>) -> bool {
        let second = seconds
            .filter(|value| value.is_finite() && *value >= 0.0 && *value < u64::MAX as f64)
            .map(|value| value.ceil() as u64);
        if self.last_eta == Some(second) {
            return false;
        }
        self.last_eta = Some(second);
        true
    }
}

/// FFmpeg's structured out_time timestamp; unknown/malformed values stay unknown.
pub fn timestamp_seconds(value: &str) -> Option<f64> {
    let mut fields = value.split(':');
    let hours = fields.next()?.parse::<u64>().ok()?;
    let minutes = fields.next()?.parse::<u8>().ok()?;
    let seconds = fields.next()?.parse::<f64>().ok()?;
    if fields.next().is_some()
        || minutes >= 60
        || !seconds.is_finite()
        || !(0.0..60.0).contains(&seconds)
    {
        return None;
    }
    let total = hours as f64 * 3600.0 + f64::from(minutes) * 60.0 + seconds;
    total.is_finite().then_some(total)
}

pub fn thread_budget(logical_cpus: usize) -> usize {
    logical_cpus.saturating_sub(2).max(1)
}

pub fn speed_factor(value: &str) -> Option<f64> {
    let value = value.trim();
    value
        .strip_suffix('x')
        .unwrap_or(value)
        .parse::<f64>()
        .ok()
        .filter(|speed| speed.is_finite() && *speed > 0.0)
}

pub fn eta_seconds(
    position: Option<f64>,
    duration: Option<f64>,
    speed: Option<f64>,
) -> Option<f64> {
    let (position, duration, speed) = (position?, duration?, speed?);
    if !position.is_finite()
        || position < 0.0
        || !duration.is_finite()
        || duration <= 0.0
        || !speed.is_finite()
        || speed <= 0.0
    {
        return None;
    }
    let eta = (duration - position).max(0.0) / speed;
    eta.is_finite().then_some(eta)
}

pub fn format_eta(seconds: Option<f64>) -> String {
    let Some(seconds) = seconds
        .filter(|seconds| seconds.is_finite() && *seconds >= 0.0 && *seconds < u64::MAX as f64)
    else {
        return "--".into();
    };
    let seconds = seconds.ceil() as u64;
    if seconds >= 3600 {
        format!(
            "{:02}h {:02}m {:02}s",
            seconds / 3600,
            seconds / 60 % 60,
            seconds % 60
        )
    } else {
        format!("{:02}m {:02}s", seconds / 60, seconds % 60)
    }
}

#[derive(Clone, Default)]
pub struct StderrProgress {
    pub duration: Option<f64>,
    pub position: Option<f64>,
    pub speed: Option<f64>,
    pub has_diagnostics: bool,
}
impl StderrProgress {
    pub fn update(&mut self, line: &str) -> bool {
        let mut changed = false;
        if ["[warning]", "[error]", "[fatal]"]
            .iter()
            .any(|level| line.contains(level))
        {
            self.has_diagnostics = true;
            log::warn!("FFmpeg diagnostic: {line}");
            changed = true;
        }
        if let Some((_, value)) = line.split_once("Duration:") {
            self.duration = value
                .split(',')
                .next()
                .and_then(|value| timestamp_seconds(value.trim()))
                .filter(|duration| *duration > 0.0);
            changed = true;
        }
        if let Some((_, value)) = line.split_once("time=") {
            self.position = value.split_whitespace().next().and_then(timestamp_seconds);
            changed = true;
        }
        if let Some((_, value)) = line.split_once("speed=") {
            self.speed = value.split_whitespace().next().and_then(speed_factor);
            changed = true;
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn eta_changes_are_visible_without_duplicate_subsecond_events() {
        let mut gate = ProgressGate::default();
        assert!(gate.should_emit_eta(None));
        assert!(!gate.should_emit_eta(None));
        assert!(gate.should_emit_eta(Some(135.8)));
        assert!(!gate.should_emit_eta(Some(135.2)));
        assert!(gate.should_emit_eta(Some(135.0)));
        assert!(!gate.should_emit_eta(Some(134.9)));
        assert!(gate.should_emit_eta(Some(0.0)));
    }
    #[test]
    fn stderr_duration_time_speed_and_eta_are_parsed_without_inventing_values() {
        let mut progress = StderrProgress::default();
        assert!(progress.update("[info] Duration: 00:04:30.00, start: 0.000, bitrate: 123"));
        assert!(progress.update("frame= 42 fps=30 time=00:02:15.00 bitrate=123kbits/s speed=1.0x"));
        assert_eq!(
            eta_seconds(progress.position, progress.duration, progress.speed),
            Some(135.0)
        );
        assert_eq!(format_eta(Some(135.0)), "02m 15s");
        assert_eq!(eta_seconds(Some(30.0), Some(90.0), Some(2.0)), Some(30.0));
        assert_eq!(format_eta(Some(3723.1)), "01h 02m 04s");
        progress.update("time=N/A speed=0x");
        assert!(eta_seconds(progress.position, progress.duration, progress.speed).is_none());
        assert_eq!(format_eta(None), "--");
        assert_eq!(speed_factor("NaNx"), None);
        assert_eq!(speed_factor("1xxx"), None);
        progress.update("[warning] A subtitle font was substituted");
        assert!(progress.has_diagnostics);
        assert_eq!(eta_seconds(Some(0.0), Some(f64::INFINITY), Some(1.0)), None);
    }
    #[test]
    fn integer_gate_limits_spam_and_unknown_duration_is_not_fabricated() {
        let mut gate = ProgressGate::default();
        assert!(gate.should_emit(None));
        assert!(!gate.should_emit(None));
        assert!(gate.should_emit(Some(0.01)));
        assert!(!gate.should_emit(Some(0.99)));
        assert!(gate.should_emit(Some(1.0)));
        assert!(!gate.should_emit(Some(1.9)));
        assert!(gate.should_emit(Some(99.9)));
        assert!(gate.should_emit(Some(100.0)));
        let mut gate = ProgressGate::default();
        let count = (0..10000)
            .filter(|sample| gate.should_emit(Some(f64::from(*sample) / 100.0)))
            .count();
        assert_eq!(count, 100);
    }
    #[test]
    fn timestamps_and_small_cpu_counts_are_safe() {
        assert_eq!(timestamp_seconds("01:02:03.456"), Some(3723.456));
        for value in [
            "N/A",
            "NaN",
            "00:60:00",
            "00:00:NaN",
            "00:00:01:02",
            "-1:00:00",
        ] {
            assert_eq!(timestamp_seconds(value), None);
        }
        for (cpus, expected) in [(0, 1), (1, 1), (2, 1), (3, 1), (4, 2), (16, 14)] {
            assert_eq!(thread_budget(cpus), expected);
        }
    }
}
```

## src-tauri/crates/encoder-core/src/session.rs

```rust
use crate::{BatchPlan, Item, Settings, TaskKind, artifacts};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct Stamp {
    length: u64,
    modified: u128,
}
fn stamp(path: &Path) -> Result<Stamp, String> {
    artifacts::no_links(path)?;
    let metadata =
        fs::metadata(path).map_err(|e| format!("Cannot inspect {}: {e}", path.display()))?;
    if !metadata.is_file() {
        return Err("Expected a regular media file.".into());
    }
    let modified = metadata
        .modified()
        .map_err(|e| e.to_string())?
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    Ok(Stamp {
        length: metadata.len(),
        modified,
    })
}
fn digest(path: &Path, cancelled: &dyn Fn() -> bool) -> Result<[u8; 32], String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    let before = stamp(path)?;
    let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        if cancelled() {
            return Err("Encoding cancelled.".into());
        }
        let count = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    if stamp(path)? != before {
        return Err("Media changed while computing its recovery checksum.".into());
    }
    Ok(hash.finalize().into())
}
#[derive(Clone, Serialize, Deserialize)]
struct Completed {
    digest: [u8; 32],
    stamp: Stamp,
}
#[derive(Clone, Serialize, Deserialize)]
struct Intent {
    index: usize,
    digest: [u8; 32],
    stamp: Stamp,
}
#[derive(Clone, Serialize, Deserialize)]
struct Checkpoint {
    version: u8,
    job_id: String,
    settings: Settings,
    items: Vec<Item>,
    inputs: Vec<Stamp>,
    completed: BTreeMap<usize, Completed>,
    #[serde(default)]
    skipped: BTreeMap<usize, Stamp>,
    intent: Option<Intent>,
}
struct Contents {
    checkpoint: Option<Checkpoint>,
    fault: Option<String>,
}
pub struct SessionStore {
    path: PathBuf,
    contents: Mutex<Contents>,
    _lock: fs::File,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryInfo {
    pub id: String,
    pub next_episode: Option<usize>,
    pub total_files: usize,
    pub completed_files: usize,
    pub can_resume: bool,
    pub problem: Option<String>,
    pub settings: Option<Settings>,
}

impl SessionStore {
    pub fn open(directory: &Path) -> Result<Arc<Self>, String> {
        crate::offline::local_path(directory)?;
        fs::create_dir_all(directory).map_err(|e| e.to_string())?;
        let directory = fs::canonicalize(directory).map_err(|e| e.to_string())?;
        artifacts::no_links(&directory)?;
        let lock_path = directory.join("runtime.lock");
        if lock_path.exists() {
            artifacts::no_links(&lock_path)?;
        }
        let lock = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(lock_path)
            .map_err(|e| e.to_string())?;
        lock.try_lock()
            .map_err(|e| format!("Another Keroro instance owns recovery state: {e}"))?;
        let path = directory.join("batch_state.json");
        let load = || -> Result<Option<Checkpoint>, String> {
            match fs::metadata(&path) {
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
                Err(e) => return Err(e.to_string()),
                Ok(meta) if meta.len() > 8 * 1024 * 1024 => {
                    return Err("Checkpoint exceeds its 8 MiB limit.".into());
                }
                _ => {}
            }
            artifacts::no_links(&path)?;
            let checkpoint: Checkpoint =
                serde_json::from_slice(&fs::read(&path).map_err(|e| e.to_string())?)
                    .map_err(|e| format!("Invalid batch checkpoint: {e}"))?;
            Self::validate(&checkpoint)?;
            Ok(Some(checkpoint))
        };
        let (checkpoint, fault) = match load() {
            Ok(state) => (state, None),
            Err(error) => (None, Some(error)),
        };
        let store = Arc::new(Self {
            path,
            contents: Mutex::new(Contents { checkpoint, fault }),
            _lock: lock,
        });
        Ok(store)
    }
    fn reconcile(checkpoint: &mut Checkpoint, path: &Path) -> Result<(), String> {
        let Some(intent) = checkpoint.intent.clone() else {
            return Ok(());
        };
        let item = checkpoint
            .items
            .get(intent.index)
            .ok_or("Invalid commit index.")?;
        if digest(&item.output, &|| false).as_ref() == Ok(&intent.digest)
            && stamp(&item.output).is_ok_and(|current| current.length == intent.stamp.length)
        {
            let mut candidate = checkpoint.clone();
            candidate.completed.insert(
                intent.index,
                Completed {
                    digest: intent.digest,
                    stamp: intent.stamp,
                },
            );
            candidate.intent = None;
            crate::atomic_file::json(path, &candidate)?;
            *checkpoint = candidate;
        }
        Ok(())
    }
    fn validate(checkpoint: &Checkpoint) -> Result<(), String> {
        if checkpoint.version != 1
            || checkpoint.items.is_empty()
            || checkpoint.items.len() > 50_000
            || checkpoint.inputs.len() != checkpoint.items.len()
            || checkpoint.job_id.is_empty()
            || checkpoint.job_id.len() > 128
            || !checkpoint
                .job_id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_'))
        {
            return Err("Unsupported or inconsistent recovery checkpoint.".into());
        }
        let input = Path::new(&checkpoint.settings.input_dir);
        let output = Path::new(&checkpoint.settings.output_dir);
        crate::offline::local_path(input)?;
        crate::offline::local_path(output)?;
        if !input.is_absolute() || !output.is_absolute() || input == output {
            return Err("Invalid checkpoint roots.".into());
        }
        let mut destinations = std::collections::HashSet::new();
        for (index, item) in checkpoint.items.iter().enumerate() {
            if item.input.parent() != Some(input)
                || item.output.parent() != Some(output)
                || !crate::is_video_path(&item.input)
            {
                return Err("Checkpoint file escaped its frozen queue roots.".into());
            }
            let expected = crate::media::output_name(
                &item.input,
                &checkpoint.settings.anime_title,
                index + 1,
            )?;
            let actual = item
                .output
                .file_name()
                .ok_or("Missing checkpoint output name.")?;
            // Earlier checkpoints keep their frozen E01 names; new batches use Ep01.
            let legacy = format!(
                "{} - E{:02} [Wii].mp4",
                checkpoint.settings.anime_title,
                index + 1
            );
            let legacy_matches = !checkpoint.settings.anime_title.is_empty()
                && actual == std::ffi::OsStr::new(&legacy);
            if (actual != expected.as_os_str() && !legacy_matches)
                || !destinations.insert(actual.to_string_lossy().to_lowercase())
            {
                return Err("Checkpoint output name differs from the original naming plan.".into());
            }
            if let Some(subtitle) = &item.subtitle {
                if item.subtitle_root.as_deref() != subtitle.parent()
                    || crate::subtitles::extension(subtitle).is_none()
                {
                    return Err("Invalid checkpoint subtitle path.".into());
                }
            }
            if item.subtitle_root.as_deref()
                != checkpoint
                    .settings
                    .subtitle_dir
                    .as_deref()
                    .filter(|path| !path.trim().is_empty())
                    .map(Path::new)
            {
                return Err("Checkpoint subtitle root differs from its saved settings.".into());
            }
        }
        if checkpoint
            .completed
            .keys()
            .chain(checkpoint.skipped.keys())
            .any(|index| *index >= checkpoint.items.len())
            || checkpoint
                .skipped
                .keys()
                .any(|index| checkpoint.completed.contains_key(index))
            || checkpoint.intent.as_ref().is_some_and(|intent| {
                intent.index >= checkpoint.items.len()
                    || checkpoint.skipped.contains_key(&intent.index)
            })
        {
            return Err("Checkpoint indices are invalid.".into());
        }
        Ok(())
    }
    pub fn begin(&self, plan: &BatchPlan) -> Result<(), String> {
        let mut contents = self.contents.lock().map_err(|e| e.to_string())?;
        if contents.checkpoint.is_some() || contents.fault.is_some() {
            return Err(
                "Resolve the previous recovery checkpoint before starting a new batch.".into(),
            );
        }
        let inputs = plan
            .items
            .iter()
            .map(|item| stamp(&item.input))
            .collect::<Result<Vec<_>, _>>()?;
        let mut skipped = BTreeMap::new();
        for (index, item) in plan.items.iter().enumerate() {
            match crate::output::existing(&item.output) {
                Ok(true) => {
                    skipped.insert(index, stamp(&item.output)?);
                }
                Ok(false) => {}
                Err(error) => log::warn!("Existing output will be checked per file: {error}"),
            }
        }
        let checkpoint = Checkpoint {
            version: 1,
            job_id: plan.job_id.clone(),
            settings: plan.settings.clone(),
            items: plan.items.clone(),
            inputs,
            completed: BTreeMap::new(),
            skipped,
            intent: None,
        };
        Self::validate(&checkpoint)?;
        crate::atomic_file::json(&self.path, &checkpoint)?;
        contents.checkpoint = Some(checkpoint);
        Ok(())
    }
    fn check_files(
        checkpoint: &Checkpoint,
        deep: bool,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<(), String> {
        for (index, item) in checkpoint.items.iter().enumerate() {
            if let Some(skipped) = checkpoint.skipped.get(&index) {
                if stamp(&item.output)? != *skipped {
                    return Err(format!(
                        "Previously skipped output changed: {}. It was preserved.",
                        item.output.display()
                    ));
                }
            } else if let Some(completed) = checkpoint.completed.get(&index) {
                if deep && digest(&item.output, cancelled)? != completed.digest {
                    return Err(format!(
                        "Completed output checksum changed: {}. It was preserved.",
                        item.output.display()
                    ));
                }
                if stamp(&item.output)?.length != completed.stamp.length {
                    return Err(format!(
                        "Completed output changed: {}. It was preserved; review it before resuming.",
                        item.output.display()
                    ));
                }
            } else {
                if crate::output::existing(&item.output)? {
                    if checkpoint
                        .intent
                        .as_ref()
                        .is_some_and(|intent| intent.index == index)
                    {
                        return Err(format!(
                            "Interrupted commit does not match its checksum: {}. It was preserved.",
                            item.output.display()
                        ));
                    }
                    continue;
                }
                if stamp(&item.input)?
                    != *checkpoint
                        .inputs
                        .get(index)
                        .ok_or("Missing source fingerprint.")?
                {
                    return Err(format!("Source changed: {}", item.input.display()));
                }
            }
        }
        Ok(())
    }
    pub fn info(&self) -> Result<Option<RecoveryInfo>, String> {
        let mut contents = self.contents.lock().map_err(|e| e.to_string())?;
        if let Some(problem) = &contents.fault {
            return Ok(Some(RecoveryInfo {
                id: "invalid".into(),
                next_episode: None,
                total_files: 0,
                completed_files: 0,
                can_resume: false,
                problem: Some(problem.clone()),
                settings: None,
            }));
        }
        let reconcile_problem = if let Some(state) = contents.checkpoint.as_mut() {
            Self::reconcile(state, &self.path).err()
        } else {
            None
        };
        let Some(state) = &contents.checkpoint else {
            return Ok(None);
        };
        let next_episode = (0..state.items.len())
            .find(|index| {
                !state.completed.contains_key(index) && !state.skipped.contains_key(index)
            })
            .map(|index| index + 1);
        let problem =
            reconcile_problem.or_else(|| Self::check_files(state, false, &|| false).err());
        Ok(Some(RecoveryInfo {
            id: state.job_id.clone(),
            next_episode,
            total_files: state.items.len(),
            completed_files: state.completed.len() + state.skipped.len(),
            can_resume: problem.is_none() && next_episode.is_some(),
            problem,
            settings: Some(state.settings.clone()),
        }))
    }
    #[cfg(test)]
    fn resume(&self, id: &str, new_job_id: &str) -> Result<BatchPlan, String> {
        self.resume_checked(id, new_job_id, &|| false)
    }
    pub fn resume_checked(
        &self,
        id: &str,
        new_job_id: &str,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<BatchPlan, String> {
        let mut contents = self.contents.lock().map_err(|e| e.to_string())?;
        let state = contents
            .checkpoint
            .as_mut()
            .filter(|state| state.job_id == id)
            .ok_or("Recovery checkpoint changed; reopen the prompt.")?;
        Self::reconcile(state, &self.path)?;
        let mut candidate = state.clone();
        for (index, item) in candidate.items.iter().enumerate() {
            if !candidate.completed.contains_key(&index)
                && !candidate.skipped.contains_key(&index)
                && !candidate
                    .intent
                    .as_ref()
                    .is_some_and(|intent| intent.index == index)
                && crate::output::existing(&item.output)?
            {
                candidate.skipped.insert(index, stamp(&item.output)?);
            }
        }
        Self::validate(&candidate)?;
        Self::check_files(&candidate, true, cancelled)?;
        candidate.job_id = new_job_id.into();
        candidate.intent = None;
        crate::atomic_file::json(&self.path, &candidate)?;
        *state = candidate;
        Ok(BatchPlan {
            job_id: new_job_id.into(),
            settings: state.settings.clone(),
            items: state.items.clone(),
            kind: TaskKind::Batch,
            warnings: Vec::new(),
        })
    }
    pub fn completed(&self, index: usize) -> Result<bool, String> {
        Ok(self
            .contents
            .lock()
            .map_err(|e| e.to_string())?
            .checkpoint
            .as_ref()
            .is_some_and(|state| {
                state.completed.contains_key(&index) || state.skipped.contains_key(&index)
            }))
    }
    pub fn record_existing(&self, index: usize) -> Result<(), String> {
        let mut contents = self.contents.lock().map_err(|e| e.to_string())?;
        let state = contents
            .checkpoint
            .as_mut()
            .ok_or("Missing active checkpoint.")?;
        let item = state.items.get(index).ok_or("Invalid skip index.")?;
        if !crate::output::existing(&item.output)? {
            return Err("Output disappeared before it could be skipped.".into());
        }
        let mut candidate = state.clone();
        if !candidate.completed.contains_key(&index) {
            candidate.skipped.insert(index, stamp(&item.output)?);
            crate::atomic_file::json(&self.path, &candidate)?;
            *state = candidate;
        }
        Ok(())
    }

    #[cfg(test)]
    fn commit(&self, index: usize, temp: &Path, output: &Path) -> Result<(), String> {
        self.commit_checked(index, temp, output, &|| false)
    }
    pub fn commit_checked(
        &self,
        index: usize,
        temp: &Path,
        output: &Path,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<(), String> {
        let mut contents = self.contents.lock().map_err(|e| e.to_string())?;
        let state = contents
            .checkpoint
            .as_mut()
            .ok_or("Missing active checkpoint.")?;
        if state
            .items
            .get(index)
            .is_none_or(|item| item.output != output)
        {
            return Err("Checkpoint commit does not match its frozen queue.".into());
        }
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(temp)
            .map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        drop(file);
        let intent = Intent {
            index,
            digest: digest(temp, cancelled)?,
            stamp: stamp(temp)?,
        };
        if intent.stamp.length == 0 {
            return Err("FFmpeg produced an empty output.".into());
        }
        state.intent = Some(intent.clone());
        crate::atomic_file::json(&self.path, state)?;
        crate::atomic_file::rename(temp, output, false)?;
        state.completed.insert(
            index,
            Completed {
                digest: intent.digest,
                stamp: intent.stamp,
            },
        );
        state.intent = None;
        crate::atomic_file::json(&self.path, state)?;
        log::info!(
            "Checkpoint committed episode {}: {}",
            index + 1,
            output.display()
        );
        Ok(())
    }
    pub fn discard(&self, expected_id: &str) -> Result<(), String> {
        let mut contents = self.contents.lock().map_err(|e| e.to_string())?;
        let actual = contents
            .checkpoint
            .as_ref()
            .map(|state| state.job_id.as_str())
            .unwrap_or("invalid");
        if actual != expected_id {
            return Err("Recovery checkpoint changed; reopen the prompt.".into());
        }
        if self.path.exists() {
            if contents.fault.is_some() {
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_err(|e| e.to_string())?
                    .as_nanos();
                crate::atomic_file::rename(
                    &self.path,
                    &self
                        .path
                        .with_file_name(format!("batch_state.invalid-{timestamp}.json")),
                    false,
                )?;
            } else {
                fs::remove_file(&self.path).map_err(|e| e.to_string())?;
            }
        }
        contents.checkpoint = None;
        contents.fault = None;
        Ok(())
    }
    pub fn finish(&self, job_id: &str) -> Result<(), String> {
        self.discard(job_id)
    }
}

pub async fn blocking<T: Send + 'static>(
    task: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tokio::task::spawn_blocking(task)
        .await
        .map_err(|e| format!("Local recovery worker failed: {e}"))?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preexisting_finals_are_skipped_and_recover_without_being_overwritten() {
        let root = tempfile::tempdir().expect("fixture");
        let state = root.path().join("state");
        let plan = plan(root.path(), "skip");
        let final_path = &plan.items[0].output;
        fs::write(final_path, b"user final").expect("final");
        let store = SessionStore::open(&state).expect("open");
        store.begin(&plan).expect("begin");
        assert!(store.completed(0).expect("skip checkpoint"));
        assert_eq!(
            store.info().expect("info").expect("saved").next_episode,
            Some(2)
        );
        drop(store);
        let store = SessionStore::open(&state).expect("reopen");
        store.resume("skip", "again").expect("resume");
        assert_eq!(fs::read(final_path).expect("preserved"), b"user final");
        assert!(store.completed(0).expect("skip retained"));
    }
    #[test]
    fn newly_existing_finals_can_be_adopted_and_changed_skipped_files_are_preserved() {
        let root = tempfile::tempdir().expect("fixture");
        let state = root.path().join("state");
        let plan = plan(root.path(), "adopt");
        let store = SessionStore::open(&state).expect("open");
        store.begin(&plan).expect("begin");
        fs::write(&plan.items[1].output, b"existing after start").expect("new final");
        store.record_existing(1).expect("checkpoint skip");
        assert!(store.completed(1).expect("skip recorded"));
        fs::write(&plan.items[2].output, b"existing after crash").expect("another final");
        store.resume("adopt", "adopted").expect("adopt on resume");
        assert!(store.completed(2).expect("adopted"));
        fs::write(&plan.items[1].output, b"modified user output, preserve it").expect("change");
        assert!(store.resume("adopted", "blocked").is_err());
        assert_eq!(
            fs::read(&plan.items[1].output).expect("preserved"),
            b"modified user output, preserve it"
        );
    }
    #[test]
    fn older_checkpoints_without_skipped_map_still_load() {
        let root = tempfile::tempdir().expect("fixture");
        let state = root.path().join("state");
        let plan = plan(root.path(), "older");
        let store = SessionStore::open(&state).expect("open");
        store.begin(&plan).expect("begin");
        drop(store);
        let path = state.join("batch_state.json");
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).expect("read")).expect("json");
        value.as_object_mut().expect("object").remove("skipped");
        crate::atomic_file::json(&path, &value).expect("old schema");
        let store = SessionStore::open(&state).expect("reopen");
        assert!(store.info().expect("info").expect("saved").can_resume);
    }
    fn plan(root: &Path, job: &str) -> BatchPlan {
        let input = root.join("input");
        let output = root.join("output");
        fs::create_dir_all(&input).expect("input");
        fs::create_dir_all(&output).expect("output");
        let input = fs::canonicalize(input).expect("canonical");
        let output = fs::canonicalize(output).expect("canonical");
        let items = (1..=3)
            .map(|index| {
                let source = input.join(format!("Episode{index:02}.mkv"));
                fs::write(&source, b"source fixture").expect("source");
                Item {
                    input: source,
                    output: output.join(format!("Keroro - Ep{index:02} [Wii].mp4")),
                    subtitle: None,
                    subtitle_root: None,
                    validation_error: None,
                }
            })
            .collect();
        BatchPlan {
            job_id: job.into(),
            settings: Settings {
                input_dir: input.to_string_lossy().into_owned(),
                output_dir: output.to_string_lossy().into_owned(),
                subtitle_dir: None,
                anime_title: "Keroro".into(),
                crf: 23,
            },
            items,
            kind: TaskKind::Batch,
            warnings: Vec::new(),
        }
    }
    #[test]
    fn legacy_episode_prefix_resumes_without_renaming_the_frozen_queue() {
        let root = tempfile::tempdir().expect("fixture");
        let state = root.path().join("state");
        let store = SessionStore::open(&state).expect("open");
        let mut plan = plan(root.path(), "legacy");
        for (index, item) in plan.items.iter_mut().enumerate() {
            item.output = item
                .output
                .with_file_name(format!("Keroro - E{:02} [Wii].mp4", index + 1));
        }
        store.begin(&plan).expect("legacy checkpoint");
        drop(store);
        let store = SessionStore::open(&state).expect("reopen");
        let restored = store.resume("legacy", "resumed").expect("resume");
        assert_eq!(restored.items[0].output, plan.items[0].output);
        assert_eq!(restored.items[2].output, plan.items[2].output);
    }
    #[test]
    fn checkpoints_resume_original_names_and_preserve_completed_outputs() {
        let root = tempfile::tempdir().expect("fixture");
        let state = root.path().join("state");
        let store = SessionStore::open(&state).expect("open");
        let plan = plan(root.path(), "original");
        store.begin(&plan).expect("checkpoint");
        let item = plan.items.first().expect("episode");
        let temp = item.output.with_file_name(".episode.original.part.mp4");
        fs::write(&temp, b"complete MP4 fixture").expect("temp");
        store.commit(0, &temp, &item.output).expect("commit");
        fs::write(
            std::path::Path::new(&plan.settings.input_dir).join("Episode00.mkv"),
            b"new source after checkpoint",
        )
        .expect("new source must not renumber original queue");
        assert_eq!(
            store.info().expect("info").expect("recovery").next_episode,
            Some(2)
        );
        assert!(
            SessionStore::open(&state).is_err(),
            "second instance must not sweep an active queue"
        );
        drop(store);
        let store = SessionStore::open(&state).expect("reopen");
        let restored = store.resume("original", "resumed").expect("resume");
        assert_eq!(restored.items.len(), 3);
        assert_eq!(restored.items[1].output, plan.items[1].output);
        assert!(store.completed(0).expect("completed"));
        assert!(!store.completed(1).expect("pending"));
        assert_eq!(
            fs::read(&item.output).expect("original output"),
            b"complete MP4 fixture"
        );
    }
    #[test]
    fn replaced_output_or_changed_pending_source_blocks_resume_without_deletion() {
        let root = tempfile::tempdir().expect("fixture");
        let store = SessionStore::open(&root.path().join("state")).expect("open");
        let plan = plan(root.path(), "test");
        store.begin(&plan).expect("begin");
        fs::write(&plan.items[1].input, b"changed source").expect("modify");
        assert!(!store.info().expect("info").expect("checkpoint").can_resume);
        assert!(store.resume("test", "retry").is_err());
        assert!(plan.items[1].input.exists());
    }
    #[test]
    fn corrupt_checkpoint_can_be_archived_without_erasing_media() {
        let root = tempfile::tempdir().expect("fixture");
        fs::write(root.path().join("batch_state.json"), b"{broken").expect("corrupt");
        let store = SessionStore::open(root.path()).expect("open with fault");
        assert!(!store.info().expect("info").expect("fault").can_resume);
        store.discard("invalid").expect("archive");
        assert!(store.info().expect("clear").is_none());
        assert!(
            fs::read_dir(root.path())
                .expect("files")
                .filter_map(Result::ok)
                .any(|entry| entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("batch_state.invalid-"))
        );
    }
    #[test]
    fn rename_to_checkpoint_crash_window_is_reconciled() {
        let root = tempfile::tempdir().expect("fixture");
        let state_dir = root.path().join("state");
        let store = SessionStore::open(&state_dir).expect("open");
        let plan = plan(root.path(), "commit-window");
        store.begin(&plan).expect("begin");
        let item = plan.items.first().expect("item");
        let temp = item.output.with_file_name(".crash.part.mp4");
        fs::write(&temp, b"complete").expect("temp");
        {
            let mut contents = store.contents.lock().expect("lock");
            let checkpoint = contents.checkpoint.as_mut().expect("checkpoint");
            checkpoint.intent = Some(Intent {
                index: 0,
                digest: digest(&temp, &|| false).expect("digest"),
                stamp: stamp(&temp).expect("stamp"),
            });
            crate::atomic_file::json(&store.path, checkpoint).expect("intent");
        }
        crate::atomic_file::rename(&temp, &item.output, false)
            .expect("commit before simulated crash");
        drop(store);
        let reopened = SessionStore::open(&state_dir).expect("recover");
        reopened.info().expect("reconcile in recovery worker");
        assert!(reopened.completed(0).expect("reconciled"));
        assert_eq!(
            reopened
                .info()
                .expect("info")
                .expect("checkpoint")
                .next_episode,
            Some(2)
        );
    }

    #[test]
    fn same_length_output_changes_are_detected_by_checksum_and_preserved() {
        let root = tempfile::tempdir().expect("fixture");
        let store = SessionStore::open(&root.path().join("state")).expect("open");
        let plan = plan(root.path(), "checksum");
        store.begin(&plan).expect("begin");
        let item = &plan.items[0];
        let temp = item.output.with_file_name(".checksum.part.mp4");
        fs::write(&temp, b"complete").expect("temp");
        store.commit(0, &temp, &item.output).expect("commit");
        fs::write(&item.output, b"modified").expect("same length replacement");
        assert!(
            store
                .resume("checksum", "retry")
                .err()
                .expect("reject")
                .contains("checksum")
        );
        assert_eq!(fs::read(&item.output).expect("preserved"), b"modified");
    }
}
```

## src-tauri/crates/encoder-core/src/subtitles.rs

```rust
use encoding_rs::{Encoding, SHIFT_JIS, UTF_16BE, UTF_16LE, WINDOWS_1252};
use std::{io::Write, path::Path};
use tempfile::{NamedTempFile, TempDir};
use tokio::{fs, io::AsyncReadExt};

/// Decimal MB. Check metadata before reading, then bound reads against growth.
pub const MAX_SUBTITLE_BYTES: u64 = 50_000_000;
pub const EXTENSIONS: &[&str] = &["srt", "ass", "ssa", "vtt"];
pub fn extension(path: &Path) -> Option<&'static str> {
    let ext = path.extension()?.to_str()?;
    EXTENSIONS
        .iter()
        .copied()
        .find(|supported| ext.eq_ignore_ascii_case(supported))
}
pub struct SubtitleText {
    pub text: String,
    pub warning: Option<String>,
}

#[derive(Debug)]
pub enum ReadError {
    Unavailable(String),
    Syntax(String),
}
impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable(message) => f.write_str(message),
            Self::Syntax(message) => write!(f, "Subtitle syntax error: {message}"),
        }
    }
}
impl From<String> for ReadError {
    fn from(message: String) -> Self {
        Self::Unavailable(message)
    }
}
impl From<&str> for ReadError {
    fn from(message: &str) -> Self {
        Self::Unavailable(message.into())
    }
}

/// Keep both owners alive until FFmpeg exits. Fields drop in declaration order.
pub struct StagedSubtitle {
    file: NamedTempFile,
    directory: TempDir,
}
impl StagedSubtitle {
    pub fn directory(&self) -> &Path {
        self.directory.path()
    }
    #[cfg(test)]
    pub(crate) fn path(&self) -> &Path {
        self.file.path()
    }
    pub fn filename(&self) -> Result<&str, String> {
        let name = self
            .file
            .path()
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or("Invalid internal subtitle filename.")?;
        if !name
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'-' | b'_'))
        {
            return Err("Unsafe internal subtitle filename.".into());
        }
        Ok(name)
    }
    pub fn close(self) -> Result<(), String> {
        let Self { file, directory } = self;
        let path = file.path().to_path_buf();
        let file_result = file.close();
        let directory_result = directory.close();
        match (file_result, directory_result) {
            (Ok(()), Ok(())) => crate::artifacts::forget_sync(&path),
            (file, directory) => Err(format!(
                "Subtitle temp cleanup failed: file={file:?}, directory={directory:?}"
            )),
        }
    }
}
fn validate_metadata(metadata: &std::fs::Metadata) -> Result<(), String> {
    if !metadata.is_file() {
        return Err("Subtitle must be a regular file.".into());
    }
    if metadata.len() > MAX_SUBTITLE_BYTES {
        return Err("Subtitle exceeds the 50 MB size limit; skipped before reading.".into());
    }
    Ok(())
}
pub async fn read_scoped(path: &Path, root: Option<&Path>) -> Result<SubtitleText, ReadError> {
    let canonical = fs::canonicalize(path)
        .await
        .map_err(|e| format!("Cannot resolve subtitle: {e}"))?;
    if root.is_none_or(|root| canonical.parent() != Some(root)) {
        return Err("Subtitle resolves outside the selected canonical subtitle folder.".into());
    }
    read(&canonical).await
}
pub async fn read(path: &Path) -> Result<SubtitleText, ReadError> {
    let ext = extension(path).ok_or("Unsupported subtitle format.")?;
    validate_metadata(
        &fs::metadata(path)
            .await
            .map_err(|e| format!("Cannot inspect subtitle: {e}"))?,
    )?;
    let file = fs::File::open(path)
        .await
        .map_err(|e| format!("Cannot read subtitle: {e}"))?;
    let metadata = file.metadata().await.map_err(|e| e.to_string())?;
    validate_metadata(&metadata)?; // Covers replacement between metadata and open.
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(metadata.len() as usize + 1)
        .map_err(|_| "Not enough memory to read this subtitle.")?;
    file.take(MAX_SUBTITLE_BYTES + 1)
        .read_to_end(&mut bytes)
        .await
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_SUBTITLE_BYTES {
        return Err("Subtitle grew beyond the 50 MB limit while reading.".into());
    }
    tokio::task::spawn_blocking(move || {
        let decoded = decode(&bytes).map_err(ReadError::Unavailable)?;
        validate_syntax(&decoded.text, ext).map_err(ReadError::Syntax)?;
        Ok(decoded)
    })
    .await
    .map_err(|e| format!("Subtitle normalization worker failed: {e}"))?
}

fn time(value: &str, decimal: char, digits: usize, short: bool) -> Option<u64> {
    let value = value.trim().split_whitespace().next()?;
    let mut fields = value.split(':');
    let first = fields.next()?;
    let second = fields.next()?;
    let third = fields.next();
    if fields.next().is_some() {
        return None;
    }
    let (hours, minutes, seconds) = if let Some(third) = third {
        (first, second, third)
    } else if short {
        ("0", first, second)
    } else {
        return None;
    };
    let (seconds, fraction) = seconds.split_once(decimal)?;
    if minutes.len() != 2
        || seconds.len() != 2
        || fraction.len() != digits
        || [hours, minutes, seconds, fraction]
            .iter()
            .any(|part| part.is_empty() || !part.bytes().all(|c| c.is_ascii_digit()))
    {
        return None;
    }
    let hours = hours.parse::<u64>().ok()?;
    let minutes = minutes.parse::<u64>().ok()?;
    let seconds = seconds.parse::<u64>().ok()?;
    if minutes >= 60 || seconds >= 60 {
        return None;
    }
    let millis = fraction
        .parse::<u64>()
        .ok()?
        .checked_mul(if digits == 2 { 10 } else { 1 })?;
    hours
        .checked_mul(3_600_000)?
        .checked_add(minutes * 60_000 + seconds * 1000 + millis)
}

fn validate_syntax(text: &str, ext: &str) -> Result<(), String> {
    let styled = matches!(ext, "ass" | "ssa");
    if styled
        && !(text
            .lines()
            .any(|line| line.trim().eq_ignore_ascii_case("[Script Info]"))
            && text
                .lines()
                .any(|line| line.trim().eq_ignore_ascii_case("[Events]")))
    {
        return Err("ASS/SSA requires [Script Info] and [Events] headers.".into());
    }
    if ext == "vtt"
        && !text.lines().next().is_some_and(|line| {
            line == "WEBVTT" || line.starts_with("WEBVTT ") || line.starts_with("WEBVTT\t")
        })
    {
        return Err("VTT requires a WEBVTT header.".into());
    }
    let mut cues = 0;
    for (number, line) in text.lines().enumerate() {
        let line = line.trim_start();
        let pair = if styled {
            if !line
                .get(..9)
                .is_some_and(|prefix| prefix.eq_ignore_ascii_case("Dialogue:"))
            {
                continue;
            }
            let body = line.get(9..).ok_or("Invalid dialogue line.")?;
            let mut fields = body.splitn(4, ',');
            let _layer_or_marked = fields.next();
            Some((fields.next().unwrap_or(""), fields.next().unwrap_or("")))
        } else {
            line.split_once("-->")
        };
        let Some((start, end)) = pair else {
            continue;
        };
        let decimal = if ext == "srt" { ',' } else { '.' };
        let digits = if styled { 2 } else { 3 };
        let start = time(start, decimal, digits, ext == "vtt");
        let end = time(end, decimal, digits, ext == "vtt");
        if !matches!((start, end), (Some(start), Some(end)) if end >= start) {
            return Err(format!(
                "Invalid or reversed timestamp at line {}.",
                number + 1
            ));
        }
        cues += 1;
    }
    if cues == 0 {
        return Err("No recognizable timed subtitle cues.".into());
    }
    Ok(())
}
pub async fn stage(text: String, ext: &'static str) -> Result<StagedSubtitle, String> {
    if !EXTENSIONS.contains(&ext) {
        return Err("Invalid internal subtitle extension.".into());
    }
    tokio::task::spawn_blocking(move || {
        // tempfile uses a non-cryptographic name generator; explicitly add OS entropy.
        let mut entropy = [0u8; 32];
        getrandom::fill(&mut entropy)
            .map_err(|e| format!("Cannot obtain secure temp-name randomness: {e}"))?;
        let token = entropy[..16]
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let prefix = format!("keroro-sub-{token}-");
        let file_token = entropy[16..]
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let file_prefix = format!("subtitle-{file_token}-");
        let mut builder = tempfile::Builder::new();
        builder.prefix(&prefix).rand_bytes(16);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            builder.permissions(std::fs::Permissions::from_mode(0o700));
        }
        let directory = match crate::artifacts::workspace() {
            Some(parent) => builder.tempdir_in(parent),
            None => builder.tempdir(),
        }
        .map_err(|e| format!("Cannot create isolated subtitle directory: {e}"))?;
        let mut file = tempfile::Builder::new()
            .prefix(&file_prefix)
            .suffix(&format!(".{ext}"))
            .rand_bytes(16)
            .tempfile_in(directory.path())
            .map_err(|e| format!("Cannot create private subtitle file: {e}"))?;
        crate::artifacts::record_sync(file.path())?;
        file.write_all(text.as_bytes())
            .and_then(|_| file.flush())
            .map_err(|e| format!("Cannot stage UTF-8 subtitle: {e}"))?;
        Ok(StagedSubtitle { file, directory })
    })
    .await
    .map_err(|e| format!("Subtitle staging worker failed: {e}"))?
}
fn strict(bytes: &[u8], encoding: &'static Encoding) -> Result<String, String> {
    encoding
        .decode_without_bom_handling_and_without_replacement(bytes)
        .map(|text| text.into_owned())
        .ok_or_else(|| format!("Invalid {} subtitle text.", encoding.name()))
}
fn decode(bytes: &[u8]) -> Result<SubtitleText, String> {
    let (text, warning) = if let Some(data) = bytes.strip_prefix(&[0xff, 0xfe]) {
        (
            strict(data, UTF_16LE)?,
            Some("Converted UTF-16 BOM subtitle to UTF-8.".into()),
        )
    } else if let Some(data) = bytes.strip_prefix(&[0xfe, 0xff]) {
        (
            strict(data, UTF_16BE)?,
            Some("Converted UTF-16 BOM subtitle to UTF-8.".into()),
        )
    } else {
        let data = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
        match std::str::from_utf8(data) {
            Ok(text) => (text.to_owned(), None),
            Err(_) if bytes.starts_with(&[0xef, 0xbb, 0xbf]) => {
                return Err("Subtitle declares UTF-8 but contains invalid bytes.".into());
            }
            Err(_) => {
                let japanese = strict(data, SHIFT_JIS).ok().filter(|text| {
                    text.chars().any(
                        |c| matches!(c as u32, 0x3040..=0x30ff | 0x3400..=0x9fff | 0xff61..=0xff9f),
                    )
                });
                if let Some(text) = japanese {
                    (text, Some("Guessed Shift-JIS/Windows-932 subtitle encoding; verify the preview or save as UTF-8.".into()))
                } else {
                    (strict(data, WINDOWS_1252)?, Some("Guessed Windows-1252 subtitle encoding; ANSI varies. Verify the preview or save as UTF-8.".into()))
                }
            }
        }
    };
    if text.trim().is_empty()
        || text.chars().any(|c| {
            c == '\0' || c == '\u{fffd}' || (c.is_control() && !matches!(c, '\r' | '\n' | '\t'))
        })
    {
        return Err("Empty, binary, or invalid subtitle text; save as UTF-8 and retry.".into());
    }
    Ok(SubtitleText { text, warning })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unicode_and_legacy_decoding_are_strict() {
        assert_eq!(
            decode("\u{30b1}\u{30ed}\u{30ed}".as_bytes())
                .expect("UTF-8")
                .text,
            "\u{30b1}\u{30ed}\u{30ed}"
        );
        assert_eq!(decode(b"\xef\xbb\xbfHello").expect("BOM").text, "Hello");
        assert_eq!(decode(b"\xff\xfeH\0i\0").expect("UTF-16").text, "Hi");
        assert!(decode(b"\xff\xfeX").is_err());
        assert!(decode(b"binary\0data").is_err());
        assert!(decode(b"\xef\xbb\xbfbroken\xff").is_err());
        let sjis = decode(&[0x83, 0x50, 0x83, 0x8d, 0x83, 0x8d]).expect("Shift-JIS");
        assert_eq!(sjis.text, "\u{30b1}\u{30ed}\u{30ed}");
        assert!(sjis.warning.expect("guess").contains("Shift-JIS"));
        assert_eq!(decode(b"caf\xe9").expect("CP1252").text, "caf\u{e9}");
    }
    #[tokio::test]
    async fn staging_is_isolated_and_drops_both_paths() {
        let first = stage("hello".into(), "ssa").await.expect("stage");
        let second = stage("world".into(), "vtt").await.expect("stage");
        assert_ne!(first.directory(), second.directory());
        assert!(first.filename().expect("safe basename").ends_with(".ssa"));
        let file = first.path().to_path_buf();
        let directory = first.directory().to_path_buf();
        drop(first);
        assert!(!file.exists() && !directory.exists());
    }
    #[tokio::test]
    async fn oversized_file_is_rejected_before_decoding() {
        let root = tempfile::tempdir().expect("fixture");
        let path = root.path().join("disguised.ass");
        let file = std::fs::File::create(&path).expect("file");
        file.set_len(MAX_SUBTITLE_BYTES + 1)
            .expect("oversized fixture");
        drop(file);
        assert!(
            read(&path)
                .await
                .err()
                .expect("reject")
                .to_string()
                .contains("skipped before reading")
        );
    }

    #[tokio::test]
    async fn scoped_reader_rejects_escape_and_accepts_canonical_root() {
        let fixture = tempfile::tempdir().expect("fixture");
        let root = fixture.path().join("selected");
        std::fs::create_dir(&root).expect("root");
        let inside = root.join("episode.vtt");
        let outside = fixture.path().join("outside.vtt");
        for path in [&inside, &outside] {
            std::fs::write(path, "WEBVTT\n\n00:00:00.000 --> 00:00:01.000\nHello\n").expect("VTT");
        }
        let canonical = std::fs::canonicalize(&root).expect("canonical");
        assert!(read_scoped(&inside, Some(&canonical)).await.is_ok());
        assert!(
            read_scoped(&root.join("../outside.vtt"), Some(&canonical))
                .await
                .is_err()
        );
    }

    #[test]
    fn malformed_timing_and_headers_cannot_silently_probe_as_another_format() {
        assert!(validate_syntax("WEBVTT\n\nBROKEN --> NOT_A_TIME\nText\n", "vtt").is_err());
        assert!(validate_syntax("[Events]\nDialogue: broken\n", "ass").is_err());
        assert!(validate_syntax("1\n00:00:02,000 --> 00:00:01,000\nText\n", "srt").is_err());
        assert!(
            validate_syntax(
                "WEBVTT\n\n00:00.000 --> 00:01.000 align:start\nText\n",
                "vtt"
            )
            .is_ok()
        );
    }
}
```

## .gitignore

```gitignore
# Logs
logs
*.log
npm-debug.log*
yarn-debug.log*
yarn-error.log*
pnpm-debug.log*
lerna-debug.log*

node_modules/
dist
dist-ssr
.work/
**/target/
*.local

# Editor directories and files
.vscode/*
!.vscode/extensions.json
.idea
.DS_Store
*.suo
*.ntvs*
*.njsproj
*.sln
*.sw?

# Heavy generated output (keep bundled sidecars and source assets)
src-tauri/target/
releases/

# Local media fixtures and temporary files
*.mp4
*.mkv
*.avi
*.mov
*.wmv
*.flv
*.webm
*.tmp
.keroro_access_test.tmp

# Extracted local test application
/Release/
*.part
```

## src-tauri/.gitignore

```gitignore
# Generated by Cargo
# will have compiled files and executables
/target/

# Generated by Tauri
# will have schema files for capabilities auto-completion
/gen/schemas

# Nested Cargo builds and frontend caches
**/target/
node_modules/

# Local media fixtures and temporary files
*.mp4
*.mkv
*.avi
*.mov
*.wmv
*.flv
*.webm
*.tmp
.keroro_access_test.tmp
*.part
```

## scripts/clean.mjs

```javascript
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
```

## extract-release.js

```javascript
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
  rejectLinks(buildDirectory);
  const executable = fs.readdirSync(buildDirectory, { withFileTypes: true })
    .find((entry) => entry.isFile() && entry.name.toLowerCase() === "keroro-wii-encoder.exe");
  if (!executable) throw new Error("Compiled application not found. Run npm run build:release first.");

  const binaries = path.join(root, "src-tauri", "binaries");
  const files = [
    [path.join(buildDirectory, executable.name), "Keroro-Wii-Encoder-v1.0.exe"],
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
  console.log(`Release ready: ${path.join(releaseDirectory, "Keroro-Wii-Encoder-v1.0.exe")}`);
  console.log("Keep ffmpeg.exe and ffprobe.exe beside the application.");
} catch (error) {
  console.error(`Release extraction failed: ${error instanceof Error ? error.message : String(error)}`);
  console.error("If the destination executable is locked, exit the app through its tray menu and retry.");
  process.exitCode = 1;
}
```
