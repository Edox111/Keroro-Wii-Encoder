import { invoke } from "./ipc";
import { getPreferences, savePreferences } from "./preferences";
export interface DriveInfo { id: string; root: string; label: string; fileSystem: string; totalBytes: number; availableBytes: number }
export const FAT32_GUARD_BYTES = 4_000_000_000;
export function fat32Warning(volume: DriveInfo | null, duration?: number | null): string | null {
  if (!volume || !/^(fat32|vfat|msdos)$/i.test(volume.fileSystem)) return null;
  if (duration == null || !Number.isFinite(duration) || duration <= 0) return "FAT32: duration unknown; the encoder will enforce a safe size limit.";
  return Math.ceil(duration * 420_000 * 1.02) >= FAT32_GUARD_BYTES ? "FAT32 4GB Warning — split this video or choose another destination." : null;
}
const gb = (bytes: number) => `${(bytes / 1_000_000_000).toFixed(1)} GB`;
export function initializeDriveSelector(output: HTMLInputElement, canChange: () => boolean, changed: (volume: DriveInfo | null) => void, save: () => void) {
  const select = document.querySelector<HTMLSelectElement>("#driveTarget")!;
  const status = document.querySelector<HTMLElement>("#driveStatus")!;
  const meter = document.querySelector<HTMLMeterElement>("#driveUsage")!;
  let drives: DriveInfo[] = [], disposed = false, refreshing = false, routing = false, request = 0, inspected = "";
  let selected = getPreferences().selectedDriveId;
  let routedPath = getPreferences().routedOutputPath;
  let volume: DriveInfo | null = null;
  function setStatus(text: string) { status.textContent = text; status.title = text; }
  function display() {
    select.replaceChildren(new Option("Output folder (manual)", ""), ...drives.map(drive => new Option(`${drive.label || "Removable drive"} (${drive.root}) — ${gb(drive.availableBytes)} free [${drive.fileSystem}]`, drive.id)));
    if (selected && !drives.some(d => d.id === selected)) { const missing = new Option("Selected drive disconnected", selected); missing.disabled = true; select.add(missing); }
    select.value = selected;
    select.title = select.selectedOptions[0]?.textContent ?? "";
    output.title = output.value;
    meter.hidden = !volume;
    if (volume) { meter.max = volume.totalBytes; meter.value = Math.max(0, volume.totalBytes - volume.availableBytes); meter.title = `${gb(volume.availableBytes)} free of ${gb(volume.totalBytes)}`; }
  }
  async function inspect(force = false) {
    const path = output.value.trim();
    if (disposed || !force && path === inspected) return;
    inspected = path; const version = ++request;
    if (path !== routedPath && selected) { selected = ""; routedPath = ""; savePreferences({ selectedDriveId: "", routedOutputPath: "" }); }
    try {
      const next = path ? await invoke("inspect_output_target", { path }) : null;
      if (disposed || version !== request) return;
      volume = next; setStatus(next ? `${gb(next.availableBytes)} free • ${next.fileSystem}` : "Select a drive or output folder.");
    } catch (error) {
      if (disposed || version !== request) return;
      volume = null; setStatus(`Drive unavailable: ${String(error)}`);
    }
    changed(volume); display();
  }
  async function route(id: string) {
    if (!canChange() || routing || disposed) return;
    if (!id) { selected = ""; routedPath = ""; savePreferences({ selectedDriveId: "", routedOutputPath: "", autoRouteWiiDrive: false }); display(); return; }
    routing = true; select.disabled = true;
    const previousPath = output.value;
    try {
      const path = await invoke("prepare_wii_drive", { driveId: id });
      if (disposed || !canChange() || output.value !== previousPath) return;
      selected = id; routedPath = path; output.value = path;
      savePreferences({ selectedDriveId: id, routedOutputPath: path }); save(); await inspect(true);
    } catch (error) { if (!disposed) setStatus(`Cannot target drive: ${String(error)}`); }
    finally { routing = false; select.disabled = !canChange(); }
  }
  async function refresh() {
    if (refreshing || disposed || document.hidden) return;
    refreshing = true;
    try {
      drives = await invoke("get_connected_drives");
      if (disposed) return;
      display();
      const preferences = getPreferences();
      const candidate = selected ? drives.find(d => d.id === selected) : drives.length === 1 ? drives[0] : undefined;
      if (preferences.autoRouteWiiDrive && candidate && canChange() && output.value !== routedPath) await route(candidate.id);
      else if (preferences.autoRouteWiiDrive && !selected && candidate && canChange()) await route(candidate.id);
      await inspect(true);
    } catch (error) { if (!disposed) setStatus(`Cannot refresh drives: ${String(error)}`); }
    finally { refreshing = false; }
  }
  const choose = () => { void route(select.value); };
  const focus = () => { void refresh(); };
  select.addEventListener("change", choose); window.addEventListener("focus", focus);
  const timer = window.setInterval(focus, 10_000);
  void refresh();
  return {
    inspect, refresh,
    setLocked(locked: boolean) { select.disabled = locked || routing; },
    get pending() { return routing; },
    async validate() {
      if (routing) throw new Error("Wait for the drive target to finish changing.");
      const path = output.value.trim();
      const current = await invoke("inspect_output_target", { path });
      if (selected && path === routedPath && current.id !== selected) throw new Error("The selected drive was disconnected or replaced. Select it again before encoding.");
      volume = current; changed(volume); display(); return current;
    },
    dispose() { disposed = true; request++; clearInterval(timer); select.removeEventListener("change", choose); window.removeEventListener("focus", focus); },
  };
}
