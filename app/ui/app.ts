// OneCorp app UI. It polls the company snapshot and renders the Company page and the Inbox.
// Every action calls a Tauri command, which runs the same library code as the CLI.
// Opened in a plain browser (no Tauri), it runs on sample data so the design can be checked quickly.

import { logo } from "./brand.ts";
import { drawerTicket, openHire, openPerson, openTicket, renderDrawer } from "./drawers.ts";
import { face } from "./look.ts";
import { preview, previewScreen } from "./preview.ts";
import { refresh, render } from "./render.ts";
import { renderPop, toggleRail } from "./shell.ts";
import { S } from "./state.ts";
import { approvalCopy, review, reviewActions } from "./views/inbox.ts";
import { checkRepo, startSetup } from "./views/onboarding.ts";
import { openSettings, renderSheet } from "./views/settings.ts";
import "./events.ts";
import { checkForUpdate } from "./update.ts";

// Room for the window buttons in the app, and in screenshots that draw them (?native).
if (window.__TAURI__ || /[?&]native\b/.test(location.search)) document.documentElement.classList.add("native");

document.getElementById("boot")?.insertAdjacentHTML("afterbegin", logo());

try { if (localStorage.getItem("onecorp.rail")) toggleRail(true); } catch {}

// The update check waits for the first load, which says whether this is a dev build.
if (window.__TAURI__) refresh(true).then(checkForUpdate); else previewScreen();

// For the UI smoke test, which drives these directly.
export { S, preview, refresh, render, renderDrawer, renderPop, renderSheet, face, review, reviewActions, approvalCopy, drawerTicket, openTicket, openHire, openPerson, openSettings, startSetup, checkRepo };
