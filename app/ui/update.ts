import { toast } from "./lib.ts";
import { S, version } from "./state.ts";
import { renderSide } from "./shell.ts";

// New releases install themselves: checked at launch and every few hours, downloaded in the
// background, then a quiet "Restart to update" in the sidebar. Dev builds never update.
export let updateReady: string | null = null;

const EVERY = 6 * 60 * 60 * 1000;

let timer: ReturnType<typeof setTimeout> | undefined;

// `asked` is "Check for Updates…" in the OneCorp menu: same check, but it says how it went.
export async function checkForUpdate(asked = false) {
  const updater = window.__TAURI__?.updater;
  const say = (msg: string, err?: boolean) => asked && toast(msg, err);
  if (S.dev || !updater) return say("Dev builds don't update. Rebuild to get the latest.");
  if (updateReady) return say(`OneCorp ${updateReady} is ready. Restart to update, in the sidebar.`);
  try {
    const found = await updater.check();
    if (found) {
      say(`Downloading OneCorp ${found.version}…`);
      await found.downloadAndInstall();
      updateReady = found.version;
      renderSide();
      say(`OneCorp ${found.version} is ready. Restart to update, in the sidebar.`);
    } else {
      say(`You're up to date. OneCorp ${version} is the latest.`);
    }
  } catch {
    // Offline, or GitHub unreachable: the next check tries again.
    say("Couldn't check for updates. Try again in a moment.", true);
  }
  clearTimeout(timer);
  timer = setTimeout(checkForUpdate, EVERY);
}

export const restartToUpdate = () => window.__TAURI__?.process?.relaunch();
