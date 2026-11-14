import { renderDrawer } from "./drawers.ts";
import { $, esc, money, toast } from "./lib.ts";
import { AGENTS, icon } from "./look.ts";
import { alertNew, watchAll } from "./notify.ts";
import { renderSide, renderTop } from "./shell.ts";
import { S, booted, invoke, seenFirstTime, settling } from "./state.ts";
import { viewBoard } from "./views/board.ts";
import { viewChat } from "./views/chat.ts";
import { viewCompany } from "./views/company.ts";
import { viewInbox } from "./views/inbox.ts";
import { startSetup, viewProfile, viewSetup, viewWelcome } from "./views/onboarding.ts";
import { markWiki, viewWiki } from "./views/wiki.ts";

// Drawing the current page, and polling the company snapshot while you're in a workspace.
/* Rendering and polling */
export let pollTimer: ReturnType<typeof setInterval> | null = null, ticks = 0;

export function syncPolling() {
  const inWorkspace = S.company && !S.profile && !S.setup;
  if (!inWorkspace && pollTimer !== null) {
    clearInterval(pollTimer);
    pollTimer = null;
  } else if (inWorkspace && pollTimer === null) {
    pollTimer = setInterval(() => { refresh(false); if (ticks++ % 4 === 0) watchAll(); }, 1500);
  }
}

let enterTimer: ReturnType<typeof setTimeout> | undefined;
export function render() {
  syncPolling();
  booted();
  // Setup actions save the form themselves before changing it. Reading it again here would copy
  // stale fields over the change: the old team over a new template, or a row over the one deleted.
  if (S.profile && $("#pf-name")) S.profile.name = $("#pf-name").value;
  renderSide(); renderTop();
  const v = $("#view");
  const banner = $("#paused");
  const limits = S.snap?.limits || [];
  banner.hidden = !((S.snap?.paused || limits.length) && !S.profile && !S.setup);
  // A plan limit isn't something you can fix from here, so it's said plainly, with no button.
  if (!banner.hidden && !S.snap!.paused) banner.innerHTML = `<span>${icon("pause")}<b>${limits.map(l => `${esc(AGENTS[l.runtime]?.name || l.runtime)} reached its plan's usage limit`).join(", ")}.</b> That work waits and starts again by itself at ${esc(limits.map(l => l.back).join(" and "))}.</span>`;
  else if (!banner.hidden) banner.innerHTML = S.snap!.spend?.paused_by_budget
    ? `<span>${icon("pause")}<b>${esc(S.company)} reached its ${money(S.snap!.spend.budget)} daily budget.</b> Running work finishes, and nothing new starts until tomorrow.</span><button class="btn" data-settings-tab="spend">Raise budget</button><button class="btn" data-keep-going="1">Keep going today</button>`
    : `<span>${icon("pause")}<b>${esc(S.company)} is paused.</b> Running work finishes, nothing new starts, and nobody replies in chat.</span><button class="btn" data-pause="0">Resume</button>`;
  // Setting up is full screen: no sidebar or top bar for things that don't exist yet.
  $(".app").classList.toggle("focus", !!(S.welcome || S.profile || S.setup));
  v.classList.toggle("flush", !S.profile && !S.setup && S.view === "chat");
  const where = S.welcome ? "welcome" : S.profile ? "profile" : S.setup ? "setup-" + S.setup.step : `${S.company}/${S.view}`;
  if (where !== S.shown) {
    S.shown = where;
    v.classList.remove("enter"); void v.offsetWidth; v.classList.add("enter");
    clearTimeout(enterTimer); enterTimer = setTimeout(() => v.classList.remove("enter"), 400);
  }
  if (S.welcome) {
    v.innerHTML = viewWelcome();
  } else if (S.profile) {
    v.innerHTML = viewProfile();
  } else if (S.setup) {
    v.innerHTML = viewSetup();
  } else if (!S.company) {
    v.innerHTML = `<div class="empty-state"><b>No company yet</b>Opening the new company flow…</div>`;
  } else if (!S.snap) {
    v.innerHTML = `<p class="muted">Opening ${esc(S.company)}…</p>`;
  } else {
    const views: Record<string, () => string> = { inbox: viewInbox, wiki: viewWiki, board: viewBoard, chat: viewChat };
    const chat = keepChatScroll();
    v.innerHTML = views[S.view]?.() ?? viewCompany();
    chat();
    if (S.view === "wiki") markWiki();
  }
  renderDrawer();
}

