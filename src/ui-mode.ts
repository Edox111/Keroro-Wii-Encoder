import { getPreferences, savePreferences } from "./preferences";
import { defaultVideo, type VideoSettings } from "./video";

type UiMode = "simple" | "advanced";
type PresetName = "classic" | "widescreen" | "fastest";
export interface PresetSettings {
  video: VideoSettings;
  crf: 20 | 23 | 26;
  cpuMode: "balanced" | "maximum";
}
interface ModeOptions {
  applyPreset(settings: PresetSettings): void;
  readSettings(): { video: VideoSettings | null; crf: number; cpuMode: string };
}
export interface UiModeController {
  setLocked(locked: boolean): void;
  dispose(): void;
}

let uiMode: UiMode = getPreferences().uiMode;
const presets: Record<PresetName, PresetSettings & { hint: string }> = {
  classic: {
    video: { ...defaultVideo }, crf: 23, cpuMode: "maximum",
    hint: "640x480 with a 4:3 display shape. A safe starting point for a CRT.",
  },
  widescreen: {
    video: { ...defaultVideo, aspectRatio: "16:9", resolutionPreset: "720x480" },
    crf: 23, cpuMode: "maximum",
    hint: "Recommended 720x480 anamorphic 16:9, fitted without stretching. Up to 30 fps.",
  },
  fastest: {
    video: { ...defaultVideo }, crf: 26, cpuMode: "maximum",
    hint: "640x480, smaller files, Full Throttle. Uses an available GPU with automatic CPU fallback.",
  },
};
function isPreset(value: string): value is PresetName {
  return value === "classic" || value === "widescreen" || value === "fastest";
}
function element<T extends HTMLElement>(id: string, type: { new (): T }): T {
  const node = document.getElementById(id);
  if (!(node instanceof type)) throw new Error(`Missing UI mode element: ${id}`);
  return node;
}

/** Presentation only: existing field callbacks own persistence and IPC settings. */
export function initializeUiMode(options: ModeOptions): UiModeController {
  const simple = element("simpleModeBtn", HTMLButtonElement);
  const advanced = element("advancedModeBtn", HTMLButtonElement);
  const panel = element("advancedPanel", HTMLDetailsElement);
  const picker = element("simplePreset", HTMLSelectElement);
  const hint = element("simplePresetHint", HTMLParagraphElement);
  const logs = element("showLogs", HTMLInputElement);
  let selected = getPreferences().simplePreset;
  let locked = true;
  let pendingReset = uiMode === "simple";
  const listeners: Array<() => void> = [];
  const on = (node: EventTarget, event: string, handler: () => void) => {
    node.addEventListener(event, handler);
    listeners.push(() => node.removeEventListener(event, handler));
  };

  function applyPreset(): void {
    const preset = presets[selected];
    options.applyPreset({ video: { ...preset.video }, crf: preset.crf, cpuMode: preset.cpuMode });
    picker.value = selected;
    hint.textContent = preset.hint;
    savePreferences({ simplePreset: selected });
  }
  function reflectSettings(): void {
    const settings = options.readSettings();
    const match = (Object.keys(presets) as PresetName[]).find((name) => {
      const p = presets[name];
      return settings.video?.aspectRatio === p.video.aspectRatio
        && settings.video.resolutionPreset === p.video.resolutionPreset
        && settings.video.autoCrop === p.video.autoCrop
        && settings.video.encoderOverride === p.video.encoderOverride
        && settings.crf === p.crf && settings.cpuMode === p.cpuMode;
    });
    picker.value = match ?? "custom";
    hint.textContent = match ? presets[match].hint
      : "Manual settings active. Simple Mode restores the last selected safe preset.";
  }
  function renderMode(): void {
    document.body.dataset.uiMode = uiMode;
    simple.setAttribute("aria-pressed", String(uiMode === "simple"));
    advanced.setAttribute("aria-pressed", String(uiMode === "advanced"));
    panel.hidden = uiMode === "simple";
    if (uiMode === "advanced") panel.open = true;
  }
  function selectMode(next: UiMode): void {
    if (locked || uiMode === next) return;
    uiMode = next;
    if (next === "simple") applyPreset();
    else reflectSettings();
    renderMode();
    savePreferences({ uiMode });
  }
  on(simple, "click", () => selectMode("simple"));
  on(advanced, "click", () => selectMode("advanced"));
  on(picker, "change", () => {
    if (locked || !isPreset(picker.value)) return;
    selected = picker.value;
    applyPreset();
  });
  logs.checked = getPreferences().showLogs;
  document.body.dataset.showLogs = String(logs.checked);
  on(logs, "change", () => {
    document.body.dataset.showLogs = String(logs.checked);
    savePreferences({ showLogs: logs.checked });
  });
  for (const id of ["crfSelect", "cpuMode", "aspectRatio", "resolutionPreset", "customWidth", "customHeight", "autoCrop", "encoderOverride"]) {
    const field = document.getElementById(id);
    if (field) on(field, field instanceof HTMLInputElement && field.type !== "checkbox" ? "input" : "change", reflectSettings);
  }
  picker.value = selected;
  hint.textContent = presets[selected].hint;
  renderMode();
  if (uiMode === "advanced") reflectSettings();

  return {
    setLocked(value) {
      const wasLocked = locked;
      locked = value;
      simple.disabled = value;
      advanced.disabled = value;
      picker.disabled = value;
      if (!value && uiMode === "simple" && (pendingReset || wasLocked)) {
        pendingReset = false;
        applyPreset();
      } else if (!value && uiMode === "advanced") reflectSettings();
    },
    dispose() { listeners.splice(0).forEach((remove) => remove()); },
  };
}
