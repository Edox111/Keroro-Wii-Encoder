import { invoke } from "@tauri-apps/api/core";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { showUpdateReady, type UpdateNotice } from "./update-toast";
import "./update-toast.css";
import { plugins } from "../src-tauri/tauri.conf.json";

interface UpdateLifecycle {
  canInstall(): boolean;
  setInstalling(value: boolean): void;
}

/** One background check per startup. Unavailable updates never block launch. */
export function startAutomaticUpdates(lifecycle: UpdateLifecycle): () => void {
  const abort = new AbortController();

  async function run(): Promise<void> {
    let update: Update | null = null;
    let reserved = false;
    let notice: UpdateNotice | null = null;
    try {
      const key = plugins.updater.pubkey.trim();
      if (!key || key.startsWith("YOUR_")) return;
      if (!await invoke<boolean>("updater_is_configured") || abort.signal.aborted) return;
      update = await check({ timeout: 10_000 });
      if (!update || abort.signal.aborted) return;
      await update.download(undefined, { timeout: 600_000 });
      if (abort.signal.aborted) return;
      notice = showUpdateReady(update.version, () => lifecycle.canInstall(), abort.signal);
      while (!abort.signal.aborted) {
        if (!await notice.waitForRestart() || abort.signal.aborted) return;
        if (!lifecycle.canInstall()) continue;
        reserved = await invoke<boolean>("prepare_update_install");
        if (reserved) break;
      }
      if (!reserved || abort.signal.aborted) return;
      notice.setInstalling();
      lifecycle.setInstalling(true);
      // Windows exits here; the NSIS /R handoff restarts the updated app.
      await update.install({ restartAfterInstall: true });
      await relaunch(); // Fallback for platforms where install returns normally.
    } catch {
      // Offline, unavailable feeds, bad signatures, and installation failures
      // must not prevent normal use or open a confirmation/error dialog.
    } finally {
      notice?.close();
      if (reserved) {
        await invoke("release_update_install").catch(() => {});
        lifecycle.setInstalling(false);
      }
      await update?.close().catch(() => {});
    }
  }

  void run();
  return () => abort.abort();
}