// Chat stays where you are across a redraw: at the latest message if you were there, even as photos
// and replies arrive after it, or where you scrolled up to read. A new channel opens at the bottom.
let chatAt = "";
// Sending a message brings you to the end, wherever you were.
export const chatToEnd = () => { chatAt = ""; };
function keepChatScroll() {
  const old = document.querySelector<HTMLElement>("#msgs"), same = chatAt === S.channel;
  const atBottom = !old || !same || old.scrollHeight - old.scrollTop - old.clientHeight < 60, top = old?.scrollTop ?? 0;
  return () => {
    const m = document.querySelector<HTMLElement>("#msgs");
    chatAt = m ? S.channel : "";
    if (!m) return;
    if (!atBottom) { m.scrollTop = top; return; }
    const pin = () => (m.scrollTop = m.scrollHeight);
    pin();
    m.querySelectorAll("img").forEach(img => img.complete || img.addEventListener("load", pin, { once: true }));
  };
}

export function typing() {
  const a = document.activeElement as HTMLInputElement | null;
  return !!a && a.matches("input, textarea") && !!a.closest("#view, #drawer") && a.value.trim() !== "";
}

export async function refresh(force: boolean) {
  if (!S.voices) S.voices = await invoke("voices").catch(() => null);
  if (!S.roles) S.roles = await invoke("roles").catch(() => null);
  if (!S.models) S.models = await invoke("models").catch(() => null);
  // Ignore a workspace tick already queued when onboarding stopped the timer.
  if (!force && (S.profile || S.setup)) return;
  try {
    if (S.founder === undefined) S.founder = await invoke("founder");
    if (!S.runtimes) S.runtimes = (await invoke("setup_info").catch(() => null))?.runtimes || null;
    // A first launch says hello before anything else: who you are, then your first company.
    if (S.founder === null && !S.profile && !S.setup) S.profile = { name: "" };
    const c = await invoke("companies");
    S.companies = c.list; S.looks = c.looks || {}; S.dev = c.dev || null;
    // The very first launch opens on a welcome. Anyone with a company goes straight in.
    if (S.welcome === undefined) S.welcome = S.founder === null && !c.list.length;
    if (!S.company || !c.list.includes(S.company)) S.company = c.current;
    if (!S.company) {
      S.snap = null;
      if (!S.setup && !S.profile) return startSetup(); // No company yet: create one.
      // Keep mounted onboarding inputs intact during polling, including empty inputs.
      const where = S.profile ? "profile" : "setup-" + S.setup!.step;
      if (!force && S.shown === where) return;
      return render();
    }
    const snap = await invoke("snapshot", { company: S.company });
    // The wiki is a small file and can change outside the app, so it's read with every refresh.
    const book = S.view === "wiki" ? await invoke("wiki", { company: S.company }) : S.wiki;
    if (S.view === "chat") S.chat = await invoke("chat", { company: S.company, channel: S.channel }).catch(() => S.chat);
    const json = JSON.stringify(snap) + JSON.stringify(book) + (S.view === "chat" ? JSON.stringify(S.chat) : "");
    if (!force && (json === S.last || typing() || settling() || S.profile || S.setup)) return;
    S.snap = snap; S.last = json; S.wiki = book;
    alertNew(snap);
    for (const ch of snap.channels || []) seenFirstTime(ch.name, ch.last);
    for (const dm of snap.dms || []) seenFirstTime(dm.channel, dm.last);
    if (S.drawer?.type === "person") S.person = await invoke("person", { company: S.company, id: S.drawer.id }).catch(() => S.person);
    if (S.drawer?.type === "ticket") S.ticket = await invoke("ticket_detail", { company: S.company, key: S.drawer.key }).catch(() => S.ticket);
    render();
  } catch (e) {
    booted();
    toast(String(e), true);
    // A failed load still leaves the workspace polling, so it recovers on its own.
    syncPolling();
  }
}
