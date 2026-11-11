import { $, esc, money } from "./lib.ts";
import { badge, icon, me } from "./look.ts";
import { S, myName, unread } from "./state.ts";
import { updateReady } from "./update.ts";

// The frame around every page: sidebar, top bar and your menu.
export const waiting = () => (S.snap ? S.snap.inbox.length + S.snap.reviews.length + (S.snap.failed || []).length : 0);

/* Sidebar and top bar */
export function toggleRail(on = !S.rail) {
  S.rail = on;
  $(".app").classList.toggle("rail", on);
  try { localStorage.setItem("onecorp.rail", on ? "1" : ""); } catch {}
  renderSide();
}

export function renderSide() {
  const n = waiting(), pages = { company: "Company", board: "Board", inbox: "Inbox", chat: "Chat", wiki: "Wiki" };
  const open = (ch: string) => S.view === "chat" && S.channel === ch;
  const anyUnread = [...(S.snap?.channels || []).map(c => [c.name, c.last] as const), ...(S.snap?.dms || []).map(d => [d.channel, d.last] as const)].some(([ch, last]) => unread(ch, last) && !open(ch));
  $("#side").innerHTML = `<div class="side-drag" data-tauri-drag-region></div>
    <div class="switch"><button class="co-switch" data-switch="1" aria-expanded="${!!S.switching}">${S.company ? `${badge(S.company, S.looks[S.company ?? ""])}<span class="coname">${esc(S.company)}</span>` : `<span class="coname">Companies</span>`}${Object.entries(S.needs).some(([c, n]) => c !== S.company && n) ? '<i class="dot" title="Another company needs you"></i>' : ""}<span class="chev">${icon("chevrons")}</span></button>
      ${S.switching ? `<div class="menu" role="menu">${S.companies.map(c => `<button class="cobtn" role="menuitem" data-co="${esc(c)}" aria-current="${c === S.company}">${badge(c, S.looks?.[c])}<span class="coname">${esc(c)}</span>${c === S.company ? '<span class="tick">✓</span>' : S.needs[c] ? `<span class="badge">${S.needs[c]}</span>` : ""}</button>`).join("")}
        <hr><button class="cobtn" role="menuitem" data-new-co="1"><span class="logo add">+</span><span class="coname">New company</span></button></div>` : ""}</div>
    ${S.company ? `${Object.entries(pages).map(([k, v]) =>
      `<button class="navbtn" data-view="${k}" title="${v}" ${S.view === k ? 'aria-current="page"' : ""}>${icon(k)}<span>${v}</span>${k === "inbox" && n ? `<span class="badge">${n}</span>` : ""}${k === "chat" && anyUnread ? `<i class="dot" title="New messages"></i>` : ""}</button>`).join("")}` : ""}
    ${updateReady ? updateButton(updateReady) : ""}
    ${S.dev ? `<span class="dev-chip" title="A dev build, on ${esc(S.dev)}">DEV · ${esc(S.dev.replace(/^\/Users\/[^/]+/, "~"))}</span>` : ""}
    <button class="side-foot" data-pop="menu" aria-expanded="${!!S.pop}" title="You and settings">${me()}<div><b>${esc(myName())}</b><span>Founder</span></div><span class="foot-more">${icon("more")}</span></button>`;
}

// Restarting stops runs in progress, so the button says so when someone is working.
function updateButton(version: string) {
  const busy = (S.snap?.employees || []).filter(e => e.status === "working").length;
  const title = busy ? `${busy === 1 ? "1 person is" : `${busy} people are`} working. Their runs stop, and you can retry them after.` : "Restarts OneCorp on the new version.";
  return `<button class="update-btn" data-update-restart="1" title="${esc(title)}">${icon("spark")}<span>Restart to update</span><em>${esc(version)}</em></button>`;
}

export function renderTop() {
  const s = S.snap;
  $("#title").textContent = S.profile ? (S.founder ? "Your profile" : "Welcome") : S.setup ? "New company" : { inbox: "Inbox", wiki: "Wiki", board: "Board", chat: "Chat" }[S.view] || (S.company ? "Company" : "OneCorp");
  $("#new-ticket").hidden = !!S.setup || !!S.profile || !s;
  // Today's spend, a glance from every page: the amount, a meter, the budget. Words only on hover.
  const sp = $("#top-spend");
  sp.hidden = $("#new-ticket").hidden;
  if (!sp.hidden && s) {
    const b = s.spend.budget, pct = b > 0 ? Math.min(100, (s.spend.today / b) * 100) : 0;
    sp.className = `top-spend ${pct >= 100 ? "over" : pct >= 80 ? "near" : ""}`;
    sp.title = b > 0 ? `Spent today, of a ${money(b)} daily budget` : "Spent today. Set a daily budget in Spend";
    sp.innerHTML = `<span>${money(s.spend.today)}</span>${b > 0 ? `<span class="meter"><i style="width:${pct}%"></i></span><span class="of">${money(b)}</span>` : ""}`;
  }
}

export function renderPop() {
  const p = $("#pop");
  p.hidden = !S.pop;
  $(".side-foot")?.setAttribute("aria-expanded", String(!!S.pop));
  if (!S.pop) return;
  if (S.pop === "menu") {
    p.innerHTML = `<div class="pop-menu"><div class="pm-head">${me()}<div><b>${esc(myName())}</b><span>Founder</span></div></div>
      ${S.spendAll !== undefined && S.companies.length > 1 ? `<div class="pm-note">${money(S.spendAll)} spent today across your ${S.companies.length} companies</div>` : ""}
      <button class="pm-item" data-settings-tab="you">${icon("user")}Edit profile</button>
      ${S.company ? `<button class="pm-item" data-settings="1">${icon("settings")}Settings<kbd>⌘,</kbd></button>` : ""}
      ${S.company ? `<button class="pm-item" data-ask="1">${icon("info")}Ask OneCorp</button>` : ""}
      <hr><button class="pm-item quiet" data-pop="forget">${icon("logout")}Forget my profile…</button></div>`;
    return;
  }
  if (S.pop === "forget") {
    p.innerHTML = `<div class="pop-form"><b>Forget your profile?</b><p class="fine">OneCorp forgets your name and photo and asks again. Your companies stay.</p>
      <div class="acts end"><button class="btn" data-pop="menu">Cancel</button><button class="btn danger" data-forget="1">Forget me</button></div></div>`;
    return;
  }
}
