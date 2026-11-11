import { toast } from "./lib.ts";
import type { Employee, Returns, State } from "./types.ts";
import { preview } from "./preview.ts";
import { refresh, render } from "./render.ts";
import { renderSheet } from "./views/settings.ts";

// What the app knows (S), and how it talks to the Tauri side.
// Every command goes through here. Its result is typed from the command name, in types.ts.
export const invoke = (window.__TAURI__ ? window.__TAURI__.core.invoke : preview) as <K extends string>(cmd: K, args?: Record<string, unknown>) => Promise<Returns<K>>;

// The app's version, from Tauri in the app and the sample in the browser preview.
export let version = "";

(window.__TAURI__?.app?.getVersion?.() || Promise.resolve("0.1.0")).then((v: string) => { version = v; if (isBooted && (S.welcome || S.sheet)) { render(); renderSheet(); } }).catch(() => {});

// A bug report on GitHub, with the version already in it, so nobody has to look it up.
export const enc = (t: string) => encodeURIComponent(t).replace(/[!'()*]/g, c => "%" + c.charCodeAt(0).toString(16).toUpperCase());

export const bugUrl = () => "https://github.com/snowztech/onecorp/issues/new?labels=bug&body=" + enc(
  `**What happened**\n\n\n**What you expected**\n\n\n**Steps to get there**\n1. \n\n---\nOneCorp ${version || "unknown version"} on macOS`);

// The layout appears once the first render has decided it. If loading stalls, show it anyway.
export let isBooted = false;

export function booted() {
  if (isBooted) return;
  isBooted = true;
  const app = document.querySelector(".app");
  app?.classList.remove("booting"); app?.classList.add("arrive");
  document.getElementById("boot")?.remove();
}

setTimeout(booted, 4000);

export const S: State = { companies: [], company: null, looks: {}, dev: null, ticket: null, snap: null, last: "", view: "company", drawer: null, person: null, addingDept: false, open: {}, channel: "general", chat: null, seen: {}, diffs: {}, wiki: null, hbEdit: false, setup: null, founder: undefined, profile: null, project: null, rail: false, needs: {}, pop: false, settings: null, health: null, confirm: null, sheet: null, shown: "" };

export async function call<K extends string>(cmd: K, args: Record<string, unknown>, done?: string): Promise<Returns<K>> {
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

export const byId = (id: string | null | undefined): Employee | undefined => (S.snap?.employees || []).find(e => e.id === id);
// The people behind a list of ids, skipping anyone who has left.
export const employeesOf = (ids: string[]) => ids.flatMap(id => byId(id) ?? []);
