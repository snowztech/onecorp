import { first, money } from "./lib.js";
import { renderSide } from "./shell.js";
import { S, invoke } from "./state.js";

// What needs you, across every company: the dock badge, the menu bar count and notifications.
/* Everything waiting on you, most urgent first. The Company page, the dock badge and notifications all use it. */
export function needItems(s) {
  return [
    ...s.reviews.filter(r => r.kind === "plan").map(r => ({ to: r.key, name: r.name, text: `planned ${r.title}. Review the plan` })),
    ...s.inbox.filter(q => q.blocking).map(q => ({ to: "q" + q.id, name: q.name, text: q.kind === "command" ? `wants to run ${q.question}` : q.question })),
    ...(s.failed || []).map(t => ({ to: t.key, name: t.name, text: `${t.key} stopped. Retry it?` })),
    ...s.reviews.filter(r => r.kind !== "plan").map(r => ({ to: r.key, name: r.name, text: `${r.title} is ready for review` })),
    ...s.inbox.filter(q => !q.blocking).map(q => ({ to: "q" + q.id, name: q.name, text: q.kind === "wiki" ? `proposes: ${q.question}` : q.question }))];
}

// Your team works while you're elsewhere, so the dock shows how much waits on you, and anything new
// gets a notification when the app isn't in front. What was already waiting at launch stays quiet.
export const notified = {};

export function alertNew(snap) {
  if (!window.__TAURI__) return;
  const items = needItems(snap);
  S.needs[snap.company] = items.length;
  const seen = notified[snap.company];
  notified[snap.company] = new Set(items.map(i => i.to));
  // The company you're looking at needs no pop-up while the app is in front.
  if (seen && !document.hasFocus()) for (const i of items.filter(i => !seen.has(i.to)).slice(0, 3)) notify(`${first(i.name)} · ${snap.company}`, i.text);
  budgetAlert(snap.company, snap.spend);
  badgeAll();
}

// Each budget moment is said once a day: the 80% heads-up, then the pause.
export const budgetSaid = {};

export function budgetAlert(company, sp) {
  if (!sp?.budget) return;
  const was = budgetSaid[company], now = sp.paused_by_budget ? "paused" : sp.warn ? "warn" : "";
  budgetSaid[company] = now;
  if (was === undefined || now === was || !now) return;
  notify(`${company} · budget`, now === "paused" ? `Reached the ${money(sp.budget)} daily budget. Nothing new starts until tomorrow.` : `80% of today's ${money(sp.budget)} budget is spent.`);
}

// Every company, every few seconds: the badge counts them all, and a question in another company
// gets a notification even when you're in this one.
export async function watchAll() {
  const all = await invoke("attention").catch(() => null);
  if (!all) return;
  for (const a of all) {
    if (a.company === S.company) continue;
    const items = needItems(a), seen = notified[a.company];
    notified[a.company] = new Set(items.map(i => i.to));
    S.needs[a.company] = items.length;
    if (seen) for (const i of items.filter(i => !seen.has(i.to)).slice(0, 3)) notify(`${first(i.name)} · ${a.company}`, i.text);
    budgetAlert(a.company, a.spend);
  }
  S.spendAll = all.reduce((t, a) => t + (a.spend?.today || 0), 0) + (S.snap?.spend?.today || 0);
  badgeAll(); renderSide();
}

export function badgeAll() {
  const total = Object.values(S.needs).reduce((a, b) => a + b, 0);
  window.__TAURI__?.window?.getCurrentWindow?.().setBadgeCount(total || undefined).catch(() => {});
  // The same count next to the menu bar icon, for when the window is closed.
  if (window.__TAURI__) invoke("set_tray_count", { count: total }).catch(() => {});
}

export async function notify(title, body) {
  const n = window.__TAURI__?.notification;
  if (!n) return;
  try {
    let ok = await n.isPermissionGranted();
    if (!ok) ok = (await n.requestPermission()) === "granted";
    if (ok) n.sendNotification({ title, body });
  } catch {}
}
