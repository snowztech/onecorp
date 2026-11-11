import { toast } from "./lib.js";
import { preview } from "./preview.js";
import { refresh, render } from "./render.js";
import { renderSheet } from "./views/settings.js";

// What the app knows (S), and how it talks to the Tauri side.
export const invoke = window.__TAURI__ ? window.__TAURI__.core.invoke : preview;

// The layout appears once the first render has decided it. If loading stalls, show it anyway.
// The app's version, from Tauri in the app and the sample in the browser preview.
export let version = "";

(window.__TAURI__?.app?.getVersion?.() || Promise.resolve("0.1.0")).then(v => { version = v; if (isBooted && (S.welcome || S.sheet)) { render(); renderSheet(); } }).catch(() => {});

// A bug report on GitHub, with the version already in it, so nobody has to look it up.
export const enc = t => encodeURIComponent(t).replace(/[!'()*]/g, c => "%" + c.charCodeAt(0).toString(16).toUpperCase());

export const bugUrl = () => "https://github.com/snowztech/onecorp/issues/new?labels=bug&body=" + enc(
  `**What happened**\n\n\n**What you expected**\n\n\n**Steps to get there**\n1. \n\n---\nOneCorp ${version || "unknown version"} on macOS`);

export let isBooted = false;

export function booted() {
  if (isBooted) return;
  isBooted = true;
  const app = document.querySelector(".app");
  app?.classList.remove("booting"); app?.classList.add("arrive");
  document.getElementById("boot")?.remove();
}

setTimeout(booted, 4000);

export const S = { companies: [], company: null, snap: null, last: "", view: "company", drawer: null, person: null, addingDept: false, open: {}, channel: "general", chat: null, seen: {}, diffs: {}, wiki: null, hbEdit: false, setup: null, founder: undefined, profile: null, project: null, rail: false, needs: {}, pop: false, settings: null, health: null, confirm: null, sheet: null, shown: "" };

export async function call(cmd, args, done) {
  try {
    const r = await invoke(cmd, args);
    if (done) toast(done);
    await refresh(true);
    return r;
  } catch (e) {
    toast(String(e), true);
    throw e;
  }
}

export const mergeLabel = () => (S.snap?.ship?.forge ? `Merge ${S.snap.ship.pr_word}` : "Approve and merge");

export const myName = () => S.founder?.name || "You";

export const byId = id => (S.snap?.employees || []).find(e => e.id === id);
