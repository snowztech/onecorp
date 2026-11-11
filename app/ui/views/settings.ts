import { logo } from "../brand.ts";
import { $, esc, first, money, tilde, toast } from "../lib.ts";
import { AGENTS, FACE_SETS, PALETTE, agentMark, badge, drawFace, face, faceSet, icon, me } from "../look.ts";
import { renderPop } from "../shell.ts";
import { S, bugUrl, invoke, version } from "../state.ts";

/* Settings: a sheet over the app, opened from your menu or ⌘,. The company, and whether everything
   OneCorp needs is in place. Your own profile lives in your menu. */
/* Settings: sections on the left, the section on the right, and changes save as you make them. */
export const SECTIONS = [["company", "Company", "company"], ["spend", "Spending", "coin"], ["agents", "Integrations", "spark"], ["appearance", "Appearance", "eye"], ["you", "You", "user"], ["about", "About", "info"]];

export function renderSheet() {
  const el = $("#sheet");
  el.hidden = !S.sheet;
  if (!S.sheet) return (el.innerHTML = "");
  const tab = SECTIONS.some(([k]) => k === S.sheet) ? S.sheet : "company";
  const sections: Record<string, () => string> = { company: companySection, spend: spendTab, agents: agentsSection, appearance: appearanceSection, you: youSection, about: aboutSection };
  const body = sections[tab]();
  el.innerHTML = `<div class="sheet settings-sheet" role="dialog" aria-modal="true" aria-label="Settings">
    <nav class="set-nav"><h2>Settings</h2>${SECTIONS.map(([k, n, ic]) => `<button data-sheet="${k}" aria-current="${k === tab}">${icon(ic)}${n}</button>`).join("")}</nav>
    <div class="set-main"><header class="set-h"><h3>${SECTIONS.find(([k]) => k === tab)?.[1]}</h3><button class="close" data-sheet-close="1" aria-label="Close">×</button></header>
      <div class="sheet-body">${body}</div></div></div>`;
}

export const saved = () => toast("Saved");

export const theme = () => { try { return localStorage.getItem("onecorp.theme") || "system"; } catch { return "system"; } };

export function setTheme(t: string) {
  try { t === "system" ? localStorage.removeItem("onecorp.theme") : localStorage.setItem("onecorp.theme", t); } catch {}
  if (t === "system") delete document.documentElement.dataset.theme; else document.documentElement.dataset.theme = t;
  // The title bar follows too, so the traffic lights sit on the right colour.
  window.__TAURI__?.window?.getCurrentWindow().setTheme(t === "system" ? null : t as "light" | "dark").catch(() => {});
}

