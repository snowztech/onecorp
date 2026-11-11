import { S } from "./state.ts";
import { renderSide } from "./shell.ts";

// New releases install themselves: checked at launch and every few hours, downloaded in the
// background, then a quiet "Restart to update" in the sidebar. Dev builds never update.
export let updateReady: string | null = null;

const EVERY = 6 * 60 * 60 * 1000;

export async function checkForUpdate() {
  const updater = window.__TAURI__?.updater;
  if (!updater || S.dev || updateReady) return;
  try {
    const found = await updater.check();
    if (found) {
      await found.downloadAndInstall();
      updateReady = found.version;
      renderSide();
    }
  } catch {
    // Offline, or GitHub unreachable: try again next time.
  }
  setTimeout(checkForUpdate, EVERY);
}

export const restartToUpdate = () => window.__TAURI__?.process?.relaunch();
