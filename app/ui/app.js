// OneCorp app UI. It polls the company snapshot and renders the Company page and the Inbox.
// Every action calls a Tauri command, which runs the same library code as the CLI.
// Opened in a plain browser (no Tauri), it runs on sample data so the design can be checked quickly.

import { logo } from "./brand.js";
import { drawerTicket, openHire, openPerson, openTicket, renderDrawer } from "./drawers.js";
import { face } from "./look.js";
import { preview, previewScreen } from "./preview.js";
import { refresh, render } from "./render.js";
import { renderPop, toggleRail } from "./shell.js";
import { S } from "./state.js";
import { approvalCopy, review, reviewActions } from "./views/inbox.js";
import { checkRepo, startSetup } from "./views/onboarding.js";
import { openSettings, renderSheet } from "./views/settings.js";
import "./events.js";

// Room for the window buttons in the app, and in screenshots that draw them (?native).
if (window.__TAURI__ || /[?&]native\b/.test(location.search)) document.documentElement.classList.add("native");

document.getElementById("boot")?.insertAdjacentHTML("afterbegin", logo());

try { if (localStorage.getItem("onecorp.rail")) toggleRail(true); } catch {}

if (window.__TAURI__) refresh(true); else previewScreen();

// For the UI smoke test, which drives these directly.
export { S, preview, refresh, render, renderDrawer, renderPop, renderSheet, face, review, reviewActions, approvalCopy, drawerTicket, openTicket, openHire, openPerson, openSettings, startSetup, checkRepo };