export function companySection() {
  const st = S.settings;
  if (!st) return `<p class="muted">Opening…</p>`;
  const look = S.looks[S.company ?? ""] || {};
  const modes = [["local", "On this Mac"], ["github", "GitHub"], ["gitlab", "GitLab"]].filter(([m]) => m === "local" || m === st.forge || m === st.ship);
  const reveal = (path: string) => window.__TAURI__?.opener ? `<button class="link-btn" data-reveal="${esc(path)}">Show in Finder</button>` : "";
  const del = S.confirm === "delete"
    ? `<form class="confirm" id="del-co"><p>This deletes <b>${esc(st.name)}</b>: ${st.people} ${st.people === 1 ? "person" : "people"}, ${st.tickets} tickets, the chat, wiki and memory. Your repo and its branches stay as they are.</p>
        <div class="su-row"><input id="del-name" placeholder="Type ${esc(st.name)} to confirm" autocomplete="off" spellcheck="false"><button class="btn danger">Delete</button><button class="btn ghost" type="button" data-confirm="">Cancel</button></div></form>`
    : `<div class="row-set"><div><b>Delete ${esc(st.name)}</b><span>Your repo is never touched.</span></div><button class="btn danger-ghost" data-confirm="delete">Delete…</button></div>`;
  return `<div class="row-set"><div class="who-row">${badge(st.name, look, "lg")}<div><b>Logo</b></div></div>
      <div class="look-row">${PALETTE.map(c => `<button type="button" class="swatch" data-look-color="${c}" aria-pressed="${!look.logo && c === look.color}" style="--c:${c}" aria-label="Color ${c}"></button>`).join("")}
        ${window.__TAURI__?.dialog ? `<button class="link-btn" data-look-logo="1">${look.logo ? "Change image" : "Image…"}</button>` : ""}</div></div>
    <div class="row-set"><div><b>Finished work</b><span>${st.ship === "local" ? "Merged on this Mac" : "Opened as pull requests"}</span></div>
      <div class="seg">${modes.map(([m, n]) => `<button data-ship="${m}" aria-pressed="${m === st.ship}">${n}</button>`).join("")}</div></div>
    <div class="row-set"><div><b>Commits</b><span>${st.git_you
        ? (st.commits === "you" ? `Every commit is yours, as ${esc(st.git_you.name)}` : `The employee writes it, you're credited too`)
        : "By the employee. Set your name in git (git config user.name and user.email) to commit as you."}</span></div>
      <div class="seg">${[["team", "As your team"], ["you", "As you"]].map(([m, n]) => `<button data-commits="${m}" aria-pressed="${m === (st.commits || "team")}" ${!st.git_you && m === "you" ? "disabled" : ""}>${n}</button>`).join("")}</div></div>
    ${st.ship === "local" ? "" : `<div class="row-set"><div><b>Pull requests</b><span>${st.pr_sign === false ? "Just the summary" : `End with “Jonas Berg, Engineer at ${esc(st.name)} · made with OneCorp”`}</span></div>
      <div class="seg">${[[true, "Signed"], [false, "Plain"]].map(([on, n]) => `<button data-pr-sign="${on}" aria-pressed="${on === (st.pr_sign !== false)}">${n}</button>`).join("")}</div></div>`}
    <div class="sec-h"><h3 class="label">Repos</h3>${window.__TAURI__?.dialog ? `<button class="btn sm" type="button" data-repo-add="pick">Add a repo…</button>` : ""}</div>
    <p class="sec-sub">Where your team works, and the commands they can run there. Each command covers what follows it, so <code>make</code> allows every target.</p>
    ${(st.repos || []).map(r => `<div class="repo-card">
      <div class="repo-top"><b>${esc(r.name)}</b>${r.main ? `<span class="pill-state">primary</span>` : ""}<span class="mono muted repo-path" title="${esc(r.path)}">${esc(tilde(r.path))}</span>
        <span class="repo-acts">${reveal(r.path)}${r.main ? "" : `<button class="link-btn" data-repo-rm="${esc(r.name)}">Remove</button>`}</span></div>
      <div class="cmd-list">${r.commands.map(cmd => `<span class="cmd-chip">${esc(cmd)}<button type="button" data-cmd-del="${esc(r.name)}" data-cmd="${esc(cmd)}" aria-label="Remove ${esc(cmd)}">×</button></span>`).join("")}
        <input class="cmd-add" data-cmd-add="${esc(r.name)}" placeholder="${r.commands.length ? "Add a command" : "Add a command, like npm test"}" autocomplete="off" spellcheck="false" aria-label="Add a command for ${esc(r.name)}"></div>
      <div class="repo-foot">${r.custom ? `<span>Your list</span><button class="link-btn" type="button" data-commands-reset="${esc(r.name)}">Reset to suggested</button>` : `<span>Suggested from the repo's files</span>`}</div></div>`).join("")}
    ${window.__TAURI__?.dialog && !S.open["repo-path"] ? `<button class="link-btn type-path" type="button" data-more="repo-path">Or type a repo's path</button>`
      : `<div class="su-row add-repo"><input id="repo-path" class="mono" placeholder="~/dev/your-api" autocomplete="off" spellcheck="false"><button class="btn" type="button" data-repo-add="path">Add repo</button></div>`}
    <div class="row-set data-row"><div><b>Company data</b><span class="mono">${esc(tilde(st.data))}</span></div>${reveal(st.data)}</div>
    <h3 class="label danger-label">Danger zone</h3>${del}`;
}

// What fixes a tool: a button that opens Terminal running the command, and the command itself to copy.
export const fixRow = (fix: string, state: string) => `<div class="fix"><button class="btn sm" type="button" data-term="${esc(fix)}">${state === "signin" ? "Sign in" : "Install"}</button><code>${esc(fix)}</code><button class="link-btn" type="button" data-copy="${esc(fix)}">Copy</button></div>`;

export function agentsSection() {
  const r = S.ready, find = (w: string) => (r || []).find(h => h.what === w);
  const tools: [string, string][] = [["Claude Code", "claude"], ["Codex", "codex"], ["GitHub CLI", "gh"]];
  const card = ([what, key]: [string, string]) => {
    const h = find(what), state = !r ? "checking" : !h ? "missing" : h.level === "ok" ? "ok" : /not signed in/.test(h.detail) ? "signin" : "missing";
    const pill = { checking: "Checking…", ok: "Connected", signin: "Not signed in", missing: "Not installed" }[state];
    return `<div class="tool t-${state}"><span class="tool-ic" style="--c:${AGENTS[key]?.color || "#18181b"}">${agentMark(key)}</span>
      <div class="tool-main"><b>${esc(what)}</b>${h?.fix && state !== "ok" ? fixRow(h.fix, state) : `<span>${esc(h?.detail || "")}</span>`}</div>
      <span class="pill-state">${pill}</span></div>`;
  };
  const codex = S.spend?.prices?.find(p => p.runtime === "codex")?.price ?? 5;
  const mark = { ok: "✓", warn: "!", fail: "×" };
  const others = (S.health || []).filter(h => !tools.some(([w]) => w === h.what));
  return `<h3 class="label tools-h">AI agents</h3><div class="tools">${tools.slice(0, 2).map(card).join("")}</div>
    <div class="row-set"><div><b>Codex cost</b><span>Codex doesn't report a cost, so OneCorp estimates it from tokens.</span></div>
      <label class="money-in inline"><span>$</span><input data-price="codex" type="number" min="0" step="0.5" value="${codex}" aria-label="Dollars per million Codex tokens"><span class="unit">/ 1M tokens</span></label></div>
    <h3 class="label tools-h">Code hosting</h3><div class="tools">${tools.slice(2).map(card).join("")}</div>
    <h3 class="label tools-h">Health</h3>
    <div class="row-set"><div><b>Everything else</b><span>Git, your repos and each company.</span></div><button class="link-btn" data-health="1">${S.health ? "Check again" : "Check"}</button></div>
    ${others.length ? `<ul class="health">${others.map(h => `<li class="hc lvl-${h.level}"><i>${mark[h.level]}</i><div><b>${esc(h.what)}</b><span>${esc(h.detail)}</span>${h.fix && h.level !== "ok" ? `<code>${esc(h.fix)}</code>` : ""}</div></li>`).join("")}</ul>` : ""}`;
}

// Who made it, which version, and where to go next. The About pane of a Mac app.
export function aboutSection() {
  const repo = "https://github.com/snowztech/onecorp";
  return `<div class="about">
    <span class="hero-mark">${logo()}</span>
    <h2>OneCorp</h2>
    <p class="muted">Run a company of AI employees, on your Mac.</p>
    ${version ? `<button class="version-chip" data-copy="${esc(version)}" title="Copy the version">Version ${esc(version)} ${icon("copy")}</button>` : ""}
    <div class="about-links"><button class="link-btn" data-open-url="${repo}">GitHub</button><button class="link-btn" data-open-url="${repo}/blob/main/CHANGELOG.md">Changelog</button><button class="link-btn" data-open-url="${esc(bugUrl())}">Report an issue</button><button class="link-btn" data-open-url="https://onecorp.dev">onecorp.dev</button></div>
    <p class="fine">Open source, MIT. © 2026 Lucas Neves Pereira, <button class="link-btn inline" data-open-url="https://github.com/snowztech">snowztech</button></p>
  </div>`;
}

export function appearanceSection() {
  const team = (S.snap?.employees || []).slice(0, 5).map(e => e.face || e.name);
  const people = team.length ? team : ["Odile Martin", "Maya Okafor", "Tomás Reis", "Rui Alves", "Lina Park"];
  const cur = faceSet();
  return `<div class="row-set"><div><b>Theme</b><span>Follows your Mac unless you pick one.</span></div>
      <div class="seg">${[["system", "System"], ["light", "Light"], ["dark", "Dark"]].map(([k, n]) => `<button data-theme-pick="${k}" aria-pressed="${k === theme()}">${n}</button>`).join("")}</div></div>
    <div class="row-set stack"><div><b>Faces</b><span>How your team looks everywhere. Photos you picked stay.</span></div>
      <div class="face-sets">${FACE_SETS.map(([k, n, d]) => `<button type="button" class="face-set" data-face-set="${k}" aria-pressed="${k === cur}">
        <span class="fs-faces">${people.map(p => drawFace(p, "neutral", k)).join("")}</span><b>${n}</b><span>${d}</span></button>`).join("")}</div></div>`;
}

export function youSection() {
  const f = S.founder;
  const forget = S.confirm === "forget"
    ? `<div class="confirm"><p>OneCorp forgets your name and photo and asks again. Your companies stay.</p><div class="acts"><button class="btn danger" data-forget="1">Forget me</button><button class="btn ghost" data-confirm="">Cancel</button></div></div>`
    : `<div class="row-set"><div><b>Forget your profile</b><span>Your companies stay.</span></div><button class="btn danger-ghost" data-confirm="forget">Forget…</button></div>`;
  const pic = window.__TAURI__?.dialog ? `<button type="button" class="pf-pic pick-pic" data-you-photo="1" title="Change photo">${me()}</button>` : `<span class="pf-pic">${me()}</span>`;
  return `<form id="you-form" class="row-set you-form"><div class="who-row">${pic}<div class="field"><label for="pf-name">Your name</label><input id="pf-name" value="${esc(f?.name || "")}" autocomplete="off"><span class="fine">Your team knows you by this name.</span></div></div>
      ${f?.photo ? `<button class="link-btn" type="button" data-you-nophoto="1">Remove photo</button>` : ""}</form>
    <h3 class="label danger-label">Danger zone</h3>${forget}`;
}

export async function openSettings(tab = "company") {
  const opening = !S.sheet;
  S.sheet = tab === "health" ? "agents" : tab; S.confirm = null; S.customBudget = false; S.pop = false; renderPop(); renderSheet();
  // The entrance plays once, when the sheet opens, not each time its content changes.
  if (opening) { const el = $("#sheet"); el.classList.add("opening"); setTimeout(() => el.classList.remove("opening"), 300); }
  await loadSection(S.sheet);
}

export async function loadSection(tab: string) {
  // Loaded fresh for the company you're on: a switch since last time must never show the old one.
  if (S.settings?.name !== S.company) { S.settings = null; renderSheet(); try { S.settings = await invoke("settings", { company: S.company }); } catch (e) { toast(String(e), true); } }
  if (tab === "spend" || tab === "agents") await loadSpend();
  if (tab === "agents" && !S.ready) S.ready = await invoke("readiness").catch(() => []);
  renderSheet();
}

export async function loadSpend() {
  try { S.spend = await invoke("spend_report", { company: S.company }); } catch (e) { toast(String(e), true); }
  renderSheet();
}

/* Spend: today against the budget, two weeks of days, and where it goes, by person, project and agent. */
export function spendTab() {
  const r = S.spend;
  if (!r) return `<p class="muted">Adding it up…</p>`;
  const t = r.totals, b = r.budget, pct = b > 0 ? Math.min(100, (t.today.cost / b) * 100) : 0;
  const any = r.days.filter(d => d.cost > 0).length >= 2, top = Math.max(...r.days.map(d => d.cost), b || 0, 0.01);
  const today = r.days[r.days.length - 1]?.day;
  const presets = [0, 5, 10, 25, 50], custom = b > 0 && !presets.includes(b);
  const row = (lead: string, w: { today: { cost: number }; week: { cost: number }; all: { cost: number } }, extra = "") => `<tr><td>${lead}</td><td class="num">${money(w.today.cost)}</td><td class="num">${money(w.week.cost)}</td><td class="num">${money(w.all.cost)}</td>${extra}</tr>`;
  const head = (extra = "") => `<thead><tr><th></th><th class="num">Today</th><th class="num">7 days</th><th class="num">All time</th>${extra}</tr></thead>`;
  const people = r.people.filter(p => p.all.cost > 0 || p.cap > 0);
  const limits = (r.limits || []).map(l => `<p class="plan-line"><span class="tool-ic sm" style="--c:${AGENTS[l.runtime]?.color || "#888"}">${agentMark(l.runtime)}</span><span><b>${esc(AGENTS[l.runtime]?.name || l.runtime)}</b> hit its plan's usage limit ${l.times === 1 ? "once" : `${l.times} times`} in the last 7 days${l.elsewhere.length ? `, including in ${esc(l.elsewhere.join(" and "))}, which shares your plan` : ""}. Work waited and picked back up on its own.</span></p>`).join("");
  const by = S.spendBy || "people", tabs: [string, string][] = [["people", "People"], ["kind", "Kind"], ["projects", "Projects"], ["agents", "Agents"]];
  const kinds = r.kinds || [];
  return `<div class="spend-now"><span class="label" title="On a subscription, what the same work would cost on the API. On an API key, what you pay.">Today, at API prices</span><b>${money(t.today.cost)}</b>
      ${b > 0 ? `<span class="meter big ${pct >= 100 ? "over" : pct >= 80 ? "near" : ""}"><i style="width:${pct}%"></i></span><span class="muted">${Math.round(pct)}% of ${money(b)}</span>` : ""}</div>
    <div class="row-set"><div><b>Daily budget</b><span>The company pauses there until tomorrow.</span></div>
      <div class="seg">${presets.map(p => `<button data-budget="${p}" aria-pressed="${p === b && !S.customBudget}">${p ? `$${p}` : "None"}</button>`).join("")}<button data-budget-custom="1" aria-pressed="${custom || !!S.customBudget}">${custom ? money(b) : "Other"}</button></div></div>
    ${S.customBudget ? `<form id="budget-form" class="row-set"><div><b>Your budget</b><span>Press Enter to save.</span></div><label class="money-in inline"><span>$</span><input id="budget-in" type="number" min="0" step="1" value="${b || ""}" placeholder="12"><span class="unit">a day</span></label></form>` : ""}
    ${limits}
    ${S.settings ? (() => {
      // Says exactly which model runs, so there's nothing to guess about "lighter".
      const t = S.settings.talk || "balanced", names = (S.settings.talk_models?.[t] || []).join(" and ");
      const line = t === "same" ? "Each person's own model answers in chat and decides as a lead." : `${names || "A smaller model"} answers in chat and decides for leads. Tickets use each person's model.`;
      return `<div class="row-set"><div><b>Chat and lead decisions</b><span>${esc(line)}</span></div>
        <div class="seg">${[["cheapest", "Cheapest"], ["balanced", "Balanced"], ["same", "Their own"]].map(([m, n]) => `<button data-talk="${m}" aria-pressed="${m === t}">${n}</button>`).join("")}</div></div>`;
    })() : ""}
    ${any ? `<div class="chart" role="img" aria-label="Spend over the last 14 days, by kind">${r.days.map(d => `<div class="bar ${d.day === today ? "today" : ""}" title="${esc(d.day)}: ${money(d.cost)}${kinds.filter(k => d.kinds?.[k.kind]).map(k => `\n${k.label} ${money(d.kinds![k.kind])}`).join("")}"><span class="stack" style="height:${Math.max(d.cost > 0 ? 3 : 0, (d.cost / top) * 100)}%">${kinds.map(k => d.kinds?.[k.kind] ? `<i class="k-${k.kind}" style="flex:${d.kinds[k.kind]}"></i>` : "").join("")}</span></div>`).join("")}${b > 0 && b < top ? `<span class="budget-line" style="bottom:${(b / top) * 100}%"></span>` : ""}</div>
      <div class="chart-axis"><span>2 weeks ago</span><span>Today</span></div>
      ${kinds.length > 1 ? `<div class="legend">${kinds.map(k => `<span><i class="k-${k.kind}"></i>${esc(k.label)}</span>`).join("")}</div>` : ""}` : t.all.cost > 0 ? "" : `<p class="muted sp-empty">Nothing spent yet. Your team's runs add up here.</p>`}
    ${t.all.cost > 0 ? `<div class="sp-switch"><div class="seg">${tabs.map(([k, n]) => `<button data-spend-by="${k}" aria-pressed="${k === by}">${n}</button>`).join("")}</div></div>
    <table class="sp">${by === "people" ? head('<th class="num">Cap</th>') : head()}<tbody>${
      by === "people" ? people.map(p => row(`<span class="who">${face(p.name, "neutral")}${esc(first(p.name))}</span>`, p, `<td class="num muted">${p.cap > 0 ? money(p.cap) : "—"}</td>`)).join("")
      : by === "kind" ? kinds.map(k => row(`<span class="who"><i class="dot-k k-${k.kind}"></i>${esc(k.label)}</span>`, k)).join("")
      : by === "projects" ? r.projects.filter(p => p.all.cost > 0).map(p => row(esc(p.title), p)).join("") || `<tr><td colspan="4" class="muted">No project has spent anything yet.</td></tr>`
      : r.agents.map(a => row(`<span class="who"><span class="tool-ic sm" style="--c:${AGENTS[a.runtime]?.color || "#888"}">${agentMark(a.runtime)}</span>${esc(AGENTS[a.runtime]?.name || a.runtime)}${a.estimated ? ' <span class="est" title="Estimated from tokens">est.</span>' : ""}</span>`, a)).join("")
    }</tbody></table>` : ""}`;
}

// Back from Terminal with Settings on Integrations: show what changed.
window.addEventListener?.("focus", async () => { if (S.sheet === "agents") { S.ready = await invoke("readiness").catch(() => S.ready); renderSheet(); } });
