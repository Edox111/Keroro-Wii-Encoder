import { populateEncoders } from "./encoder-selector";
import { isHardwareEncoder } from "./hardware";
import { getLanguagePreferences, updateLanguagePreferences } from "./language-preferences";


const languages = [
  { value: "eng", label: "English" },
  { value: "fra", label: "French" },
  { value: "deu", label: "German" },
  { value: "jpn", label: "Japanese" },
  { value: "srp", label: "Serbian" },
  { value: "spa", label: "Spanish" },
];

function populateLanguages(select: HTMLSelectElement, value: string, fallback: string, label: string): void {
  const options = [...languages];
  if (value !== fallback && !options.some(option => option.value === value)) {
    options.push({ value, label: value.toUpperCase() });
  }
  options.sort((a, b) => a.label.localeCompare(b.label, "en"));
  const defaultOption = new Option(label, fallback);
  defaultOption.className = "language-default";
  const group = document.createElement("optgroup");
  group.label = "Specific languages";
  group.append(...options.map(option => new Option(option.label, option.value)));
  select.replaceChildren(defaultOption, group);
  select.value = value;
  select.title = select.selectedOptions[0]?.textContent ?? "";
}

export function initializeSettingsModal(chooseDirectory: (current: string) => Promise<string | null>, canOpen: () => boolean) {
  const trigger = document.querySelector<HTMLButtonElement>("#settingsBtn");
  if (!trigger) throw new Error("Missing settings button.");
  const modal = document.createElement("dialog"); modal.className = "app-modal settings-modal"; modal.id = "settingsModal";
  modal.setAttribute("aria-labelledby", "settingsHeading");
  // Static markup only; paths and language metadata are assigned through DOM properties.
  modal.innerHTML = `<form class="modal-layout">
    <header class="modal-heading"><h2 id="settingsHeading">Preferences</h2>
      <p>Defaults apply to new files and automatic selections. Manual track choices are kept.</p>
    </header>
    <div class="modal-body">
      <fieldset class="settings-section"><legend>Encoding &amp; storage</legend>
        <div class="settings-field"><label for="hardwareEncoder">Hardware Encoder</label>
          <select id="hardwareEncoder" aria-describedby="encoderHelp"></select>
          <small id="encoderHelp">Detected GPUs are tested with the WiiMC profile. Driver failures fall back to x264.</small>
        </div>
        <label class="settings-toggle" for="autoRouteWiiDrive"><span>Auto-route output to Wii SD/USB when connected</span><input id="autoRouteWiiDrive" type="checkbox" role="switch"></label>
      </fieldset>
      <fieldset class="settings-section"><legend>Languages</legend>
        <div class="settings-language-grid">
          <div class="settings-field"><label for="preferredAudioLang">Preferred audio</label><select id="preferredAudioLang" aria-describedby="languageHelp"></select></div>
          <div class="settings-field"><label for="preferredSubLang">Preferred subtitles</label><select id="preferredSubLang" aria-describedby="languageHelp"></select></div>
        </div>
        <small id="languageHelp">Any uses the first audio track. None keeps external subtitle routing without auto-selecting embedded subtitles.</small>
      </fieldset>
      <div class="settings-field"><label for="defaultOutputDir">Default output folder</label>
        <div class="settings-folder"><input id="defaultOutputDir" type="text" placeholder="Optional fallback folder" maxlength="32768"><button type="button" id="settingsBrowseBtn">Browse</button></div>
        <small>Used when no output folder is selected. Matched external subtitles take priority over automatic embedded selections.</small>
      </div>
      <fieldset class="settings-section"><legend>When the batch finishes</legend>
        <label class="settings-toggle" for="playSoundOnComplete"><span>Play completion sound<small>Separate from UI click sounds.</small></span><input type="checkbox" role="switch" id="playSoundOnComplete"></label>
        <label class="settings-toggle" for="openFolderOnComplete"><span>Open output folder</span><input type="checkbox" role="switch" id="openFolderOnComplete"></label>
      </fieldset>
      <p id="settingsStatus" role="status"></p>
    </div>
    <footer class="modal-actions"><button type="button" id="settingsCancelBtn">Cancel</button><button type="submit" class="modal-primary">Save Preferences</button></footer>
  </form>`;
  document.body.append(modal);
  const form = modal.querySelector("form")!;
  const encoder = modal.querySelector<HTMLSelectElement>("#hardwareEncoder")!;
  const autoRoute = modal.querySelector<HTMLInputElement>("#autoRouteWiiDrive")!;
  const audio = modal.querySelector<HTMLSelectElement>("#preferredAudioLang")!;
  const subtitle = modal.querySelector<HTMLSelectElement>("#preferredSubLang")!;
  const folder = modal.querySelector<HTMLInputElement>("#defaultOutputDir")!;
  const sound = modal.querySelector<HTMLInputElement>("#playSoundOnComplete")!;
  const openFolder = modal.querySelector<HTMLInputElement>("#openFolderOnComplete")!;
  const browse = modal.querySelector<HTMLButtonElement>("#settingsBrowseBtn")!;
  const cancel = modal.querySelector<HTMLButtonElement>("#settingsCancelBtn")!;
  const status = modal.querySelector<HTMLElement>("#settingsStatus")!;
  let disposed = false, picking = false, generation = 0;
  function openSettings() {
    if (disposed || !canOpen() || modal.open) return;
    generation++;
    const current = getLanguagePreferences();
    autoRoute.checked = current.autoRouteWiiDrive;
    void populateEncoders(encoder, current.hardwareEncoder).catch(error => { if (!disposed && modal.open) status.textContent = `GPU discovery unavailable: ${String(error)}. Automatic CPU fallback remains available.`; });
    populateLanguages(audio, current.preferredAudioLang, "any", "Any (first available track)");
    populateLanguages(subtitle, current.preferredSubLang, "none", "None / External (default)");
    sound.checked = current.playSoundOnComplete; openFolder.checked = current.openFolderOnComplete;
    folder.value = current.defaultOutputDir; folder.title = folder.value; status.textContent = ""; modal.showModal();
  }
  async function pick() {
    if (picking) return;
    const request = generation;
    picking = true; browse.disabled = true;
    try { const path = await chooseDirectory(folder.value); if (!disposed && modal.open && request === generation && path !== null) { folder.value = path; folder.title = path; } }
    catch (error) { if (!disposed && modal.open && request === generation) status.textContent = `Cannot select folder: ${String(error)}`; }
    finally { picking = false; browse.disabled = false; }
  }
  function save(event: SubmitEvent) {
    event.preventDefault(); if (picking || disposed) return;
    try {
      if (!isHardwareEncoder(encoder.value)) throw new Error("Choose a supported encoder.");
      if (!updateLanguagePreferences({ hardwareEncoder: encoder.value, autoRouteWiiDrive: autoRoute.checked, preferredAudioLang: audio.value, preferredSubLang: subtitle.value, defaultOutputDir: folder.value, playSoundOnComplete: sound.checked, openFolderOnComplete: openFolder.checked })) {
        status.textContent = "Preferences apply for this session, but local storage is unavailable."; return;
      }
      modal.close();
    } catch (error) { status.textContent = String(error); }
  }
  const updateTitle = (event: Event) => {
    if (event.target instanceof HTMLSelectElement) event.target.title = event.target.selectedOptions[0]?.textContent ?? "";
    if (event.target === folder) folder.title = folder.value;
  };
  form.addEventListener("change", updateTitle);
  folder.addEventListener("input", updateTitle);
  const close = () => modal.close();
  const onPick = () => { void pick(); };
  trigger.addEventListener("click", openSettings); browse.addEventListener("click", onPick);
  cancel.addEventListener("click", close); form.addEventListener("submit", save);
  trigger.disabled = false;
  return { dispose() { disposed = true; trigger.removeEventListener("click", openSettings); browse.removeEventListener("click", onPick); cancel.removeEventListener("click", close); form.removeEventListener("submit", save); form.removeEventListener("change", updateTitle); folder.removeEventListener("input", updateTitle); modal.close(); modal.remove(); trigger.disabled = true; } };
}
