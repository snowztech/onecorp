// OneCorp app UI. It polls the company snapshot and renders the Company page and the Inbox.
// Every action calls a Tauri command, which runs the same library code as the CLI.
// Opened in a plain browser (no Tauri), it runs on sample data so the design can be checked quickly.

const invoke = window.__TAURI__ ? window.__TAURI__.core.invoke : preview;
if (window.__TAURI__) document.documentElement.classList.add("native");
const $ = s => document.querySelector(s);
const esc = s => String(s ?? "").replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]));
const first = n => String(n || "").split(" ")[0];

const S = { companies: [], company: null, snap: null, last: "", view: "company", drawer: null, person: null, addingDept: false, open: {}, channel: "general", chat: null, seen: {}, diffs: {}, handbook: null, hbEdit: false, setup: null, founder: undefined, profile: null, project: null, rail: false, needs: {}, pop: false, settings: null, health: null, confirm: null, sheet: null, shown: "" };

function toast(msg, isError) {
  const t = $("#toast");
  t.textContent = msg;
  t.className = "toast show" + (isError ? " err" : "");
  clearTimeout(toast.h);
  toast.h = setTimeout(() => (t.className = "toast"), isError ? 5000 : 2600);
}

async function call(cmd, args, done) {
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

/* Faces: drawn from the name, so everyone keeps the same face. The mood follows what they're doing. */
const hash = s => [...String(s)].reduce((h, c) => (h * 31 + c.charCodeAt(0)) >>> 0, 7);
const SKIN = ["#f3d5bd", "#e6b590", "#c88e64", "#9a6542", "#6b442b"];
const HAIR = ["#1e1916", "#4b301e", "#8b5b2c", "#d0a660", "#b4442f", "#70747d"];
function face(name, status) {
  const h = hash(name), L = { skin: h % 5, hair: (h >>> 3) % 6, style: (h >>> 6) % 5, acc: (h >>> 9) % 4 }, hue = h % 360;
  const mood = { working: "working", blocked: "blocked", idle: "sleep", queued: "neutral", done: "happy" }[status] || "neutral";
  const sk = SKIN[L.skin], hr = HAIR[L.hair], ink = "#2a211c", alt = (hue + 180) % 360;
  const cap = '<path d="M19 30q0-17 13-17t13 17q-3-9-13-9t-13 9z"/>';
  const back = L.style === 1 ? '<rect x="18" y="20" width="28" height="27" rx="9"/>' : "";
  const front = [cap, cap, '<circle cx="32" cy="12" r="5"/>' + cap,
    [[20, 22], [25, 17], [32, 15], [39, 17], [44, 22]].map(([x, y]) => `<circle cx="${x}" cy="${y}" r="5.5"/>`).join(""),
    '<path d="M20 25q12-12 24 0q-12-5-24 0z"/>'][L.style];
  const eyes = '<circle cx="27" cy="31" r="1.7"/><circle cx="37" cy="31" r="1.7"/>';
  const E = { happy: '<path d="M24.5 31q2.5-3 5 0M34.5 31q2.5-3 5 0" stroke-width="1.8" fill="none"/>', sleep: '<path d="M24.5 30.5h5M34.5 30.5h5" stroke-width="1.8"/>',
    blocked: eyes + '<path d="M24 26l5-2M40 26l-5-2" stroke-width="1.6"/>', working: eyes, neutral: eyes };
  const M = { happy: '<path d="M27 36q5 6 10 0z" fill="#fff" stroke-width="1.5"/>', sleep: '<path d="M30 37.5h4" stroke-width="1.6"/>',
    blocked: '<path d="M28 39q4-3.5 8 0" fill="none" stroke-width="1.6"/>', working: '<path d="M29 37.5h6" stroke-width="1.6"/>', neutral: '<path d="M28 36.5q4 3 8 0" fill="none" stroke-width="1.6"/>' };
  const A = ["", `<g fill="none" stroke="${ink}" stroke-width="1.5"><circle cx="27" cy="30.5" r="4.2"/><circle cx="37" cy="30.5" r="4.2"/><path d="M31.2 30.5h1.6"/></g>`,
    '<path d="M18 31a14 14 0 0 1 28 0" fill="none" stroke="#2f3440" stroke-width="3"/><rect x="15" y="27" width="5" height="10" rx="2" fill="#2f3440"/><rect x="44" y="27" width="5" height="10" rx="2" fill="#2f3440"/>',
    `<path d="M18.5 27q0-15 13.5-15t13.5 15z" fill="hsl(${alt} 45% 45%)"/><rect x="18" y="24" width="28" height="5" rx="2.5" fill="hsl(${alt} 45% 34%)"/>`][L.acc];
  return `<span class="av"><svg viewBox="0 0 64 64" aria-hidden="true"><rect width="64" height="64" fill="hsl(${hue} 50% 86%)"/>
    <g fill="${hr}">${back}</g><path d="M11 64q2-17 21-17t21 17z" fill="hsl(${hue} 52% 46%)"/><rect x="28.5" y="40" width="7" height="9" rx="2" fill="${sk}"/>
    <circle cx="32" cy="30" r="13" fill="${sk}"/><g fill="${hr}">${front}</g><g fill="${ink}" stroke="${ink}" stroke-linecap="round">${E[mood]}${M[mood]}</g>${A}</svg></span>`;
}

const RUNTIME = { claude: ["Claude Code", "#d97757"], codex: ["Codex", "#3a9b7a"] };
function chip(e) {
  const [label, color] = RUNTIME[e.runtime] || [e.runtime, "#888"];
  return `<span class="chip"><i style="background:${color}"></i>${esc(label)}${e.model ? " · " + esc(e.model) : ""}</span>`;
}
// Links to a pull request open in your browser, not inside the app.
function prLink(url, n) {
  if (!url) return "";
  const where = S.snap?.ship?.forge || "the forge";
  return `<a class="link" data-open-url="${esc(url)}">#${esc(n)} on ${esc(where)} ↗</a>`;
}
const mergeLabel = () => (S.snap?.ship?.forge ? `Merge ${S.snap.ship.pr_word}` : "Approve and merge");
/* You, the founder: your photo, or a face generated from your name */
function me() {
  const f = S.founder;
  return f?.photo ? `<span class="av"><img src="${esc(f.photo)}" alt=""></span>` : monogram(f?.name);
}
// You, without a photo: your initials. Drawn faces are for your AI team, so you never look like one of them.
const initials = name => (String(name || "").trim().split(/\s+/).filter(Boolean).slice(0, 2).map(w => w[0]).join("") || "?").toUpperCase();
const monogram = name => `<span class="av mono-av">${String(name || "").trim() ? esc(initials(name)) : icon("user")}</span>`;
const myName = () => S.founder?.name || "You";
const byId = id => (S.snap?.employees || []).find(e => e.id === id);
const waiting = () => (S.snap ? S.snap.inbox.length + S.snap.reviews.length + (S.snap.failed || []).length : 0);

/* Icons: one small stroke set, so every page speaks the same visual language */
const ICON = {
  company: '<path d="M4 21V5a1 1 0 0 1 1-1h9a1 1 0 0 1 1 1v16M15 9h4a1 1 0 0 1 1 1v11M8 8h3M8 12h3M8 16h3M3 21h18"/>',
  board: '<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M9 4v16M15 4v16"/>',
  inbox: '<path d="M3 13h5l1.5 3h5l1.5-3h5"/><path d="M5.5 5h13l2.5 8v5a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-5z"/>',
  chat: '<path d="M20 12a8 8 0 0 1-11.6 7.1L4 20l1-4.4A8 8 0 1 1 20 12z"/>',
  handbook: '<path d="M5 4.5A1.5 1.5 0 0 1 6.5 3H19v15H6.5A1.5 1.5 0 0 0 5 19.5z"/><path d="M5 19.5A1.5 1.5 0 0 0 6.5 21H19M9 7h6"/>',
  settings: '<circle cx="12" cy="12" r="3"/><path d="M19 12a7 7 0 0 0-.1-1.2l2-1.6-2-3.4-2.4 1a7 7 0 0 0-2-1.2L14 3h-4l-.5 2.6a7 7 0 0 0-2 1.2l-2.4-1-2 3.4 2 1.6a7 7 0 0 0 0 2.4l-2 1.6 2 3.4 2.4-1a7 7 0 0 0 2 1.2L10 21h4l.5-2.6a7 7 0 0 0 2-1.2l2.4 1 2-3.4-2-1.6c.1-.4.1-.8.1-1.2z"/>',
  user: '<circle cx="12" cy="8" r="4"/><path d="M4 21a8 8 0 0 1 16 0"/>',
  more: '<circle cx="5" cy="12" r="1"/><circle cx="12" cy="12" r="1"/><circle cx="19" cy="12" r="1"/>',
  check: '<path d="M5 12.5l4.5 4.5L19 7.5"/>',
  chevrons: '<path d="M8 9.5l4-4 4 4M8 14.5l4 4 4-4"/>',
  pause: '<path d="M9 5v14M15 5v14"/>',
  plus: '<path d="M12 5v14M5 12h14"/>',
  logout: '<path d="M15 4h3a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2h-3M10 16l-4-4 4-4M6 12h10"/>',
};
const icon = k => `<svg class="ic" viewBox="0 0 24 24" aria-hidden="true">${ICON[k]}</svg>`;

/* Sidebar and top bar */
function toggleRail(on = !S.rail) {
  S.rail = on;
  document.querySelector(".app").classList.toggle("rail", on);
  try { localStorage.setItem("onecorp.rail", on ? "1" : ""); } catch {}
  renderSide();
}
function renderSide() {
  const n = waiting(), pages = { company: "Company", board: "Board", inbox: "Inbox", chat: "Chat", handbook: "Handbook" };
  const unread = (S.snap?.channels || []).some(ch => ch.last && ch.last > (S.seen[ch.name] || 0) && !(S.view === "chat" && S.channel === ch.name));
  $("#side").innerHTML = `<div class="side-drag" data-tauri-drag-region></div>
    <div class="switch"><button class="co-switch" data-switch="1" aria-expanded="${!!S.switching}">${S.company ? `${badge(S.company, S.looks?.[S.company])}<span class="coname">${esc(S.company)}</span>` : `<span class="coname">Companies</span>`}${Object.entries(S.needs).some(([c, n]) => c !== S.company && n) ? '<i class="dot" title="Another company needs you"></i>' : ""}<span class="chev">${icon("chevrons")}</span></button>
      ${S.switching ? `<div class="menu" role="menu">${S.companies.map(c => `<button class="cobtn" role="menuitem" data-co="${esc(c)}" aria-current="${c === S.company}">${badge(c, S.looks?.[c])}<span class="coname">${esc(c)}</span>${c === S.company ? '<span class="tick">✓</span>' : S.needs[c] ? `<span class="badge">${S.needs[c]}</span>` : ""}</button>`).join("")}
        <hr><button class="cobtn" role="menuitem" data-new-co="1"><span class="logo add">+</span><span class="coname">New company</span></button></div>` : ""}</div>
    ${S.company ? `${Object.entries(pages).map(([k, v]) =>
      `<button class="navbtn" data-view="${k}" title="${v}" ${S.view === k ? 'aria-current="page"' : ""}>${icon(k)}<span>${v}</span>${k === "inbox" && n ? `<span class="badge">${n}</span>` : ""}${k === "chat" && unread ? `<i class="dot" title="New messages"></i>` : ""}</button>`).join("")}` : ""}
    <button class="side-foot" data-pop="menu" aria-expanded="${!!S.pop}" title="You and settings">${me()}<div><b>${esc(myName())}</b><span>Founder</span></div><span class="foot-more">${icon("more")}</span></button>`;
}
function renderTop() {
  const s = S.snap;
  $("#title").textContent = S.profile ? (S.founder ? "Your profile" : "Welcome") : S.setup ? "New company" : { inbox: "Inbox", handbook: "Handbook", board: "Board", chat: "Chat" }[S.view] || (S.company ? "Company" : "OneCorp");
  $("#new-ticket").hidden = !!S.setup || !!S.profile || !s;
}

/* Company page: you on top, departments beneath as rooms of desks */
// The agent someone runs on, as a small badge on their avatar.
function agentBadge(runtime) {
  const a = AGENTS[runtime];
  return a ? `<span class="agent-badge" style="--c:${a.color}" title="Works on ${esc(a.name)}">${agentMark(runtime)}</span>` : "";
}
function desk(e, lead) {
  const show = e.bubble ? "show" : "";
  const works = (RUNTIME[e.runtime]?.[0] || e.runtime) + (e.model ? ` · ${e.model}` : "");
  return `<button class="desk ${e.status}" data-person="${esc(e.id)}" title="${esc(e.name)} works on ${esc(works)}">
    <span class="bubble ${show}">${esc(e.bubble)}</span>
    <span class="station"><span class="seat with-agent">${face(e.name, e.status)}${agentBadge(e.runtime)}</span><span class="monitor"><i></i><i></i><i></i><i></i></span></span>
    <span class="table"></span>${e.status === "idle" ? '<span class="zz">z</span>' : ""}
    <span class="plate"><b>${esc(first(e.name))}${lead ? '<em class="leadtag">Lead</em>' : ""}</b><span>${esc(e.role)}</span></span></button>`;
}
function viewCompany() {
  const s = S.snap;
  const rooms = s.departments.map(d => {
    const ms = d.members.map(byId).filter(Boolean), lead = d.lead && byId(d.lead);
    const busy = ms.filter(e => e.status === "working").length, stuck = ms.filter(e => e.status === "blocked").length;
    const meta = [lead ? "Led by " + esc(first(lead.name)) : d.id === null ? "Move them into a department from their desk" : "No lead yet", ms.length + (ms.length === 1 ? " person" : " people")];
    if (busy) meta.push(`<span class="ok">${busy} working</span>`);
    if (stuck) meta.push(`<span class="warn">${stuck} waiting on you</span>`);
    const empty = d.id === null ? "" : `<button class="desk empty" data-hire="${d.id}"><span class="station"><span class="seat ghost"></span></span><span class="table"></span><span class="plate"><b>Empty desk</b><span>Hire into ${esc(d.name)}</span></span></button>`;
    return `<li><section class="room"><header class="room-h"><div><h3>${esc(d.name)}</h3><p>${meta.join(" · ")}</p></div>${d.id === null ? "" : `<button class="btn ghost hire-btn" data-hire="${d.id}">+ Hire</button>`}</header>
      <div class="room-floor">${ms.map(e => desk(e, d.lead === e.id)).join("")}${empty}</div></section></li>`;
  }).join("");
  const add = S.addingDept
    ? `<li><form class="room-new" id="dept-form"><label for="dept-name"><b>Name the department</b></label><div class="row"><input id="dept-name" placeholder="Engineering, Design, Growth…" autocomplete="off"><button class="btn primary">Add</button><button class="btn ghost" type="button" data-dept-cancel="1">Cancel</button></div></form></li>`
    : `<li><button class="dept-add" data-add-dept="1">+ New department</button></li>`;
  const working = s.employees.filter(e => e.status === "working").length;
  const depts = s.departments.filter(d => d.id !== null).length;
  const facts = [`${s.employees.length} ${s.employees.length === 1 ? "person" : "people"}`, `${depts} ${depts === 1 ? "department" : "departments"}`,
    s.ship?.forge ? `ships to ${s.ship.forge}` : "merges locally", ...(s.spend.cost > 0 || s.spend.tokens > 0 ? [`${s.spend.label} spent`] : []), ...(working ? [`<span class="ok">${working} working now</span>`] : [])];
  const items = needItems(s);
  const needs = items.slice(0, 3).map(i => `<button class="need" data-goto="${esc(i.to)}">${face(i.name, "neutral")}<span><b>${esc(first(i.name))}</b> ${esc(i.text)}</span></button>`).join("");
  const needsCard = `<section class="side-sec needs-sec"><h3 class="label">Needs you ${items.length ? `<span class="badge">${items.length}</span>` : ""}</h3>
    ${items.length ? `<div class="needs">${needs}</div><button class="link-btn more-in" data-view="inbox">Open Inbox →</button>` : `<p class="all-clear">${icon("check")}All clear, nothing waiting on you</p>`}</section>`;
  // Nobody working and nothing waiting: say what you could do next.
  const busy = s.employees.some(e => ["working", "queued", "blocked"].includes(e.status)) || s.kickoff.planning.length;
  const free = s.employees.find(e => e.status === "idle");
  const idle = !busy && s.tickets.length && free
    ? `<p class="idle">Your team is free · <button class="link-btn" data-plan-new="1">Plan a project</button> · <button class="link-btn" data-new-ticket="${esc(free.id)}">Give ${esc(first(free.name))} a ticket</button></p>` : "";
  return `<div class="hq"><div>
      ${kickoffCard()}
      <div class="founder"><button class="me-btn" data-profile="1" title="Edit your profile">${me()}</button><div><b>${esc(myName())}</b><span>Founder · ${facts.join(" · ")}</span>${idle}</div><button class="btn ghost" data-add-dept="1">+ Department</button></div>
      <ul class="depts">${rooms}${add}</ul></div>
    <aside class="sidecol">
      ${needsCard}
      <section class="side-sec"><h3 class="label">Live <button class="live ${s.paused ? "off" : ""}" data-pause="${s.paused ? "0" : "1"}" title="${s.paused ? "Resume work" : "Pause: running work finishes, nothing new starts"}">${s.paused ? "Paused · Resume" : "Working · Pause"}</button></h3><ul class="feed">${feed(s.feed)}</ul></section>
    </aside></div>`;
}
/* Settings: a sheet over the app, opened from your menu or ⌘,. The company, and whether everything
   OneCorp needs is in place. Your own profile lives in your menu. */
function renderSheet() {
  const el = $("#sheet");
  el.hidden = !S.sheet;
  if (!S.sheet) return (el.innerHTML = "");
  const st = S.settings, tab = S.sheet;
  let body = `<p class="muted">Opening settings…</p>`;
  if (st && tab === "company") {
    const modes = [["local", "On this Mac"], ["github", "GitHub"], ["gitlab", "GitLab"]].filter(([m]) => m === "local" || m === st.forge || m === st.ship);
    const reveal = path => window.__TAURI__?.opener ? `<button class="link-btn" data-reveal="${esc(path)}">Show in Finder</button>` : "";
    const del = S.confirm === "delete"
      ? `<form class="confirm" id="del-co"><p>This deletes <b>${esc(st.name)}</b>: ${st.people} ${st.people === 1 ? "person" : "people"}, ${st.tickets} tickets, the chat, handbook and memory. Running work stops. Your repo and its branches stay as they are.</p>
          <div class="su-row"><input id="del-name" placeholder="Type ${esc(st.name)} to confirm" autocomplete="off" spellcheck="false"><button class="btn danger">Delete</button><button class="btn ghost" type="button" data-confirm="">Cancel</button></div></form>`
      : `<div class="row-set"><div><b>Delete ${esc(st.name)}</b><span>Removes the company from OneCorp. Your repo is never touched.</span></div><button class="btn danger-ghost" data-confirm="delete">Delete…</button></div>`;
    body = `<div class="row-set"><div><b>Repo</b><span class="mono">${esc(st.repo)}</span></div>${reveal(st.repo)}</div>
      <div class="row-set"><div><b>Finished work</b><span>${st.ship === "local" ? "Merges into the repo on this Mac" : `Opens pull requests on ${esc(st.origin || "")}`}</span></div>
        <div class="seg">${modes.map(([m, n]) => `<button data-ship="${m}" aria-pressed="${m === st.ship}">${n}</button>`).join("")}</div></div>
      <div class="row-set"><div><b>OneCorp's data</b><span class="mono">${esc(st.data)}</span></div>${reveal(st.data)}</div>
      <h3 class="label danger-label">Danger zone</h3>${del}`;
  }
  if (tab === "health") {
    const mark = { ok: "✓", warn: "!", fail: "×" };
    body = S.health === null ? `<p class="muted">Checking your setup…</p>`
      : `<ul class="health">${(S.health || []).map(h => `<li class="hc lvl-${h.level}"><i>${mark[h.level]}</i><div><b>${esc(h.what)}</b><span>${esc(h.detail)}</span>${h.fix && h.level !== "ok" ? `<code>${esc(h.fix)}</code>` : ""}</div></li>`).join("")}</ul>
        <button class="link-btn" data-health="1">Check again</button>`;
  }
  const tabs = [["company", "Company"], ["health", "Setup check"]];
  el.innerHTML = `<div class="sheet" role="dialog" aria-modal="true" aria-label="Settings">
    <header class="sheet-h"><div><h2>Settings</h2><span class="muted">${esc(S.company || "")}${st ? ` · OneCorp ${esc(st.version)}` : ""}</span></div><button class="close" data-sheet-close="1" aria-label="Close">×</button></header>
    <nav class="seg tabs">${tabs.map(([k, n]) => `<button data-sheet="${k}" aria-pressed="${k === tab}">${n}</button>`).join("")}</nav>
    <div class="sheet-body">${body}</div></div>`;
}
async function openSettings(tab = "company") {
  const opening = !S.sheet;
  S.sheet = tab; S.confirm = null; S.pop = false; renderPop(); renderSheet();
  // The entrance plays once, when the sheet opens, not each time its content changes.
  if (opening) { const el = $("#sheet"); el.classList.add("opening"); setTimeout(() => el.classList.remove("opening"), 300); }
  try { S.settings = await invoke("settings", { company: S.company }); } catch (e) { toast(String(e), true); }
  renderSheet();
  if (S.health === null || tab === "health") { S.health = null; S.health = await invoke("health").catch(() => []); renderSheet(); }
}

/* Everything waiting on you, most urgent first. The Company page, the dock badge and notifications all use it. */
function needItems(s) {
  return [
    ...s.reviews.filter(r => r.kind === "plan").map(r => ({ to: r.key, name: r.name, text: `planned ${r.title}. Review the plan` })),
    ...s.inbox.filter(q => q.blocking).map(q => ({ to: "q" + q.id, name: q.name, text: q.question })),
    ...(s.failed || []).map(t => ({ to: t.key, name: t.name, text: `${t.key} stopped. Retry it?` })),
    ...s.reviews.filter(r => r.kind !== "plan").map(r => ({ to: r.key, name: r.name, text: `${r.title} is ready for review` })),
    ...s.inbox.filter(q => !q.blocking).map(q => ({ to: "q" + q.id, name: q.name, text: q.kind === "handbook" ? `proposes: ${q.question}` : q.question }))];
}
// Your team works while you're elsewhere, so the dock shows how much waits on you, and anything new
// gets a notification when the app isn't in front. What was already waiting at launch stays quiet.
const notified = {};
function alertNew(snap) {
  if (!window.__TAURI__) return;
  const items = needItems(snap);
  S.needs[snap.company] = items.length;
  const seen = notified[snap.company];
  notified[snap.company] = new Set(items.map(i => i.to));
  // The company you're looking at needs no pop-up while the app is in front.
  if (seen && !document.hasFocus()) for (const i of items.filter(i => !seen.has(i.to)).slice(0, 3)) notify(`${first(i.name)} · ${snap.company}`, i.text);
  badgeAll();
}
// Every company, every few seconds: the badge counts them all, and a question in another company
// gets a notification even when you're in this one.
async function watchAll() {
  const all = await invoke("attention").catch(() => null);
  if (!all) return;
  for (const a of all) {
    if (a.company === S.company) continue;
    const items = needItems(a), seen = notified[a.company];
    notified[a.company] = new Set(items.map(i => i.to));
    S.needs[a.company] = items.length;
    if (seen) for (const i of items.filter(i => !seen.has(i.to)).slice(0, 3)) notify(`${first(i.name)} · ${a.company}`, i.text);
  }
  badgeAll(); renderSide();
}
function badgeAll() {
  const total = Object.values(S.needs).reduce((a, b) => a + b, 0);
  window.__TAURI__?.window?.getCurrentWindow?.().setBadgeCount(total || undefined).catch(() => {});
}
async function notify(title, body) {
  const n = window.__TAURI__?.notification;
  if (!n) return;
  try {
    let ok = await n.isPermissionGranted();
    if (!ok) ok = (await n.requestPermission()) === "granted";
    if (ok) n.sendNotification({ title, body });
  } catch {}
}

/* Kickoff: from what you're building to a plan of tickets */
function kickoffCard() {
  const k = S.snap.kickoff, team = S.snap.employees;
  // Planning is live work worth seeing from the office. The rest of planning lives on the Board.
  if (k.planning.length) {
    const p = k.planning[0];
    return `<section class="kick busy">${face(p.name, "working")}<div><b>${esc(first(p.name))} is planning ${esc(p.title || "a project")}</b>
      <span class="muted">Reading the repo and turning it into tickets. The plan shows up in your Inbox to review.</span></div>
      <button class="btn" data-ticket="${esc(p.key)}">Follow along</button></section>`;
  }
  return gettingStarted();
}

/* Getting started: five steps from an empty company to your first merged work. Each step is done
   by doing it for real, so the list can't lie. It goes away when you're through, or when you hide it. */
function gettingStarted() {
  // Only real work counts: planning and the handbook your lead writes at launch are not "work" yet.
  const s = S.snap, work = s.tickets.filter(t => !t.kind || t.kind === "work");
  const firstDept = s.departments.find(d => d.id !== null);
  const plan = (s.projects || [])[0];
  const steps = [
    { done: s.employees.length > 0, title: "Hire your team", sub: "Who works here, and on which AI.", act: firstDept ? `<button class="btn primary" data-hire="${firstDept.id}">Hire someone</button>` : `<button class="btn primary" data-add-dept="1">Add a department</button>` },
    { done: !!s.kickoff.brief, title: "Say what you're building", sub: "Everyone reads it before every ticket.", act: `<button class="btn primary" data-brief="1">Write it</button>` },
    { done: !!plan, title: "Plan your first project", sub: "Your planner breaks it into tickets with owners.", act: `<button class="btn primary" data-plan-new="1">Plan a project</button>` },
    { done: work.some(t => !["backlog"].includes(t.status)) || plan?.status === "done", title: "Approve the plan", sub: "Change owners, remove tickets, then start it.", act: plan?.status === "review" ? `<button class="btn primary" data-goto="${esc(plan.key)}">Review the plan</button>` : `<span class="muted">Waiting for the plan</span>` },
    { done: work.some(t => t.status === "done"), title: "Merge your first work", sub: "Read the changes in your Inbox and approve them.", act: `<button class="btn" data-view="inbox">Open Inbox</button>` },
  ];
  const left = steps.filter(x => !x.done).length;
  let hidden = false;
  try { hidden = localStorage.getItem("onecorp.started." + S.company) === "hide"; } catch {}
  if (!left || hidden) return "";
  const next = steps.findIndex(x => !x.done);
  return `<section class="start"><header><div><b>Getting started</b><span class="muted">${steps.length - left} of ${steps.length} done</span></div>
      <span class="start-bar"><i style="width:${((steps.length - left) / steps.length) * 100}%"></i></span><button class="link-btn" data-hide-start="1">Hide</button></header>
    <ol>${steps.map((x, i) => `<li class="${x.done ? "done" : i === next ? "next" : ""}"><span class="st">${x.done ? icon("check") : i + 1}</span>
      <div><b>${x.title}</b>${i === next ? `<span class="muted">${x.sub}</span>` : ""}</div>${i === next ? x.act : ""}</li>`).join("")}</ol></section>`;
}

/* Plan a project: like an epic, except your planner breaks it down and you approve it */
function drawerPlan() {
  const s = S.snap, team = s.employees, isFirst = !(s.projects || []).length, k = s.kickoff;
  const by = byId(k.planner) || team[0];
  return `<div class="dhead"><div><h2>Plan a project</h2><span class="muted">A feature or a goal. Your planner breaks it into tickets with owners, and you approve the plan in your Inbox.</span></div><button class="close" data-close="1" aria-label="Close">×</button></div>
    <form class="dbody" id="plan-form">
      <div class="field"><label for="pj-name">Project</label><input id="pj-name" value="${isFirst ? "First version" : ""}" placeholder="Stripe billing" autocomplete="off"></div>
      <div class="field"><label for="pj-goal">What should it do?${isFirst && k.brief ? ' <span class="muted">(optional, your brief is used)</span>' : ""}</label>
        <textarea id="pj-goal" rows="6" placeholder="A Pro plan at $6 a month with a free tier. Checkout with Stripe, and a billing page to cancel."></textarea></div>
      <div class="field"><label for="pj-by">Planned by</label><select id="pj-by">${team.map(e => `<option value="${esc(e.id)}" ${e.id === by?.id ? "selected" : ""}>${esc(e.name)}, ${esc(e.role)}</option>`).join("")}</select></div>
      <div class="acts"><button class="btn primary">Plan it</button><span class="fine">For a single task, use New ticket.</span></div>
    </form>`;
}

function feed(items) {
  if (!items.length) return `<li class="none">Nothing yet. Start a ticket and it shows up here.</li>`;
  return items.map(f => `<li><button class="fl" data-ticket="${esc(f.key)}">${face(f.name, "neutral")}<div><p><b>${esc(first(f.name))}</b> ${esc(f.summary)}</p><p class="ft"><span class="key">${esc(f.key)}</span> ${esc(f.title || "")}</p></div><time>${hhmm(f.at)}</time></button></li>`).join("");
}

/* Inbox: everything that needs you, answerable in one click */
function viewInbox() {
  const s = S.snap;
  const blocking = s.inbox.filter(q => q.kind !== "handbook" && q.blocking), other = s.inbox.filter(q => q.kind !== "handbook" && !q.blocking), books = s.inbox.filter(q => q.kind === "handbook");
  const plans = s.reviews.filter(r => r.kind === "plan"), work = s.reviews.filter(r => r.kind !== "plan");
  const cards = [...plans.map(planReview), ...(s.failed || []).map(stopped), ...blocking.map(q => question(q)), ...work.map(review), ...other.map(q => question(q)), ...books.map(handbook)];
  const leads = (s.with_leads || []).map(q => question(q, true));
  const zero = `<div class="empty-state calm"><span class="empty-ic">${icon("check")}</span><b>You're all caught up</b><span>Questions, plans and reviews from your team land here when they need you.</span></div>`;
  return `<div class="inbox">${cards.length ? cards.join("") : zero}
    ${leads.length ? `<h2 class="section">With your leads <small class="muted">They decide what they can and pass up the rest. Step in anytime.</small></h2>${leads.join("")}` : ""}</div>`;
}
function stopped(t) {
  return `<article class="ask stopped" id="in-${esc(t.key)}">${face(t.name, "blocked")}<div>
    <span class="kind">Stopped · ${esc(t.key)}</span><h4>${esc(t.title)}</h4>
    <p>${esc(first(t.name))}'s run ${t.why && !/^stopped before/.test(t.why) ? `failed: ${esc(t.why)}` : "stopped before it finished."}</p>
    <div class="acts"><button class="btn primary" data-retry="${esc(t.key)}">Retry</button></div></div></article>`;
}
function question(q, withLead) {
  const opts = q.options.map(o => `<button class="btn primary" data-answer="${q.id}" data-val="${esc(o)}">${esc(o)}</button>`).join("");
  const def = q.default && !q.options.includes(q.default) ? `<button class="btn" data-answer="${q.id}" data-val="${esc(q.default)}">Go with ${esc(q.default)}</button>` : "";
  const kind = withLead ? `With ${esc(first(q.lead_name || "their lead"))}` : q.blocking ? "Blocking" : "Question";
  const via = q.via_name ? `<p class="via"><b>${esc(first(q.via_name))}</b> passed this to you${q.note ? `: ${esc(q.note)}` : "."}</p>` : "";
  return `<article class="ask ${q.blocking ? "blocking" : ""} ${withLead ? "lead" : ""}" id="in-q${q.id}">${face(q.name, q.blocking ? "blocked" : "neutral")}<div>
    <span class="kind">${kind} · ${esc(q.key || "")}</span><h4>${esc(q.question)}</h4>${via}
    <p>${esc(first(q.name))} ${q.blocking ? `stopped until ${withLead ? "someone" : "you"} answer${withLead ? "s" : ""}.` : q.default ? `went ahead with “${esc(q.default)}” for now.` : "is carrying on meanwhile."}</p>
    <div class="acts">${opts}${def}</div>
    ${more(`ans-${q.id}`, "Write your own answer", `<form class="reply" data-reply="${q.id}"><input id="ans-${q.id}" placeholder="Your answer"><button class="btn">Send</button></form>`)}</div></article>`;
}
// Free text stays folded until asked for, so each card leads with its one-click answers.
function more(id, label, form) {
  return S.open[id] ? form : `<button class="link-btn" type="button" data-more="${esc(id)}">${label}</button>`;
}
function planReview(r) {
  const people = S.snap.employees;
  const rows = r.tickets.map(t => `<li class="pt">
    <span class="k">${esc(t.key)}</span>
    <div class="pt-main"><b>${esc(t.title)}</b>${t.description ? `<span class="muted">${esc(t.description)}</span>` : ""}${t.after ? `<span class="after">after ${esc(t.after)}</span>` : ""}</div>
    <select data-plan-assign="${esc(t.key)}" aria-label="Who does ${esc(t.key)}"><option value="">Nobody yet</option>${people.map(e => `<option value="${esc(e.id)}" ${e.id === t.assignee ? "selected" : ""}>${esc(first(e.name))}</option>`).join("")}</select>
    <button class="x" data-plan-del="${esc(t.key)}" title="Remove from the plan" aria-label="Remove ${esc(t.key)}">×</button></li>`).join("");
  return `<article class="ask plan" id="in-${esc(r.key)}">${face(r.name, "done")}<div>
    <span class="kind">Project plan · ${esc(r.key)} · by ${esc(first(r.name))}</span><h4>${esc(r.title)}, in ${r.tickets.length} ${r.tickets.length === 1 ? "ticket" : "tickets"}</h4>
    ${r.summary ? `<p>${esc(r.summary)}</p>` : ""}
    <ol class="plan-list">${rows || `<li class="muted">No tickets left. Plan again, or dismiss it.</li>`}</ol>
    <div class="acts"><button class="btn good" data-approve="${esc(r.key)}" ${r.tickets.length ? "" : "disabled"}>Start the plan</button><button class="btn" data-dismiss="${esc(r.key)}">Dismiss</button></div>
    ${more(`chg-${r.key}`, "Plan it again…", `<form class="reply" data-changes="${esc(r.key)}"><input id="chg-${esc(r.key)}" placeholder="What should change?"><button class="btn">Plan again</button></form>`)}</div></article>`;
}
function review(r) {
  return `<article class="ask review" id="in-${esc(r.key)}">${face(r.name, "done")}<div>
    <span class="kind">Review · ${esc(r.key)}</span><h4>${esc(r.title)}</h4>${r.pr_url ? `<p class="prline">${prLink(r.pr_url, r.pr_number)}</p>` : ""}<p>${esc(r.summary || `${first(r.name)} finished and is waiting for your review.`)}</p>
    <div class="acts"><button class="btn good" data-approve="${esc(r.key)}">${mergeLabel()}</button>${S.diffs[r.key] ? "" : `<button class="btn" data-diff="${esc(r.key)}">Show changes</button>`}</div>
    ${changes(r.key)}
    ${more(`chg-${r.key}`, "Ask for changes", `<form class="reply" data-changes="${esc(r.key)}"><input id="chg-${esc(r.key)}" placeholder="What should change?"><button class="btn">Send back</button></form>`)}</div></article>`;
}
function handbook(q) {
  return `<article class="ask" id="in-q${q.id}">${face(q.name, "neutral")}<div>
    <span class="kind">Handbook · ${esc(q.section)}</span><h4>${esc(q.question)}</h4><p>${esc(first(q.name))} thinks everyone should follow this from now on.</p>
    <div class="acts"><button class="btn primary" data-answer="${q.id}" data-val="yes">Add to the handbook</button><button class="btn" data-answer="${q.id}" data-val="no">Not for everyone</button></div>
    ${more(`ans-${q.id}`, "Add it in your own words", `<form class="reply" data-reply="${q.id}"><input id="ans-${q.id}" placeholder="The rule, as you'd put it"><button class="btn">Add</button></form>`)}</div></article>`;
}

/* Chat: #general and a channel per department. Your team posts as they work; @mention someone to ask them. */
function viewChat() {
  const s = S.snap, chans = s.channels || [];
  const ch = chans.find(c => c.name === S.channel) || chans[0];
  if (!ch) return "";
  S.seen[ch.name] = ch.last || 0;
  const dept = s.departments.find(d => d.id === ch.department);
  const people = (dept ? dept.members.map(byId) : s.employees).filter(Boolean);
  const lead = dept?.lead && byId(dept.lead);
  const list = chans.map(c => `<button class="chan" data-channel="${esc(c.name)}" aria-current="${c.name === ch.name}"><span class="hash">#</span>${esc(c.name)}${c.last && c.last > (S.seen[c.name] || 0) ? '<i class="dot"></i>' : ""}</button>`).join("");
  const msgs = S.chat?.channel === ch.name ? S.chat.messages : null;
  let prev = null;
  const rows = (msgs || []).map(m => {
    const same = prev && prev.author === m.author && !prev.pending && !m.pending && sameMinutes(prev.at, m.at, 5);
    prev = m;
    const who = m.founder ? me() : face(m.name, m.pending ? "working" : "neutral");
    const body = m.pending ? `<p class="typing">writing<span><i></i><i></i><i></i></span></p>` : `<p>${chatText(m.text)}</p>`;
    const meta = `<header><b>${esc(m.founder ? m.name : first(m.name))}</b>${m.founder ? '<span class="you">you</span>' : ""}<time>${hhmm(m.at)}</time>${m.key ? `<a class="tkey" data-ticket="${esc(m.key)}">${esc(m.key)}</a>` : ""}</header>`;
    return `<li class="msg ${same ? "cont" : ""}">${same ? "<span></span>" : who}<div>${same ? "" : meta}${body}</div></li>`;
  }).join("");
  const empty = `<li class="chat-empty"><b>Nothing in #${esc(ch.name)} yet</b>Your team posts here as they work. Mention someone with @ to ask them something.</li>`;
  const sub = dept ? `${lead ? "Led by " + esc(first(lead.name)) + " · " : ""}${people.length} ${people.length === 1 ? "person" : "people"}` : ch.name === "general" ? "Everyone at " + esc(s.company) : "";
  return `<div class="chat">
    <nav class="chans" aria-label="Channels"><h6>Channels</h6>${list}</nav>
    <section class="room-chat">
      <header class="chat-h"><div><h3>#${esc(ch.name)}</h3><span class="muted">${sub}</span></div>
        <div class="faces">${people.slice(0, 8).map(e => `<button class="face-btn" data-mention="${esc(e.id)}" title="Mention ${esc(first(e.name))}">${face(e.name, e.status)}</button>`).join("")}</div></header>
      <ul class="msgs" id="msgs">${msgs === null ? `<li class="chat-empty muted">Loading…</li>` : rows || empty}</ul>
      <form class="composer" id="say"><div class="compose-box"><div class="mpop" id="mpop" role="listbox" hidden></div>
        <input id="say-text" placeholder="Message #${esc(ch.name)}. Type @ to ask someone, @all for everyone" autocomplete="off" role="combobox" aria-controls="mpop">
        <p class="calls" id="calls"></p></div><button class="btn primary">Send</button></form>
    </section></div>`;
}
// The database keeps UTC. Shown in your own time.
const utc = at => new Date(String(at).replace(" ", "T") + "Z");
const hhmm = at => { const d = utc(at); return isNaN(d) ? "" : d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }); };
/* Mentions: type @ for a list of your team, and see who'll reply before you send. */
const fold = t => String(t).normalize("NFD").replace(/[\u0300-\u036f]/g, "").toLowerCase();
// Everyone in the channel you're in: the whole company in #general, a department's people in theirs.
function channelPeople() {
  const ch = (S.snap.channels || []).find(c => c.name === S.channel), dept = S.snap.departments.find(d => d.id === ch?.department);
  return (dept ? dept.members.map(byId) : S.snap.employees).filter(Boolean);
}
// Who a message calls, the same way the core reads it: @firstname, or @all for the channel.
function calledBy(text) {
  const team = S.snap.employees, out = [];
  for (const [, w] of String(text).matchAll(/@([\p{L}\d]+)/gu)) {
    const id = fold(w);
    const add = ["all", "everyone", "channel"].includes(id) ? channelPeople() : team.filter(e => e.id === id);
    for (const e of add) if (!out.includes(e)) out.push(e);
  }
  return out;
}
function mentionAt(input) {
  const m = input.value.slice(0, input.selectionStart).match(/(^|\s)@([\p{L}\d]*)$/u);
  return m ? fold(m[2]) : null;
}
function updateMentions(input) {
  const pop = $("#mpop"), calls = $("#calls");
  if (!pop) return;
  const q = mentionAt(input), here = channelPeople();
  if (q === null) { pop.hidden = true; S.mention = null; }
  else {
    const people = [...here, ...S.snap.employees.filter(e => !here.includes(e))].filter(e => fold(first(e.name)).startsWith(q) || e.id.startsWith(q));
    const all = "all".startsWith(q) ? [{ all: true }] : [];
    const opts = [...people, ...all].slice(0, 7);
    S.mention = opts.length ? { opts, i: Math.min(S.mention?.i || 0, opts.length - 1) } : null;
    pop.hidden = !S.mention;
    if (S.mention) pop.innerHTML = opts.map((o, k) => o.all
      ? `<button type="button" class="mopt" data-mpick="${k}" aria-selected="${k === S.mention.i}"><span class="all-ic">@</span><span><b>all</b><small>Everyone in #${esc(S.channel)}, ${here.length} ${here.length === 1 ? "person" : "people"}</small></span></button>`
      : `<button type="button" class="mopt" data-mpick="${k}" aria-selected="${k === S.mention.i}"><span class="with-agent">${face(o.name, o.status)}${agentBadge(o.runtime)}</span><span><b>${esc(first(o.name))}</b><small>${esc(o.role)}${here.includes(o) ? "" : " · not in this channel"}</small></span></button>`).join("");
  }
  // Who'll reply, before you send. Each one is a paid AI reply, so the count is said out loud.
  const who = calledBy(input.value);
  const names = who.length > 3 ? `${who.length} people` : who.map(e => esc(first(e.name))).join(who.length === 2 ? " and " : ", ");
  calls.innerHTML = who.length ? `<span class="calls-faces">${who.slice(0, 4).map(e => face(e.name, "neutral")).join("")}</span><span>${names} will reply here</span>` : "";
}
function pickMention(k) {
  const input = $("#say-text"), o = S.mention?.opts[k];
  if (!o) return;
  const at = input.selectionStart, before = input.value.slice(0, at).replace(/@[\p{L}\d]*$/u, "@" + (o.all ? "all" : first(o.name)) + " ");
  input.value = before + input.value.slice(at);
  input.setSelectionRange(before.length, before.length);
  S.mention = null; input.focus(); updateMentions(input);
}
const sameMinutes = (a, b, n) => Math.abs(utc(b) - utc(a)) <= n * 60000;
// Mentions and ticket keys stand out, and a ticket key opens the ticket.
function chatText(text) {
  return esc(text).replace(/@([\p{L}\d]+)/gu, '<span class="mention">@$1</span>').replace(/\b([A-Z]{2,5}-\d+)\b/g, '<a class="tkey" data-ticket="$1">$1</a>');
}

/* Handbook: the file every employee reads before each ticket, shown as sections you can edit in place */
// A small Markdown renderer for the handbook: headings, lists, quotes, code, bold, italics, links.
// Enough for a wiki page, and nothing that needs a library.
function inline(t) {
  return esc(t)
    .replace(/`([^`]+)`/g, "<code>$1</code>")
    .replace(/\*\*([^*]+)\*\*/g, "<b>$1</b>")
    .replace(/(^|[^*])\*([^*\s][^*]*)\*/g, "$1<em>$2</em>")
    .replace(/\[([^\]]+)\]\((https?:[^)\s]+)\)/g, '<a class="link" data-open-url="$2">$1</a>');
}
function md(text) {
  const out = [], toc = [];
  let list = null, para = [], code = null;
  const flush = () => {
    if (para.length) out.push(`<p>${inline(para.join(" "))}</p>`), (para = []);
    if (list) out.push(`<${list.tag}>${list.items.map(i => `<li>${inline(i)}</li>`).join("")}</${list.tag}>`), (list = null);
  };
  for (const line of String(text || "").split("\n")) {
    if (code !== null) {
      if (line.startsWith("```")) { out.push(`<pre><code>${esc(code.join("\n"))}</code></pre>`); code = null; } else code.push(line);
      continue;
    }
    if (line.startsWith("```")) { flush(); code = []; continue; }
    const h = line.match(/^(#{1,3})\s+(.*)/);
    if (h) {
      flush();
      const n = h[1].length, id = "hb-" + toc.length;
      if (n === 2) toc.push([id, h[2]]);
      out.push(n === 1 ? `<h1>${inline(h[2])}</h1>` : n === 2 ? `<h2 id="${id}">${inline(h[2])}</h2>` : `<h3>${inline(h[2])}</h3>`);
      continue;
    }
    const li = line.match(/^\s*(?:[-*]|(\d+)\.)\s+(.*)/);
    if (li) {
      if (para.length) flush();
      const tag = li[1] ? "ol" : "ul";
      if (list && list.tag !== tag) flush();
      (list ||= { tag, items: [] }).items.push(li[2]);
      continue;
    }
    if (line.startsWith("> ")) { flush(); out.push(`<blockquote>${inline(line.slice(2))}</blockquote>`); continue; }
    if (!line.trim()) { flush(); continue; }
    if (list) flush();
    para.push(line.trim());
  }
  if (code !== null) out.push(`<pre><code>${esc(code.join("\n"))}</code></pre>`);
  flush();
  return { html: out.join(""), toc };
}
function viewHandbook() {
  if (S.handbook === null) return `<p class="muted">Opening the handbook…</p>`;
  const proposals = S.snap.inbox.filter(q => q.kind === "handbook");
  const props = proposals.length ? `<section class="doc-props"><span class="label">Proposed by your team</span><div class="props">${proposals.map(q => `
      <div class="prop">${face(q.name, "neutral")}<div><b>${esc(first(q.name))}</b> <span class="muted">for ${esc(q.section)}</span><p>${esc(q.question)}</p>
        <div class="acts"><button class="btn primary" data-answer="${q.id}" data-val="yes">Add</button><button class="btn" data-answer="${q.id}" data-val="no">Not for everyone</button></div></div></div>`).join("")}</div></section>` : "";
  // Editing: Markdown on the left, the page as everyone will read it on the right.
  if (S.hbEdit) {
    return `<form class="wiki-edit" id="hb-raw">
      <header><div><span class="label">Editing the handbook</span><span class="muted">Markdown. ## starts a section. Everyone reads it from their next ticket.</span></div>
        <div class="acts"><button class="btn ghost" type="button" data-hb-edit="0">Cancel</button><button class="btn primary">Save</button></div></header>
      <div class="wiki-split"><textarea id="hb-text" spellcheck="false">${esc(S.handbook)}</textarea><article class="doc wiki" id="hb-preview">${md(S.handbook).html}</article></div></form>`;
  }
  const brief = S.snap.kickoff.brief;
  const briefBody = S.open.brief
    ? `<form id="brief-form" class="doc-edit"><textarea id="brief-text" class="brief-text" rows="5" placeholder="The product, who it's for, what the first version needs.">${esc(brief)}</textarea>
        <div class="acts"><button class="btn primary">Save</button><button class="btn" type="button" data-replan="1">Save and plan it</button><button class="btn ghost" type="button" data-close-open="brief">Cancel</button></div></form>`
    : brief ? `<p>${esc(brief).replace(/\n/g, "<br>")}</p>` : `<p class="muted">Nothing yet. Say what you're building and your team can plan it.</p>`;
  // The page's own # title leads, then what we're building, then the rest of the page.
  const title = (S.handbook.match(/^#\s+(.*)$/m) || [])[1] || `${S.company} handbook`;
  const page = md(S.handbook.replace(/^#\s+.*$/m, ""));
  return `<div class="doc-wrap"><article class="doc">
      <div class="wiki-top"><span class="label">Handbook · everyone reads it before every ticket</span><button class="btn" data-hb-edit="1">Edit</button></div>
      <div class="wiki"><h1>${inline(title)}</h1></div>
      ${props}
      <section id="hb-brief" class="doc-sec brief-sec"><h2>What we're building ${S.open.brief ? "" : `<button class="link-btn" data-more="brief">Edit</button>`}</h2>${briefBody}</section>
      <div class="wiki">${page.html || `<p class="muted">Empty for now. Edit it to write your mission, decisions and conventions.</p>`}</div>
      <footer class="doc-foot"><button class="link-btn" data-hb-edit="1">Edit this page</button><button class="link-btn" data-learn="1">Update Codebase from the repo</button></footer>
    </article>
    <nav class="toc" aria-label="On this page"><span class="label">On this page</span><button data-toc="hb-brief">What we're building</button>${page.toc.map(([id, n]) => `<button data-toc="${id}">${esc(n.replace(/[`*]/g, ""))}</button>`).join("")}</nav></div>`;
}

/* Board: every ticket by where it stands. Click one for its log and actions. */
const COLUMNS = [["Backlog", ["backlog"]], ["Up next", ["todo", "queued", "waiting"]], ["Working", ["doing"]], ["Needs you", ["blocked", "review", "failed"]], ["Done", ["done"]]];
const PILL = { blocked: ["Question", "warn"], review: ["Review", "info"], failed: ["Stopped", ""], waiting: ["Waiting", ""] };
const spent = t => t.spend && !/^(\$0\.00|0 tok)$/.test(t.spend);
function viewBoard() {
  const s = S.snap;
  if (!s.tickets.length) return `<div class="empty-state"><span class="empty-ic">${icon("board")}</span><b>No tickets yet</b>Plan a project and your team breaks it into tickets, or add a single one with New ticket.<div class="acts center"><button class="btn primary" data-plan-new="1">Plan a project</button></div></div>`;
  const nameOf = id => byId(id)?.name || id || "";
  const projects = s.projects || [], current = projects.find(p => p.key === S.project);
  if (S.project && !current) S.project = null;
  const inView = t => !S.project || t.plan === S.project || t.key === S.project;
  const chip = p => {
    const pct = p.total ? Math.round((p.done / p.total) * 100) : 0;
    const state = p.status === "review" ? "to review" : ["todo", "queued", "doing"].includes(p.status) ? "planning" : `${p.done}/${p.total}`;
    return `<button class="pchip" data-project="${esc(p.key)}" aria-pressed="${S.project === p.key}">${esc(p.title)}<span class="prog"><i style="width:${pct}%"></i></span><span class="n">${state}</span></button>`;
  };
  const bar = `<div class="board-bar"><div class="pchips">${projects.length ? `<button class="pchip" data-project="" aria-pressed="${!S.project}">All work</button>${projects.map(chip).join("")}` : `<span class="muted">No projects yet. Plan one to group its tickets here.</span>`}</div>
    <button class="btn" data-plan-new="1">Plan a project</button></div>
    ${current?.goal ? `<p class="pgoal">${esc(current.goal)}</p>` : ""}`;
  const cols = COLUMNS.map(([title, statuses]) => {
    let ts = s.tickets.filter(t => statuses.includes(t.status) && inView(t));
    if (title === "Done") ts = ts.slice(-12);
    return `<section class="col"><h4>${title}<span>${ts.length}</span></h4>${ts.slice().reverse().map(t => `
      <button class="tk ${t.status} ${t.kind === "plan" ? "is-plan" : ""}" data-ticket="${esc(t.key)}"><span class="k">${esc(t.key)}${t.kind === "plan" ? " · plan" : ""}${PILL[t.status] ? `<span class="pill ${PILL[t.status][1]}">${PILL[t.status][0]}</span>` : ""}</span><span class="t">${esc(t.title)}</span>
        ${t.after && t.status !== "done" ? `<span class="after">after ${esc(t.after)}</span>` : ""}
        <span class="row">${t.assignee ? `${face(nameOf(t.assignee), "neutral")}<span class="muted">${esc(first(nameOf(t.assignee)))}</span>` : `<span class="muted">Nobody yet</span>`}${spent(t) ? `<span class="cost">${esc(t.spend)}</span>` : ""}</span></button>`).join("") || `<p class="muted none">Nothing here</p>`}</section>`;
  }).join("");
  return `${bar}<div class="board">${cols}</div>`;
}
const STATUS = { backlog: "In the backlog", waiting: "Waiting for another ticket", todo: "Up next", queued: "Up next", doing: "Working", blocked: "Waiting on you", review: "Ready for review", failed: "Stopped", done: "Merged" };
function drawerTicket() {
  const t = S.ticket;
  if (!t) return `<div class="dhead"><h2>Loading</h2><button class="close" data-close="1" aria-label="Close">×</button></div>`;
  const people = (S.snap?.employees || []).map(e => `<option value="${esc(e.id)}" ${e.id === t.employee ? "selected" : ""}>${esc(e.name)}</option>`).join("");
  const acts = {
    backlog: `<select id="t-who" data-assign="${esc(t.key)}" aria-label="Who does it"><option value="">Nobody yet</option>${people}</select><button class="btn primary" data-start="${esc(t.key)}">Start</button>`,
    waiting: `<button class="btn" data-stop="${esc(t.key)}">Back to backlog</button>`,
    doing: `<button class="btn" data-stop="${esc(t.key)}">Stop</button>`,
    queued: `<button class="btn" data-stop="${esc(t.key)}">Cancel</button>`,
    failed: `<button class="btn primary" data-retry="${esc(t.key)}">Retry</button>`,
    blocked: `<button class="btn primary" data-view="inbox" data-close="1">Answer it in the Inbox</button>`,
    review: `<button class="btn good" data-approve="${esc(t.key)}">${t.kind === "plan" ? "Start the plan" : mergeLabel()}</button>${S.diffs[t.key] ? "" : `<button class="btn" data-diff="${esc(t.key)}">Show changes</button>`}`,
  }[t.status] || "";
  const back = t.status === "review" ? `<form class="reply" data-changes="${esc(t.key)}"><input id="chg-${esc(t.key)}" placeholder="Ask for changes…"><button class="btn">Send back</button></form>` : "";
  const log = t.log.map(a => `<div class="k-${esc(a.kind)}"><span class="at">${esc(String(a.at).slice(5, 16))}</span>${esc(a.kind)} ${esc(a.summary)}</div>`).join("");
  return `<div class="dhead">${face(t.name, t.status === "doing" ? "working" : "neutral")}<div><span class="mono muted">${esc(t.key)} · ${esc(STATUS[t.status] || t.status)}</span><h2>${esc(t.title)}</h2></div><button class="close" data-close="1" aria-label="Close">×</button></div>
    <div class="dbody">
      ${t.description ? `<p class="details">${esc(t.description)}</p>` : ""}
      <dl class="kv"><dt>Who</dt><dd>${t.employee ? `<a class="link" data-person="${esc(t.employee)}">${esc(t.name)}</a>` : "Nobody yet"}</dd><dt>Spent</dt><dd>${esc(t.spend)}</dd>
        ${t.created_by ? `<dt>Created by</dt><dd>${esc(t.created_by)}</dd>` : ""}
        ${t.after ? `<dt>After</dt><dd><a class="link" data-ticket="${esc(t.after)}">${esc(t.after)}</a></dd>` : ""}
        ${t.pr_url ? `<dt>Pull request</dt><dd>${prLink(t.pr_url, t.pr_number)}</dd>` : ""}
        ${t.branch ? `<dt>Branch</dt><dd class="mono">${esc(t.branch)}${t.base ? ` <span class="muted">from ${esc(t.base)}</span>` : ""}</dd>` : ""}</dl>
      ${acts ? `<div class="acts">${acts}</div>` : ""}${back}${changes(t.key)}
      <div><h5>Everything that happened</h5>${log ? `<div class="term tall" id="term">${log}</div>` : `<p class="muted" style="margin:0">Nothing yet.</p>`}</div>
      ${["backlog", "todo", "waiting", "done", "failed"].includes(t.status) ? (S.confirm === "ticket:" + t.key
        ? `<div class="confirm"><p>Delete ${esc(t.key)}? Its log goes with it.${t.status === "done" ? " The merged work stays in your repo." : t.status === "failed" ? " Its unfinished branch is thrown away." : ""}</p>
            <div class="acts"><button class="btn danger" data-remove="${esc(t.key)}">Delete</button><button class="btn ghost" data-confirm="">Cancel</button></div></div>`
        : `<div class="dfoot"><button class="link-btn danger-link" data-confirm="ticket:${esc(t.key)}">Delete ticket…</button></div>`) : ""}
    </div>`;
}
/* New ticket: details, who, what it waits for, and start now or later */
function drawerNewTicket() {
  const open = (S.snap?.tickets || []).filter(t => t.status !== "done"), team = S.snap?.employees || [];
  const pick = S.drawer?.who || (team.find(e => e.status === "idle") || team[0])?.id;
  return `<div class="dhead"><div><h2>New ticket</h2><span class="muted">Plan it now, start it now or later.</span></div><button class="close" data-close="1" aria-label="Close">×</button></div>
    <form class="dbody" id="nt">
      <div class="field"><label for="nt-title">What needs doing</label><input id="nt-title" placeholder="Add magic link login" autocomplete="off"></div>
      <div class="field"><label for="nt-who">Who</label><select id="nt-who"><option value="">Nobody yet</option>${team.map(e => `<option value="${esc(e.id)}" ${e.id === pick ? "selected" : ""}>${esc(e.name)}, ${esc(e.role)}${e.status === "idle" ? " (free)" : ""}</option>`).join("")}</select></div>
      <div class="field"><label for="nt-desc">Details <span class="muted">(optional)</span></label><textarea id="nt-desc" rows="5" placeholder="What done looks like, links, constraints"></textarea></div>
      <div class="field"><label for="nt-after">After</label><select id="nt-after"><option value="">Nothing, it can start right away</option>${open.map(t => `<option value="${esc(t.key)}">${esc(t.key)} ${esc(t.title)}</option>`).join("")}</select></div>
      <div class="acts"><button class="btn primary" data-nt="now">Start now</button><button class="btn" data-nt="later">Save for later</button></div>
    </form>`;
}
async function saveNewTicket(later) {
  const title = $("#nt-title").value.trim();
  if (!title) return $("#nt-title").focus();
  const args = { company: S.company, title, employee: $("#nt-who").value || null, description: $("#nt-desc").value, after: $("#nt-after").value || null, later };
  const key = await call("new_ticket", args, later ? "Saved to the backlog" : "Started").catch(() => null);
  if (key) { S.view = "board"; openTicket(key); }
}

async function openTicket(key) {
  S.drawer = { type: "ticket", key }; S.ticket = null; S.confirm = null; renderDrawer();
  try { S.ticket = await invoke("ticket_detail", { company: S.company, key }); } catch (e) { toast(String(e), true); }
  renderDrawer();
}

/* The changes behind a review: commits, files, then the diff file by file. Loaded on click. */
function changes(key) {
  const d = S.diffs[key];
  if (!d) return "";
  if (d === "loading") return `<div class="changes"><p class="muted">Loading the changes…</p></div>`;
  const files = d.files.length;
  const commits = d.commits.map(c => `<li><span class="mono">${esc(c.hash)}</span> ${esc(c.subject)} <span class="muted">· ${esc(c.author)}</span></li>`).join("");
  const blocks = d.patch.split(/^(?=diff --git )/m).filter(b => b.startsWith("diff --git"));
  const open = d.added + d.removed < 300;
  const body = blocks.map(b => {
    const path = (b.match(/^diff --git a\/(.+?) b\//) || [])[1] || "file";
    const f = d.files.find(x => x.path === path) || { added: 0, removed: 0 };
    const lines = b.replace(/\n$/, "").split("\n").slice(1).filter(l => !/^(index |--- |\+\+\+ |new file mode|deleted file mode|similarity|rename )/.test(l));
    const html = lines.map(l => `<span class="${l.startsWith("@@") ? "hunk" : l.startsWith("+") ? "add" : l.startsWith("-") ? "del" : ""}">${esc(l) || " "}</span>`).join("\n");
    return `<details class="file" ${open ? "open" : ""}><summary><span class="mono">${esc(path)}</span><span class="n"><span class="ok">+${f.added}</span> <span class="del">−${f.removed}</span></span></summary><pre class="patch">${html}</pre></details>`;
  }).join("");
  return `<div class="changes">
    <div class="diffsum"><b>${files} ${files === 1 ? "file" : "files"}</b> <span class="ok">+${d.added}</span> <span class="del">−${d.removed}</span> <span class="muted">· ${esc(d.branch)} into ${esc(d.base)}</span>
      <button class="btn" data-hide-diff="${esc(key)}">Hide</button></div>
    ${commits ? `<ul class="commits">${commits}</ul>` : ""}${body || `<p class="muted">No file changes on this branch.</p>`}
    ${d.truncated ? `<p class="muted">This change is too big to show in full. See the rest with <code>onecorp diff ${esc(key)}</code>.</p>` : ""}</div>`;
}

/* Drawers: a person, or hiring into a department */
function renderDrawer() {
  const d = $("#drawer"), open = !!S.drawer;
  d.classList.toggle("open", open); $("#scrim").classList.toggle("open", open); d.setAttribute("aria-hidden", String(!open));
  if (!open) return;
  if (S.drawer.type === "hire") return (d.innerHTML = drawerHire());
  if (S.drawer.type === "new-ticket") return (d.innerHTML = drawerNewTicket());
  if (S.drawer.type === "plan") return (d.innerHTML = drawerPlan());
  if (S.drawer.type === "ticket") { d.innerHTML = drawerTicket(); const term = $("#term"); if (term) term.scrollTop = term.scrollHeight; return; }
  const p = S.person;
  if (!p) return (d.innerHTML = `<div class="dhead"><h2>Loading</h2><button class="close" data-close="1" aria-label="Close">×</button></div>`);
  const e = byId(p.id) || { status: "idle" };
  const depts = S.snap.departments.filter(x => x.id !== null), dept = depts.find(x => x.id === p.department);
  const head = `<div class="dhead"><span class="with-agent">${face(p.name, e.status)}${agentBadge(p.runtime)}</span><div><h2>${esc(p.name)}</h2><span class="muted">${esc(p.role)}${dept ? ` · ${esc(dept.name)}` : ""}</span></div><button class="close" data-close="1" aria-label="Close">×</button></div>`;
  if (S.drawer.edit) {
    d.innerHTML = `${head}<form class="dbody" id="emp-form">
      ${personFields(S.drawer.form ||= { name: p.name, role: p.role, runtime: p.runtime, model: p.model, bio: p.bio })}
      <div class="acts"><button class="btn primary">Save</button><button class="btn ghost" type="button" data-edit-person="">Cancel</button></div></form>`;
    return;
  }
  const leads = dept?.lead === p.id;
  const activity = p.activity.map(a => `<div class="k-${esc(a.kind)}"><span class="at">${esc(String(a.at).slice(11, 16))}</span>${esc(a.kind)} ${esc(a.summary)}</div>`).join("");
  const letGo = S.confirm === "fire:" + p.id
    ? `<div class="confirm"><p>Let ${esc(first(p.name))} go? They stop taking work. Their tickets and commits stay on record${leads ? `, and ${esc(dept.name)} passes to the next person in it` : ""}.</p>
        <div class="acts"><button class="btn danger" data-fire="${esc(p.id)}">Let go</button><button class="btn ghost" data-confirm="">Cancel</button></div></div>`
    : `<div class="dfoot"><button class="link-btn danger-link" data-confirm="fire:${esc(p.id)}">Let ${esc(first(p.name))} go…</button></div>`;
  d.innerHTML = `${head}
    <div class="dbody">
      ${p.bio ? `<p style="margin:0">${esc(p.bio)}</p>` : ""}
      <dl class="kv"><dt>Department</dt><dd><select id="move" data-move="${esc(p.id)}"><option value="">None</option>${depts.map(x => `<option value="${x.id}" ${p.department === x.id ? "selected" : ""}>${esc(x.name)}</option>`).join("")}</select>
          ${dept ? (leads ? `<span class="leadtag">Lead</span>` : `<button class="link-btn" data-make-lead="${esc(p.id)}">Make lead</button>`) : ""}</dd>
        <dt>Works on</dt><dd>${chip(p)}</dd></dl>
      <div class="acts"><button class="btn" data-edit-person="1">Edit</button><button class="btn" data-new-ticket="${esc(p.id)}">Give ${esc(first(p.name))} a ticket</button></div>
      <div><h5>${p.current ? `Now · ${esc(p.current.key)} · ${esc(p.current.title)}` : "Now"}</h5>${p.current?.status === "doing" ? `<div class="acts" style="margin-bottom:8px"><button class="btn" data-stop="${esc(p.current.key)}">Stop this run</button></div>` : ""}${activity ? `<div class="term" id="term">${activity}</div>` : `<p class="muted" style="margin:0">Free right now.</p>`}</div>
      <div><h5>What ${esc(first(p.name))} has learned</h5>${p.memory.length ? `<ul class="notes">${p.memory.map(m => `<li>${esc(m)}</li>`).join("")}</ul>` : `<p class="muted" style="margin:0">Nothing yet. Notes build up as they work and as you review them.</p>`}</div>
      <div><h5>Tickets</h5>${p.tickets.length ? `<ul class="tlist">${p.tickets.map(t => `<li><button class="tl" data-ticket="${esc(t.key)}"><span class="k">${esc(t.key)}</span>${esc(t.title)}<span class="s chip">${esc(STATUS[t.status] || t.status)}</span></button></li>`).join("")}</ul>` : `<p class="muted" style="margin:0">None yet.</p>`}</div>
      ${letGo}
    </div>`;
  const term = $("#term"); if (term) term.scrollTop = term.scrollHeight;
}

// Candidates differ in how they work, not just in name: cheap and fast, senior, or one that can build and test.
const NAMES = ["Noor Haddad", "Jonas Berg", "Inês Costa", "Kwame Mensah", "Priya Raman", "Léa Moreau", "Mateo Silva", "Hana Sato", "Oskar Lind", "Amara Obi", "Rafael Duarte", "Mei Lin"];
function candidates(role) {
  const taken = new Set((S.snap?.employees || []).map(e => first(e.name).toLowerCase()));
  const pool = NAMES.filter(n => !taken.has(first(n).toLowerCase()));
  const start = hash(role) % Math.max(pool.length - 2, 1);
  const tiers = [
    { tier: "Quick and cheap", runtime: "claude", model: "haiku", line: "Does exactly what the ticket says. Great for small, clear tasks." },
    { tier: "Senior", runtime: "claude", model: "sonnet", line: "Pushes back on vague tickets and ships small, careful changes." },
    { tier: "Builds and tests", runtime: "codex", model: "", line: "Runs your build and tests in a sandbox before handing work back." },
  ];
  const have = S.runtimes || ["claude", "codex"];
  const usable = tiers.map(t => (have.includes(t.runtime) ? t : { ...t, runtime: have.find(r => r !== "fake") || "fake", model: "" }));
  return usable.map((t, i) => ({ ...t, name: pool[(start + i) % pool.length] || "New hire " + (i + 1) }));
}
// The agents someone can work on, and the models each offers. Default means the agent's own choice.
const AGENTS = {
  claude: { name: "Claude Code", by: "Anthropic", color: "#d97757", models: [["", "Default"], ["opus", "Opus"], ["sonnet", "Sonnet"], ["haiku", "Haiku"]] },
  codex: { name: "Codex", by: "OpenAI", color: "#18181b", models: [["", "Default"], ["gpt-5-codex", "GPT-5 Codex"], ["gpt-5", "GPT-5"]] },
  fake: { name: "Stand-in", by: "Free, no AI", color: "#8a90a0", models: [["", "No model"]] },
};
const GLYPHS = {
  claude: '<path d="M12 3.5v17M3.5 12h17M6 6l12 12M18 6 6 18" stroke="#fff" stroke-width="2.4" stroke-linecap="round"/>',
  codex: '<path d="M6 8.5 10 12l-4 3.5M12.5 16H18" stroke="#fff" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" fill="none"/>',
  gh: '<path d="M12 3.5a8.5 8.5 0 0 0-2.7 16.6c.4.1.6-.2.6-.4v-1.5c-2.4.5-2.9-1-2.9-1-.4-1-1-1.3-1-1.3-.8-.5.1-.5.1-.5.9.1 1.3.9 1.3.9.8 1.3 2 1 2.5.7.1-.6.3-1 .6-1.2-1.9-.2-3.9-.9-3.9-4.2 0-.9.3-1.7.9-2.3-.1-.2-.4-1.1.1-2.3 0 0 .7-.2 2.3.9a8 8 0 0 1 4.2 0c1.6-1.1 2.3-.9 2.3-.9.5 1.2.2 2.1.1 2.3.6.6.9 1.4.9 2.3 0 3.3-2 4-3.9 4.2.3.3.6.8.6 1.6v2.4c0 .2.2.5.6.4A8.5 8.5 0 0 0 12 3.5z" fill="#fff"/>',
};
// An agent's mark: its glyph, white on its color.
const agentMark = k => `<svg viewBox="0 0 24 24" aria-hidden="true">${GLYPHS[k] || ""}</svg>`;
function agentPicker(f) {
  const have = S.runtimes || Object.keys(AGENTS), a = AGENTS[f.runtime] || AGENTS.claude;
  const models = a.models.some(([v]) => v === (f.model || "")) ? a.models : [...a.models, [f.model, f.model]];
  return `<div class="field"><span class="flabel">Works on</span><div class="agents">${Object.entries(AGENTS).filter(([k]) => k !== "fake" || f.runtime === "fake").map(([k, x]) => {
      const ok = have.includes(k) || k === f.runtime;
      return `<button type="button" class="agent" data-agent="${k}" aria-pressed="${k === f.runtime}" ${ok ? "" : 'disabled title="Not installed on this Mac. Settings shows how to install it"'}>
        <span class="glyph" style="--c:${x.color}">${agentMark(k)}</span><span><b>${x.name}</b><small>${ok ? x.by : "Not installed"}</small></span></button>`;
    }).join("")}</div><input type="hidden" id="pf-rt" value="${esc(f.runtime)}"></div>
    <div class="field"><span class="flabel">Model</span><div class="models">${models.map(([v, n]) => `<button type="button" class="mchip" data-model="${esc(v)}" aria-pressed="${v === (f.model || "")}">${esc(n)}</button>`).join("")}</div><input type="hidden" id="pf-m" value="${esc(f.model || "")}"></div>`;
}
// The fields for a person, shared by hiring and editing, so both look and behave the same.
function personFields(f) {
  // Their current runtime is always listed, so saving never switches it by accident.
  const runtimes = [...new Set([...(S.runtimes || ["claude", "codex", "fake"]), ...(f.runtime ? [f.runtime] : [])])];
  return `<div class="field"><label for="pf-n">Name</label><input id="pf-n" value="${esc(f.name)}" placeholder="Noor Haddad" autocomplete="off"></div>
    <div class="field"><label for="pf-r">Role</label><input id="pf-r" value="${esc(f.role)}" placeholder="Engineer, Designer, Writer…" autocomplete="off"></div>
    ${agentPicker({ ...f, runtime: runtimes.includes(f.runtime) ? f.runtime : runtimes[0] })}
    <div class="field"><label for="pf-b">How they work <span class="muted">(optional)</span></label><textarea id="pf-b" rows="3" placeholder="Left empty, it's written from their role.">${esc(f.bio || "")}</textarea></div>`;
}
const readPerson = () => ({ name: $("#pf-n").value.trim(), role: $("#pf-r").value.trim() || "Engineer", runtime: $("#pf-rt").value, model: $("#pf-m").value.trim() || null, bio: $("#pf-b").value.trim() });
function drawerHire() {
  const dept = S.snap.departments.find(x => x.id === S.drawer.dept), f = S.drawer.form;
  const picks = candidates(f.role || "Engineer");
  return `<div class="dhead"><span class="with-agent"><span class="hire-face">${face(f.name || "New hire", "neutral")}</span>${agentBadge(f.runtime)}</span><div><h2>Hire into ${esc(dept?.name || "the company")}</h2><span class="muted">Start from a suggestion or make your own. You can change anything later.</span></div><button class="close" data-close="1" aria-label="Close">×</button></div>
    <form class="dbody" id="hire-form">
      <div class="picks">${picks.map((c, i) => `<button type="button" class="pick" data-pick-fill="${i}">${face(c.name, "neutral")}<div><b>${esc(first(c.name))}</b><span>${esc(c.tier)}</span></div></button>`).join("")}</div>
      ${personFields(f)}
      <div class="acts"><button class="btn primary">Hire</button></div>
    </form>`;
}
function openHire(deptId) {
  const dept = S.snap.departments.find(x => x.id === deptId);
  const role = !dept || /eng/i.test(dept.name) ? "Engineer" : /design/i.test(dept.name) ? "Product designer" : /product/i.test(dept.name) ? "Product manager" : dept.name + " specialist";
  const c = candidates(role)[1] || candidates(role)[0];
  S.drawer = { type: "hire", dept: deptId, form: { name: c.name, role, runtime: c.runtime, model: c.model, bio: "" } };
  renderDrawer();
}
async function openPerson(id) {
  S.drawer = { type: "person", id }; S.person = null; renderDrawer();
  try { S.person = await invoke("person", { company: S.company, id }); } catch (e) { toast(String(e), true); }
  renderDrawer();
}

/* Your profile: a name and a photo, kept on this Mac. No account. The first launch asks full screen,
   after that it's a small popover from your name in the sidebar. */
// Onboarding keeps a picked photo aside until you continue; afterwards a photo saves right away.
const hasPhoto = () => !!(S.profile?.photoUrl || (!S.profile && S.founder?.photo) || (S.profile && S.founder?.photo && !S.profile.noPhoto));
const photoActs = () => `${window.__TAURI__?.dialog ? `<button class="link-btn" type="button" data-pf="pick">${hasPhoto() ? "Change photo" : "Add a photo"}</button>` : ""}
  ${hasPhoto() ? `<button class="link-btn" type="button" data-pf="face">Remove photo</button>` : ""}`.trim();
function pfPic(name) {
  if (S.profile?.photoUrl) return `<span class="av"><img src="${esc(S.profile.photoUrl)}" alt=""></span>`;
  if (S.founder?.photo && !S.profile?.noPhoto) return me();
  return monogram(name);
}
function focusTop(cancel) {
  return `<div class="focus-top" data-tauri-drag-region><span class="brand">${logo()}OneCorp</span>${cancel ? `<button class="btn ghost" type="button" data-su="cancel">Cancel</button>` : ""}</div>`;
}
function viewProfile() {
  const skip = S.companies.length ? `<button class="link-btn" type="button" data-skip-setup="1">Skip, go to ${esc(S.companies[0])}</button>` : `<span class="fine">Next: connect your AI</span>`;
  return onboarding("You", false, `<form class="onb-main welcome" id="profile">
      <h1>Who's running the company?</h1>
      <p class="muted">Your team works for you and knows you by name. Everything stays on this Mac. There's no account.</p>
      <div class="me-row"><button type="button" class="pf-pic pick-pic" data-pf="pick" title="Add a photo">${pfPic(S.profile.name)}<span class="pick-hint">${hasPhoto() ? "Change" : "Photo"}</span></button><div class="field grow"><label for="pf-name">Your name</label>
        <input id="pf-name" value="${esc(S.profile.name)}" placeholder="Lucas" autocomplete="off">
        <p class="field-err" id="pf-err" hidden>Say what your team should call you.</p></div></div>
      <div class="photo-acts">${photoActs() || `<span class="fine">Add a photo anytime.</span>`}</div>
      <div class="su-nav">${skip}<button class="btn primary">Continue</button></div>
    </form>`);
}
function renderPop() {
  const p = $("#pop");
  p.hidden = !S.pop;
  $(".side-foot")?.setAttribute("aria-expanded", String(!!S.pop));
  if (!S.pop) return;
  if (S.pop === "menu") {
    p.innerHTML = `<div class="pop-menu"><div class="pm-head">${me()}<div><b>${esc(myName())}</b><span>Founder</span></div></div>
      <button class="pm-item" data-pop="profile">${icon("user")}Edit profile</button>
      ${S.company ? `<button class="pm-item" data-settings="1">${icon("settings")}Settings<kbd>⌘,</kbd></button>` : ""}
      <hr><button class="pm-item quiet" data-pop="forget">${icon("logout")}Forget my profile…</button></div>`;
    return;
  }
  if (S.pop === "forget") {
    p.innerHTML = `<div class="pop-form"><b>Forget your profile?</b><p class="fine">OneCorp forgets your name and photo and asks again. Your companies stay.</p>
      <div class="acts end"><button class="btn" data-pop="menu">Cancel</button><button class="btn danger" data-forget="1">Forget me</button></div></div>`;
    return;
  }
  p.innerHTML = `<form id="profile" class="pop-form">
    <div class="pop-head"><span class="pf-pic">${pfPic(myName())}</span><div class="acts">${photoActs()}</div></div>
    <div class="field"><label for="pf-name">Your name</label><input id="pf-name" value="${esc(S.founder?.name || "")}" autocomplete="off"></div>
    <p class="fine">Your team knows you by this name. It stays on this Mac.</p>
    <div class="acts end"><button class="btn" type="button" data-pf="cancel">Cancel</button><button class="btn primary">Save</button></div></form>`;
}
async function saveProfile(photo, removePhoto) {
  const name = ($("#pf-name")?.value ?? S.profile.name).trim();
  if (!name) {
    // Said right where you're looking, not in a toast at the bottom.
    const err = $("#pf-err"), input = $("#pf-name");
    if (err) { err.hidden = false; input.classList.remove("shake"); void input.offsetWidth; input.classList.add("shake"); } else toast("Say what to call you first", true);
    input?.focus(); return false;
  }
  try {
    await invoke("save_founder", { name, photo: photo || null, removePhoto: !!removePhoto });
    S.founder = await invoke("founder");
    return true;
  } catch (e) { toast(String(e), true); return false; }
}

/* New company: repo, name, starting team, review. Four short steps, nothing needs the terminal. */
async function startSetup() {
  try {
    const [info, repoInfo] = await Promise.all([invoke("setup_info"), invoke("new_repo_info").catch(() => ({ parent: "", owners: [] }))]);
    const real = info.runtimes.filter(r => r !== "fake");
    invoke("readiness").then(r => { if (S.setup) { S.setup.ready = r; render(); } }).catch(() => {});
    S.setup = { step: 0, mode: "existing", path: "", repo: null, name: "", mission: "", runtime: real[0] || "fake", info, repoInfo,
      folder: "", parent: repoInfo.parent || "", owner: "", private: true, people: [], color: PALETTE[S.companies.length % PALETTE.length], logo: null, logoUrl: null };
    setTemplate("startup");
  } catch (e) { return toast(String(e), true); }
  S.drawer = null;
  render();
}
// Picking a template fills the team with its people, which you then edit freely.
function setTemplate(key) {
  const s = S.setup, t = s.info.templates.find(x => x.key === key);
  s.template = key;
  s.people = t.hires.map(h => ({ name: h.name, role: h.role, dept: h.dept || "", runtime: t.runtime || s.runtime }));
}
function readSetup() {
  const s = S.setup, val = id => document.getElementById(id)?.value;
  if (val("su-path") !== undefined) s.path = val("su-path").trim();
  if (val("su-name") !== undefined) s.name = val("su-name").trim();
  if (val("su-mission") !== undefined) s.mission = val("su-mission").trim();
  if (val("su-brief") !== undefined) s.brief = val("su-brief").trim();
  if (val("su-folder") !== undefined) s.folder = val("su-folder").trim();
  if (val("su-parent") !== undefined) s.parent = val("su-parent").trim();
  if (val("su-owner") !== undefined) s.owner = val("su-owner");
  const priv = document.getElementById("su-private");
  if (priv) s.private = priv.checked;
  s.people.forEach((p, i) => {
    for (const k of ["name", "role", "dept", "runtime"]) {
      const v = val(`su-p${i}-${k}`);
      if (v !== undefined) p[k] = v.trim();
    }
  });
}
async function checkRepo(path) {
  const s = S.setup;
  s.path = path; s.checked = path; s.repo = null; render();
  if (!path) return;
  try { s.repo = await invoke("check_repo", { path }); } catch (e) { s.repo = { ok: false, problem: String(e) }; }
  if (s.repo.ok && !s.name) s.name = s.repo.suggested;
  if (s.repo.ok && !s.folder) s.folder = s.name;
  render();
}
async function makeRepo() {
  const s = S.setup;
  if (!s.folder) { toast("Name the folder for the new repo", true); return $("#su-folder")?.focus(); }
  s.creating = true; render();
  try {
    const path = await invoke("create_repo", { parent: s.parent, folder: s.folder, mission: s.mission, owner: s.owner || null, private: s.private });
    s.creating = false; s.mode = "existing";
    toast(s.owner ? `Created ${s.folder}, here and on GitHub` : `Created ${s.folder}`);
    await checkRepo(path);
  } catch (e) { s.creating = false; render(); toast(String(e), true); }
}
const RUNTIME_NAMES = { claude: "Claude Code", codex: "Codex", fake: "Stand-in, no AI" };
function shipNote(r) {
  if (!r.forge) return `<p class="su-warn">Approved work merges into this repo on your Mac. Add a GitHub or GitLab remote later to ship as pull requests.</p>`;
  if (r.signed_in) return `<p class="su-ok">Finished work opens ${esc(r.pr_word)}s on ${esc(r.forge)}, and approving merges them. ${esc(r.account || "")}</p>`;
  const cli = r.forge === "GitHub" ? "gh" : "glab";
  return `<p class="su-warn">This repo is on ${esc(r.forge)}, but ${cli} isn't signed in, so work merges locally for now. Run <code>${cli} auth login</code>, then <code>onecorp ship ${r.forge.toLowerCase()}</code>.</p>`;
}
// A company's badge: its logo if it has one, otherwise its first letter on its color.
const PALETTE = ["#2b55c9", "#d97757", "#3a9b7a", "#8b5cf6", "#d4a019", "#e05d8a", "#2f9fc6", "#5f6573"];
function badge(name, look, cls = "") {
  if (look?.logo) return `<span class="logo ${cls} img"><img src="${esc(look.logo)}" alt=""></span>`;
  return `<span class="logo ${cls}" style="background:${esc(look?.color || "var(--fg)")};color:#fff">${esc((name || "?")[0].toUpperCase())}</span>`;
}
// The whole journey, shown on every step. A first launch also counts you.
function journey(at) {
  const steps = [...(S.companies.length && !S.onboarding && at !== "You" ? [] : ["You"]), "Connect AI", "Company", "Repo", "Team", "Launch"], i = steps.indexOf(at);
  return `<ol class="onb-steps">${steps.map((n, k) => `<li class="${k === i ? "on" : k < i ? "done" : ""}"><span class="n">${k < i ? icon("check") : k + 1}</span>${n}</li>`).join("")}</ol>`;
}
// Setup screens: the steps on the left, the step itself on the right, over a faint grid.
const onboarding = (at, cancel, form) => `<div class="focus-wrap">${focusTop(cancel)}<div class="onb"><aside>${journey(at)}</aside>${form}</div></div>`;
function viewLaunching() {
  const s = S.setup, l = s.launch;
  return `<div class="focus-wrap"><div class="launching">
      ${badge(s.name, { color: s.color, logo: s.logoUrl }, "xl")}
      <h1>Setting up ${esc(s.name)}</h1>
      <ol>${l.steps.map((t, i) => `<li class="${i < l.done ? "done" : i === l.done ? "now" : ""}"><span class="n">${i < l.done ? icon("check") : i === l.done ? '<i class="spin"></i>' : ""}</span>${esc(t)}</li>`).join("")}</ol>
    </div></div>`;
}
function viewSetup() {
  if (S.setup.launch) return viewLaunching();
  const s = S.setup, names = ["Connect AI", "Company", "Repo", "Team", "Launch"];
  const back = s.step ? `<button class="btn" type="button" data-su="back">Back</button>` : "<span></span>";
  const nav = (label, disabled) => `<div class="su-nav">${back}<button class="btn primary" ${disabled ? "disabled" : ""}>${label}</button></div>`;
  const picker = window.__TAURI__?.dialog;
  let body = "";
  const look = { color: s.color, logo: s.logoUrl };
  if (s.step === 0) {
    const r = s.ready, find = w => (r || []).find(h => h.what === w);
    const tools = [["Claude Code", "Anthropic's coding agent", "claude"], ["Codex", "OpenAI's coding agent", "codex"], ["GitHub CLI", "Optional. Ships work as pull requests", "gh"]];
    const card = ([what, about, key]) => {
      const h = find(what), state = !r ? "checking" : !h ? "missing" : h.level === "ok" ? "ok" : /not signed in/.test(h.detail) ? "signin" : "missing";
      const pill = { checking: "Checking…", ok: "Connected", signin: "Not signed in", missing: "Not installed" }[state];
      return `<div class="tool t-${state}"><span class="tool-ic" style="--c:${AGENTS[key]?.color || "#18181b"}">${agentMark(key)}</span>
        <div class="tool-main"><b>${esc(what)}</b><span>${esc(about)}</span>${h?.fix && state !== "ok" ? `<div class="fix"><code>${esc(h.fix)}</code><button class="link-btn" type="button" data-copy="${esc(h.fix)}">Copy</button></div>` : ""}</div>
        <span class="pill-state">${pill}</span></div>`;
    };
    const git = find("git"), agents = r && tools.slice(0, 2).some(([w]) => find(w)?.level === "ok");
    body = `<h1>Connect your AI</h1><p class="muted">Your employees run on these agents, signed in as you. OneCorp never sees your keys.</p>
      <div class="tools">${tools.map(card).join("")}</div>
      ${git && git.level !== "ok" ? `<p class="su-bad">git isn't installed. Run <code>${esc(git.fix || "xcode-select --install")}</code></p>` : ""}
      ${r && !agents ? `<p class="su-warn">Your employees need Claude Code or Codex to work. Install one, sign in, then check again.</p>` : ""}
      <p class="fine">Run a command in Terminal, then <button class="link-btn" type="button" data-recheck="1">check again</button>.</p>
      ${nav("Continue", !r || !agents)}`;
  } else if (s.step === 1) {
    const prefix = (s.name || "acme").replace(/[^a-z0-9]/gi, "").slice(0, 3).toUpperCase() || "ACM";
    body = `<h1>Found your company</h1><p class="muted">Its name, its look, and what it's for. Your team sees all of it.</p>
      <div class="co-card">${badge(s.name || "?", look, "xl")}<div class="field grow"><label for="su-name">Name</label><input id="su-name" class="big" value="${esc(s.name)}" placeholder="snowztech" autocomplete="off" spellcheck="false">
        <span class="fine">Letters, digits, - and _. Tickets are numbered from it, like ${esc(prefix)}-1.</span></div></div>
      <div class="field"><span class="flabel">Logo</span><div class="look-row"><div class="swatches">${PALETTE.map(c => `<button type="button" class="swatch" data-su="color-${c}" aria-pressed="${c === s.color && !s.logoUrl}" style="--c:${c}" aria-label="Color ${c}"></button>`).join("")}</div>
        ${picker ? `<button class="btn" type="button" data-su="logo">${s.logoUrl ? "Change logo…" : "Upload a logo…"}</button>` : ""}${s.logoUrl ? `<button class="link-btn" type="button" data-su="logo-remove">Use the letter</button>` : ""}</div></div>
      <div class="field"><label for="su-mission">Mission <span class="muted">(optional, it opens the handbook)</span></label><input id="su-mission" value="${esc(s.mission)}" placeholder="Open source tools for building AI agents" autocomplete="off"></div>
      <div class="field"><label for="su-brief">What are you building? <span class="muted">(optional, your team plans it once you launch)</span></label><textarea id="su-brief" rows="3" placeholder="A notes app for writers. Magic link login, sync, export to Markdown.">${esc(s.brief || "")}</textarea></div>
      ${nav("Continue")}`;
  } else if (s.step === 2) {
    const tabs = `<div class="su-tabs" role="tablist"><button type="button" role="tab" data-su="mode-existing" aria-selected="${s.mode === "existing"}">Use an existing repo</button>
      <button type="button" role="tab" data-su="mode-new" aria-selected="${s.mode === "new"}">Start a new one</button></div>`;
    if (s.mode === "new") {
      const owners = s.repoInfo.owners || [];
      const github = owners.length
        ? `<label class="su-check" for="su-gh"><input type="checkbox" id="su-gh" data-su="toggle-gh" ${s.owner ? "checked" : ""}> Also create it on GitHub</label>
           ${s.owner ? `<div class="su-row"><select id="su-owner" aria-label="GitHub account or org">${owners.map(o => `<option ${o === s.owner ? "selected" : ""}>${esc(o)}</option>`).join("")}</select>
             <span class="mono muted">/${esc((s.folder || "your-repo").toLowerCase())}</span>
             <label class="su-check" for="su-private"><input type="checkbox" id="su-private" ${s.private ? "checked" : ""}> Private</label></div>` : ""}`
        : `<p class="su-warn">Sign in to GitHub with <code>gh auth login</code> to also create the repo there. You can add a remote later too.</p>`;
      body = `<h1>Start ${esc(s.name)} from scratch</h1><p class="muted">A new folder with git, a README and a first commit, ready for your team.</p>${tabs}
        <div class="field"><label for="su-folder">Folder name</label><input id="su-folder" value="${esc(s.folder)}" placeholder="notes-app" autocomplete="off" spellcheck="false"></div>
        <div class="field"><label for="su-parent">Inside</label><div class="su-row">${picker ? `<button class="btn" type="button" data-su="parent">Choose…</button>` : ""}<input id="su-parent" value="${esc(s.parent)}" autocomplete="off" spellcheck="false"></div></div>
        ${github}
        <div class="su-nav">${back}<button class="btn primary" type="button" data-su="create-repo" ${s.creating ? "disabled" : ""}>${s.creating ? "Creating…" : "Create the repo"}</button></div>`;
    } else {
      const r = s.repo;
      const result = !r ? "" : r.ok
        ? `<p class="su-ok">✓ A git repo on <b>${esc(r.branch)}</b>.</p>${shipNote(r)}${r.used_by.length ? `<p class="su-warn">${esc(r.used_by.join(", "))} already works on this repo. That's fine, they'll just share it.</p>` : ""}`
        : `<p class="su-bad">${esc(r.problem)}</p>`;
      body = `<h1>Where does ${esc(s.name)} work?</h1><p class="muted">Employees work in their own copies of it, so your checkout is never touched.</p>${tabs}
        <div class="su-row">${picker ? `<button class="btn" type="button" data-su="pick">Choose a folder…</button>` : ""}<input id="su-path" value="${esc(s.path)}" placeholder="/Users/you/dev/your-repo" autocomplete="off" spellcheck="false"></div>
        ${result}${nav("Continue")}`;
    }
  } else if (s.step === 3) {
    const real = s.info.runtimes.filter(x => x !== "fake");
    const rows = s.people.map((p, i) => `<li class="person-row"><span class="with-agent">${face(p.name || "?", "neutral")}${agentBadge(p.runtime)}</span>
      <input id="su-p${i}-name" value="${esc(p.name)}" placeholder="Name" aria-label="Name" autocomplete="off">
      <input id="su-p${i}-role" value="${esc(p.role)}" placeholder="Role" aria-label="Role" autocomplete="off">
      <input id="su-p${i}-dept" value="${esc(p.dept)}" placeholder="Department" aria-label="Department" list="su-depts" autocomplete="off">
      <span class="mini-agents" role="group" aria-label="Works on">${real.map(k => `<button type="button" class="mini-agent" data-su-agent="${i}:${k}" aria-pressed="${k === p.runtime}" title="${esc(AGENTS[k]?.name || k)}" style="--c:${AGENTS[k]?.color || "#888"}">${agentMark(k)}</button>`).join("")}<input type="hidden" id="su-p${i}-runtime" value="${esc(p.runtime)}"></span>
      <button class="x" type="button" data-su="del-${i}" aria-label="Remove ${esc(p.name)}">×</button></li>`).join("");
    const depts = [...new Set(s.people.map(p => p.dept).filter(Boolean).concat(["Engineering", "Product", "Design", "Growth"]))];
    body = `<h1>Who's on the team to start?</h1><p class="muted">Start from a template, then change anything: names, roles, departments. The first person in a department leads it.</p>
      <div class="tpl-cards">${s.info.templates.map(x => {
        const faces = x.hires.length ? x.hires.slice(0, 5).map(h => face(h.name, "neutral")).join("") : `<span class="seat ghost"></span>`;
        return `<button type="button" class="tpl-card" data-tpl="${x.key}" aria-pressed="${x.key === s.template}">
          <span class="tpl-faces">${faces}</span><b>${esc(x.name)}</b><span>${esc(x.description)}</span></button>`;
      }).join("")}</div>
      <ul class="people">${rows || `<li class="muted">Nobody yet. Add someone, or start empty and hire later.</li>`}</ul>
      <datalist id="su-depts">${depts.map(d => `<option value="${esc(d)}">`).join("")}</datalist>
      <button class="btn" type="button" data-su="add-person">+ Add someone</button>
      ${nav("Continue")}`;
  } else {
    const ships = s.repo.forge && s.repo.signed_in ? `As ${esc(s.repo.pr_word)}s on ${esc(s.repo.forge)}` : "By merging on your Mac";
    body = `<h1>Ready to launch</h1><p class="muted">Everything can change later. This is just where you start.</p>
      <div class="launch">
        <header>${badge(s.name, look, "xl")}<div><b>${esc(s.name)}</b><span>${esc(s.mission || "No mission yet")}</span></div></header>
        <dl class="kv"><dt>Repo</dt><dd class="mono">${esc(s.repo.path)} <span class="muted">on ${esc(s.repo.branch)}</span></dd>
          <dt>Ships</dt><dd>${ships}</dd>
          <dt>Handbook</dt><dd>Starts with what OneCorp reads in the repo</dd>
          ${s.brief && s.people.length ? `<dt>First plan</dt><dd>Your planner starts right after launch</dd>` : ""}
          <dt>Team</dt><dd>${s.people.length ? `<span class="launch-team">${s.people.map(p => `<span title="${esc(p.name)}, ${esc(p.role)}">${face(p.name, "happy")}</span>`).join("")}</span> ${s.people.length} ${s.people.length === 1 ? "person" : "people"}` : "Nobody yet, hire after launch"}</dd></dl>
      </div>
      ${nav(`Launch ${esc(s.name)}`)}`;
  }
  return onboarding(names[s.step], S.companies.length, `<form class="onb-main setup ${s.step === 3 ? "wide" : ""}" id="setup">${body}</form>`);
}

/* Rendering and polling */
let pollTimer = null, ticks = 0;
function syncPolling() {
  const inWorkspace = S.company && !S.profile && !S.setup;
  if (!inWorkspace && pollTimer !== null) {
    clearInterval(pollTimer);
    pollTimer = null;
  } else if (inWorkspace && pollTimer === null) {
    pollTimer = setInterval(() => { refresh(false); if (ticks++ % 4 === 0) watchAll(); }, 1500);
  }
}
function render() {
  syncPolling();
  // Setup actions save the form themselves before changing it. Reading it again here would copy
  // stale fields over the change: the old team over a new template, or a row over the one deleted.
  if (S.profile && $("#pf-name")) S.profile.name = $("#pf-name").value;
  renderSide(); renderTop();
  const v = $("#view");
  const banner = $("#paused");
  banner.hidden = !(S.snap?.paused && !S.profile && !S.setup);
  if (!banner.hidden) banner.innerHTML = `<span>${icon("pause")}<b>${esc(S.company)} is paused.</b> Running work finishes, nothing new starts, and nobody replies in chat.</span><button class="btn" data-pause="0">Resume</button>`;
  // Setting up is full screen: no sidebar or top bar for things that don't exist yet.
  document.querySelector(".app").classList.toggle("focus", !!(S.profile || S.setup));
  v.classList.toggle("flush", !S.profile && !S.setup && S.view === "chat");
  const where = S.profile ? "profile" : S.setup ? "setup-" + S.setup.step : `${S.company}/${S.view}`;
  if (where !== S.shown) {
    S.shown = where;
    v.classList.remove("enter"); void v.offsetWidth; v.classList.add("enter");
    clearTimeout(render.t); render.t = setTimeout(() => v.classList.remove("enter"), 400);
  }
  if (S.profile) {
    v.innerHTML = viewProfile();
  } else if (S.setup) {
    v.innerHTML = viewSetup();
  } else if (!S.company) {
    v.innerHTML = `<div class="empty-state"><b>No company yet</b>Opening the new company flow…</div>`;
  } else if (!S.snap) {
    v.innerHTML = `<p class="muted">Opening ${esc(S.company)}…</p>`;
  } else {
    v.innerHTML = { inbox: viewInbox, handbook: viewHandbook, board: viewBoard, chat: viewChat }[S.view]?.() ?? viewCompany();
    const m = $("#msgs"); if (m) m.scrollTop = m.scrollHeight;
  }
  renderDrawer();
}
function typing() {
  const a = document.activeElement;
  return a && a.matches("input, textarea") && a.closest("#view, #drawer") && a.value.trim() !== "";
}
async function refresh(force) {
  // Ignore a workspace tick already queued when onboarding stopped the timer.
  if (!force && (S.profile || S.setup)) return;
  try {
    if (S.founder === undefined) S.founder = await invoke("founder");
    if (!S.runtimes) S.runtimes = (await invoke("setup_info").catch(() => null))?.runtimes || null;
    // A first launch says hello before anything else: who you are, then your first company.
    if (S.founder === null && !S.profile && !S.setup) S.profile = { name: "" };
    const c = await invoke("companies");
    S.companies = c.list; S.looks = c.looks || {};
    if (!S.company || !c.list.includes(S.company)) S.company = c.current;
    if (!S.company) {
      S.snap = null;
      if (!S.setup && !S.profile) return startSetup(); // No company yet: create one.
      // Keep mounted onboarding inputs intact during polling, including empty inputs.
      const where = S.profile ? "profile" : "setup-" + S.setup.step;
      if (!force && S.shown === where) return;
      return render();
    }
    const snap = await invoke("snapshot", { company: S.company });
    // The handbook is a small file and can change outside the app, so it's read with every refresh.
    const book = S.view === "handbook" ? await invoke("handbook", { company: S.company }) : S.handbook;
    if (S.view === "chat") S.chat = await invoke("chat", { company: S.company, channel: S.channel }).catch(() => S.chat);
    const json = JSON.stringify(snap) + (book ?? "") + (S.view === "chat" ? JSON.stringify(S.chat) : "");
    if (!force && (json === S.last || typing() || S.profile || S.setup)) return;
    S.snap = snap; S.last = json; S.handbook = book;
    alertNew(snap);
    // Channels count as read the first time they're seen, so opening the app doesn't light up every dot.
    for (const ch of snap.channels || []) if (!(ch.name in S.seen)) S.seen[ch.name] = ch.last || 0;
    if (S.drawer?.type === "person") S.person = await invoke("person", { company: S.company, id: S.drawer.id }).catch(() => S.person);
    if (S.drawer?.type === "ticket") S.ticket = await invoke("ticket_detail", { company: S.company, key: S.drawer.key }).catch(() => S.ticket);
    render();
  } catch (e) {
    toast(String(e), true);
    // A failed load still leaves the workspace polling, so it recovers on its own.
    syncPolling();
  }
}

/* Actions */
document.addEventListener("mousedown", e => {
  const o = e.target.closest?.("[data-mpick]");
  if (o) { e.preventDefault(); pickMention(Number(o.dataset.mpick)); }
});
document.addEventListener("click", async e => {
  if (S.pop && !e.target.closest("#pop, [data-profile], [data-pop]")) { S.pop = false; renderPop(); }
  if (e.target.id === "sheet") { S.sheet = null; return renderSheet(); }
  if (S.switching && !e.target.closest(".switch")) { S.switching = false; renderSide(); }
  if (e.target.id === "scrim") { S.drawer = null; return renderDrawer(); }
  const el = e.target.closest("[data-plan-del],[data-dismiss],[data-replan],[data-new-ticket],[data-nt],[data-start],[data-more],[data-switch],[data-skip-setup],[data-su-agent],[data-hb-edit],[data-learn],[data-agent],[data-model],[data-copy],[data-recheck],[data-hide-start],[data-brief],[data-pause],[data-pick-fill],[data-edit-person],[data-fire],[data-make-lead],[data-dept-cancel],[data-remove],[data-goto],[data-rail],[data-plan-new],[data-project],[data-pop],[data-settings],[data-sheet],[data-sheet-close],[data-toc],[data-close-open],[data-ship],[data-reveal],[data-health],[data-confirm],[data-forget],[data-channel],[data-mention],[data-profile],[data-pf],[data-open-url],[data-ticket],[data-stop],[data-new-co],[data-su],[data-tpl],[data-retry],[data-hb-del],[data-hb-raw],[data-diff],[data-hide-diff],[data-co],[data-view],[data-person],[data-hire],[data-add-dept],[data-close],[data-answer],[data-approve],[data-pick]");
  if (!el) return;
  const d = el.dataset;
  if (d.planDel) { await call("delete_ticket", { company: S.company, key: d.planDel }, `Removed ${d.planDel} from the plan`).catch(() => {}); return; }
  if (d.dismiss) { el.disabled = true; await call("dismiss_plan", { company: S.company, key: d.dismiss }, "Plan dismissed").catch(() => (el.disabled = false)); return; }
  if (d.replan) {
    const brief = $("#brief-text").value.trim();
    if (!brief) return $("#brief-text").focus();
    await call("kickoff", { company: S.company, brief, by: null }, "Saved. Your team is planning it").catch(() => {});
    return;
  }
  if (d.newTicket) { S.drawer = { type: "new-ticket", who: byId(d.newTicket) ? d.newTicket : null }; renderDrawer(); return $("#nt-title")?.focus(); }
  if (d.nt) { e.preventDefault(); return saveNewTicket(d.nt === "later"); }
  if (d.start) {
    el.disabled = true;
    await call("start_ticket", { company: S.company, key: d.start }, `${d.start} is starting`).catch(() => (el.disabled = false));
    return;
  }
  if (d.channel) { S.channel = d.channel; S.chat = null; render(); return refresh(true); }
  if (d.mention) {
    const input = $("#say-text"), name = first(byId(d.mention)?.name || d.mention);
    input.value = (input.value.trim() ? input.value.trimEnd() + " " : "") + "@" + name + " ";
    return input.focus();
  }
  if (d.skipSetup) { if (await saveProfile(S.profile.photo, S.profile.noPhoto)) { S.profile = null; toast(`Welcome back, ${myName()}`); refresh(true); } return; }
  if (d.hideStart) { try { localStorage.setItem("onecorp.started." + S.company, "hide"); } catch {} return render(); }
  if (d.brief) { S.view = "handbook"; S.open.brief = true; render(); await refresh(true); return $("#brief-text")?.focus(); }
  if (d.pause) {
    const on = d.pause === "1";
    await call("set_paused", { company: S.company, paused: on }, on ? `${S.company} is paused. Running work finishes, nothing new starts` : `${S.company} is back at work`).catch(() => {});
    return;
  }
  if (d.suAgent) { readSetup(); const [i, k] = d.suAgent.split(":"); S.setup.people[Number(i)].runtime = k; return render(); }
  if (d.agent) { Object.assign(S.drawer.form ||= {}, readPerson(), { runtime: d.agent, model: null }); return renderDrawer(); }
  if ("model" in d && el.classList.contains("mchip")) {
    $("#pf-m").value = d.model;
    el.parentElement.querySelectorAll(".mchip").forEach(b => b.setAttribute("aria-pressed", String(b === el)));
    return;
  }
  if (d.copy) { navigator.clipboard?.writeText(d.copy).then(() => toast("Copied. Paste it in Terminal")).catch(() => toast(d.copy)); return; }
  if (d.recheck) { S.setup.ready = null; render(); S.setup.ready = await invoke("readiness").catch(() => []); return render(); }
  if (d.pickFill) {
    const keep = readPerson(), c = candidates(keep.role)[Number(d.pickFill)];
    S.drawer.form = { ...keep, name: c.name, runtime: c.runtime, model: c.model };
    renderDrawer(); return $("#pf-n")?.focus();
  }
  if ("editPerson" in d) { S.drawer.edit = !!d.editPerson; S.drawer.form = null; S.confirm = null; renderDrawer(); return $("#pf-n")?.focus(); }
  if (d.makeLead) { await call("make_lead", { company: S.company, id: d.makeLead }, "They lead the department now").catch(() => {}); return openPerson(d.makeLead); }
  if (d.fire) {
    el.disabled = true;
    const who = first(byId(d.fire)?.name || d.fire);
    try { await invoke("fire", { company: S.company, id: d.fire }); } catch (err) { el.disabled = false; return toast(String(err), true); }
    toast(`${who} has left the company`); S.confirm = null; S.drawer = null; renderDrawer(); return refresh(true);
  }
  if (d.deptCancel) { S.addingDept = false; return render(); }
  if (d.goto) {
    S.view = "inbox"; S.drawer = null; render();
    const card = document.getElementById("in-" + d.goto);
    if (card) { card.scrollIntoView({ behavior: "smooth", block: "center" }); card.classList.add("flash"); setTimeout(() => card.classList.remove("flash"), 1600); }
    return;
  }
  if (d.rail) return toggleRail();
  if (d.planNew) { S.drawer = { type: "plan" }; renderDrawer(); return $("#pj-name")?.select(); }
  if ("project" in d) { S.project = d.project || null; return render(); }
  if (d.switch) { S.switching = !S.switching; return renderSide(); }
  if (d.toc) return document.getElementById(d.toc)?.scrollIntoView({ behavior: "smooth", block: "start" });
  if (d.closeOpen) { S.open[d.closeOpen] = false; return render(); }
  if (d.ship) {
    try { await invoke("set_ship", { company: S.company, mode: d.ship }); S.settings = await invoke("settings", { company: S.company }); toast(d.ship === "local" ? "Finished work merges on this Mac" : "Finished work opens pull requests"); }
    catch (err) { toast(String(err), true); }
    renderSheet(); return refresh(true);
  }
  if (d.reveal) return window.__TAURI__.opener.revealItemInDir(d.reveal).catch(err => toast(String(err), true));
  if (d.health) { S.health = null; renderSheet(); S.health = await invoke("health").catch(() => []); return renderSheet(); }
  if ("confirm" in d) { S.confirm = d.confirm || null; if (S.drawer?.type === "ticket") renderDrawer(); renderSheet(); return $("#del-name")?.focus(); }
  if (d.remove) {
    el.disabled = true;
    try { await invoke("remove_ticket", { company: S.company, key: d.remove }); }
    catch (err) { el.disabled = false; return toast(String(err), true); }
    toast(`Deleted ${d.remove}`); S.confirm = null; S.drawer = null; renderDrawer(); return refresh(true);
  }
  if (d.forget) {
    el.disabled = true;
    try { await invoke("forget_founder"); } catch (err) { el.disabled = false; return toast(String(err), true); }
    S.founder = null; S.confirm = null; S.pop = false; S.profile = { name: "" }; renderPop(); return render();
  }
  if (d.more) { S.open[d.more] = true; render(); return document.getElementById(d.more)?.focus(); }
  if (d.profile) { S.pop = "profile"; renderPop(); return $("#pf-name")?.focus(); }
  if (d.pop) { S.pop = d.pop === "menu" && S.pop === "menu" ? false : d.pop; renderPop(); return $("#pf-name")?.focus(); }
  if (d.settings) return openSettings();
  if (d.sheet) { S.sheet = d.sheet; S.confirm = null; return renderSheet(); }
  if (d.sheetClose) { S.sheet = null; return renderSheet(); }
  if (d.pf) {
    if (d.pf === "cancel") { S.pop = false; return renderPop(); }
    if (d.pf === "face") {
      if (S.profile) { S.profile.photo = S.profile.photoUrl = null; S.profile.noPhoto = true; return render(); }
      if (await saveProfile(null, true)) { render(); renderPop(); }
      return;
    }
    if (d.pf === "pick") {
      if (!window.__TAURI__?.dialog) return;
      const path = await window.__TAURI__.dialog.open({ multiple: false, title: "Pick a photo of you", filters: [{ name: "Photos", extensions: ["png", "jpg", "jpeg", "webp", "gif"] }] }).catch(() => null);
      if (!path) return;
      // During onboarding the photo comes first, before your name: shown now, saved when you continue.
      if (S.profile) {
        try { S.profile.photoUrl = await invoke("image_preview", { path: String(path) }); S.profile.photo = String(path); S.profile.noPhoto = false; } catch (err) { toast(String(err), true); }
        return render();
      }
      if (await saveProfile(String(path))) { render(); renderPop(); }
      return;
    }
  }
  if (d.openUrl) {
    e.preventDefault();
    return window.__TAURI__?.opener ? window.__TAURI__.opener.openUrl(d.openUrl).catch(err => toast(String(err), true)) : window.open(d.openUrl, "_blank");
  }
  if (d.ticket) return openTicket(d.ticket);
  if (d.stop) {
    el.disabled = true;
    await call("stop", { company: S.company, key: d.stop }, `Stopped ${d.stop}. Retry it anytime.`).catch(() => (el.disabled = false));
    return;
  }
  if (d.newCo) { S.switching = false; return startSetup(); }
  if (d.tpl) { readSetup(); setTemplate(d.tpl); return render(); }
  if (d.su) {
    readSetup();
    const s = S.setup;
    if (d.su === "cancel") { S.setup = null; S.onboarding = false; S.view = "company"; return refresh(true); }
    if (d.su === "back") { s.step--; return render(); }
    if (d.su === "check") return checkRepo(s.path);
    if (d.su.startsWith("color-")) { s.color = d.su.slice(6); s.logo = s.logoUrl = null; return render(); }
    if (d.su === "logo-remove") { s.logo = s.logoUrl = null; return render(); }
    if (d.su === "logo") {
      const path = await window.__TAURI__.dialog.open({ multiple: false, title: "Pick a logo", filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg", "webp", "svg", "gif"] }] }).catch(() => null);
      if (!path) return;
      try { s.logoUrl = await invoke("image_preview", { path: String(path) }); s.logo = String(path); } catch (e) { toast(String(e), true); }
      return render();
    }
    if (d.su === "mode-existing" || d.su === "mode-new") { s.mode = d.su.slice(5); return render(); }
    if (d.su === "create-repo") return makeRepo();
    if (d.su === "toggle-gh") { s.owner = s.owner ? "" : (s.repoInfo.owners[0] || ""); return render(); }
    if (d.su === "add-person") { s.people.push({ name: "", role: "Engineer", dept: "Engineering", runtime: s.runtime }); render(); return document.getElementById(`su-p${s.people.length - 1}-name`)?.focus(); }
    if (d.su.startsWith("del-")) { s.people.splice(Number(d.su.slice(4)), 1); return render(); }
    if (d.su === "parent") {
      const path = await window.__TAURI__.dialog.open({ directory: true, multiple: false, title: "Where should the new repo go?" }).catch(() => null);
      if (path) { s.parent = String(path); render(); }
      return;
    }
    if (d.su === "pick") {
      const path = await window.__TAURI__.dialog.open({ directory: true, multiple: false, title: "Pick the company's repo" }).catch(() => null);
      if (path) checkRepo(String(path));
      return;
    }
  }
  if (d.retry) {
    el.disabled = true;
    await call("retry", { company: S.company, key: d.retry }, `${d.retry} is running again`).catch(() => (el.disabled = false));
    return;
  }
  if (d.hbEdit) { S.hbEdit = d.hbEdit === "1"; render(); return $("#hb-text")?.focus(); }
  if (d.learn) { await call("learn_repo", { company: S.company }, "Codebase updated from the repo").catch(() => {}); return; }
  if (d.diff) {
    S.diffs[d.diff] = "loading"; render();
    try { S.diffs[d.diff] = await invoke("review", { company: S.company, key: d.diff }); }
    catch (err) { delete S.diffs[d.diff]; toast(String(err), true); }
    return render();
  }
  if (d.hideDiff) { delete S.diffs[d.hideDiff]; return render(); }
  if (d.co) { S.switching = false; S.setup = null; S.company = d.co; S.snap = null; S.last = ""; render(); await call("use_company", { name: d.co }); }
  else if (d.view) { S.view = d.view; S.hbEdit = false; if (d.close) S.drawer = null; render(); if (d.view === "handbook") refresh(true); }
  else if (d.person) openPerson(d.person);
  else if (d.hire) { openHire(Number(d.hire)); }
  else if (d.addDept) { S.addingDept = true; S.view = "company"; render(); $("#dept-name")?.focus(); }
  else if (d.close) { S.drawer = null; renderDrawer(); }
  else if (d.answer) { el.disabled = true; await call("answer", { company: S.company, id: Number(d.answer), text: d.val }, d.val === "no" ? "Left out of the handbook" : "Answered").catch(() => (el.disabled = false)); }
  else if (d.approve) { el.disabled = true; el.textContent = "Merging…"; await call("approve", { company: S.company, key: d.approve }, `${d.approve} merged. New tickets start from it.`).catch(() => { el.disabled = false; el.textContent = mergeLabel(); }); }

});
document.addEventListener("submit", async e => {
  e.preventDefault();
  const f = e.target;
  if (f.id === "plan-form") {
    const name = $("#pj-name").value.trim(), goal = $("#pj-goal").value.trim(), by = $("#pj-by").value || null;
    if (!name) return $("#pj-name").focus();
    if (!goal && !S.snap.kickoff.brief) { toast("Say what the project should do", true); return $("#pj-goal").focus(); }
    const btn = f.querySelector(".btn.primary"); btn.disabled = true; btn.textContent = "Starting…";
    const key = await call("plan_project", { company: S.company, name, goal, by }, `${first(byId(by)?.name || "Your planner")} is planning ${name}. The plan shows up in your Inbox`).catch(() => null);
    if (!key) { btn.disabled = false; btn.textContent = "Plan it"; return; }
    S.drawer = null; S.view = "board"; S.project = key; return render();
  }
  if (f.id === "brief-form") {
    await call("save_brief", { company: S.company, brief: $("#brief-text").value }, "Brief saved. Everyone reads it from their next ticket").then(() => { S.open.brief = false; render(); }).catch(() => {});
    return;
  }
  if (f.id === "profile") {
    const firstTime = !!S.profile;
    if (!(await saveProfile(S.profile?.photo, S.profile?.noPhoto))) return;
    if (!firstTime) { S.pop = false; renderPop(); render(); return toast("Saved"); }
    S.profile = null;
    // Continue follows the steps on screen, whether or not you already have a company.
    S.onboarding = true;
    return startSetup();
    return refresh(true);
  }
  if (f.id === "setup") {
    readSetup();
    const s = S.setup;
    if (s.step === 0) {
      const ok = (s.ready || []).some(h => ["Claude Code", "Codex"].includes(h.what) && h.level === "ok");
      if (!ok) return toast("Connect Claude Code or Codex first. Your employees need one to work", true);
      s.step = 1; render(); return $("#su-name")?.focus();
    }
    if (s.step === 1) {
      if (!/^[A-Za-z0-9_-]+$/.test(s.name)) { toast("A company name can use letters, digits, - and _, like snowztech or my-notes", true); return $("#su-name")?.focus(); }
      if (S.companies.includes(s.name)) return toast(`There's already a company called ${s.name}`, true);
      if (!s.folder) s.folder = s.name.toLowerCase();
      s.step = 2; render(); return $("#su-path")?.focus();
    }
    if (s.step === 2) {
      // Continue checks the path itself: typing, pasting or picking a folder all work the same.
      if (!s.path) { toast("Pick the repo's folder, or paste its path", true); return $("#su-path")?.focus(); }
      if (!s.repo || s.checked !== s.path) await checkRepo(s.path);
      if (s.repo?.ok) { s.step = 3; render(); }
      return;
    }
    if (s.step === 3) {
      // The same check the core makes, early, so you fix it here and not after a failed create.
      const ids = s.people.filter(p => p.name.trim()).map(p => p.name.trim().split(/\s+/)[0].toLowerCase());
      const twice = ids.find((id, i) => ids.indexOf(id) !== i);
      if (twice) return toast(`Two people would both be ${twice}. Give one of them a different first name`, true);
      s.step = 4; return render();
    }
    // Launching: the real steps tick by while the company is set up, at a pace you can read.
    const people = s.people.filter(p => p.name.trim()), real = people.some(p => p.runtime !== "fake");
    const lead = people.find(p => /eng/i.test(p.dept || "")) || people[0];
    s.launch = { done: 0, steps: [`Founding ${s.name}`, people.length ? `Hiring ${people.length} ${people.length === 1 ? "person" : "people"}` : "Opening the office",
      `Connecting ${s.repo.path.split("/").pop()}`, real && lead ? `${first(lead.name)} starts writing the handbook` : "Reading the repo",
      ...(s.brief && people.length ? ["Planning the first version"] : [])] };
    render();
    const tick = setInterval(() => { if (s.launch.done < s.launch.steps.length - 1) { s.launch.done++; render(); } }, 420);
    const pace = new Promise(r => setTimeout(r, 420 * s.launch.steps.length));
    try {
      const [name] = await Promise.all([invoke("create_company", { name: s.name, repo: s.repo.path, mission: s.mission, people, brief: s.brief || null, color: s.color, logo: s.logo }), pace]);
      clearInterval(tick); s.launch.done = s.launch.steps.length; render();
      await new Promise(r => setTimeout(r, 450));
      S.setup = null; S.onboarding = false; S.company = name; S.view = "company"; S.snap = null; S.last = "";
      await refresh(true);
    } catch (e) { clearInterval(tick); s.launch = null; render(); toast(String(e), true); }
    return;
  }
  if (f.id === "del-co") {
    const name = S.settings.name;
    if ($("#del-name").value.trim() !== name) { toast(`Type ${name} to confirm`, true); return $("#del-name").focus(); }
    const btn = f.querySelector(".danger"); btn.disabled = true; btn.textContent = "Deleting…";
    try { await invoke("delete_company", { name }); } catch (err) { btn.disabled = false; btn.textContent = `Delete ${name}`; return toast(String(err), true); }
    toast(`Deleted ${name}. Your repo is untouched.`);
    S.company = null; S.snap = null; S.last = ""; S.settings = null; S.confirm = null; S.sheet = null; S.view = "company"; renderSheet();
    return refresh(true);
  }
  if (f.id === "say") {
    const input = $("#say-text"), text = input.value.trim();
    if (!text) return input.focus();
    input.value = ""; updateMentions(input);
    await call("say", { company: S.company, channel: S.channel, text }).catch(() => { input.value = text; updateMentions(input); });
    return $("#say-text")?.focus();
  }
  if (f.id === "dept-form") {
    const name = $("#dept-name").value.trim();
    if (!name) return $("#dept-name").focus();
    const id = await call("add_department", { company: S.company, name }, `${name} is set up. Hire its first person.`).catch(() => null);
    if (id !== null) { S.addingDept = false; render(); openHire(id); }
  } else if (f.id === "hb-raw") {
    await call("save_handbook", { company: S.company, text: $("#hb-text").value }, "Saved. Everyone reads it from their next ticket").then(() => { S.hbEdit = false; render(); }).catch(() => {});
  } else if (f.id === "hire-form" || f.id === "emp-form") {
    const p = readPerson();
    if (!p.name) { toast("Give them a name", true); return $("#pf-n").focus(); }
    const btn = f.querySelector(".btn.primary"); btn.disabled = true;
    try {
      if (f.id === "hire-form") {
        await invoke("hire", { company: S.company, ...p, department: S.drawer.dept });
        toast(`${first(p.name)} joined. Give them a ticket whenever you're ready`); S.drawer = null; renderDrawer();
      } else {
        await invoke("update_employee", { company: S.company, id: S.drawer.id, ...p });
        toast("Saved"); S.drawer.edit = false; await openPerson(S.drawer.id);
      }
      await refresh(true);
    } catch (err) { btn.disabled = false; toast(String(err), true); }
  } else if (f.dataset.reply) {
    const input = f.querySelector("input"), text = input.value.trim();
    if (!text) return input.focus();
    await call("answer", { company: S.company, id: Number(f.dataset.reply), text }, "Answered").catch(() => {});
  } else if (f.dataset.changes) {
    const input = f.querySelector("input"), feedback = input.value.trim();
    if (!feedback) return input.focus();
    await call("request_changes", { company: S.company, key: f.dataset.changes, feedback }, "Sent back. It's saved to their memory too.").catch(() => {});
  }
});
// The drawn face follows your name as you type it.
const faceLater = {};
document.addEventListener("input", e => {
  if (e.target.id === "say-text") updateMentions(e.target);
  if (e.target.id === "su-name" && !S.setup?.logoUrl) { const b = $(".co-card .logo"); if (b) b.textContent = (e.target.value.trim() || "?")[0].toUpperCase(); }
  if (e.target.id === "hb-text") $("#hb-preview").innerHTML = md(e.target.value).html;
  if (e.target.id === "pf-n" && S.drawer?.type === "hire") {
    clearTimeout(faceLater.t);
    faceLater.t = setTimeout(() => { const pic = $(".hire-face"); if (pic) { pic.innerHTML = face(e.target.value || "New hire", "neutral"); pic.classList.remove("swap"); void pic.offsetWidth; pic.classList.add("swap"); } }, 400);
  }
  if (e.target.id === "pf-name" && S.profile && !hasPhoto()) { const m = $(".welcome .mono-av"); if (m) m.innerHTML = e.target.value.trim() ? esc(initials(e.target.value)) : icon("user"); }
});
document.addEventListener("change", async e => {
  if (e.target.dataset?.planAssign) {
    const employee = e.target.value;
    if (employee) await call("assign", { company: S.company, key: e.target.dataset.planAssign, employee }, `Given to ${first(byId(employee)?.name)}`).catch(() => {});
    return;
  }
  if (e.target.dataset?.assign) {
    const key = e.target.dataset.assign, employee = e.target.value;
    if (employee) await call("assign", { company: S.company, key, employee }, `Given to ${first(byId(employee)?.name)}`).catch(() => {});
    return openTicket(key);
  }
  if (e.target.id === "move" && e.target.value) {
    await call("move_to_department", { company: S.company, employee: e.target.dataset.move, department: Number(e.target.value) }, "Moved").catch(() => {});
    if (S.drawer?.type === "person") openPerson(S.drawer.id);
  }
});
document.addEventListener("keydown", e => {
  if (e.target.id === "say-text" && S.mention) {
    const n = S.mention.opts.length;
    if (e.key === "ArrowDown" || e.key === "ArrowUp") { e.preventDefault(); S.mention.i = (S.mention.i + (e.key === "ArrowDown" ? 1 : n - 1)) % n; return updateMentions(e.target); }
    if (e.key === "Enter" || e.key === "Tab") { e.preventDefault(); return pickMention(S.mention.i); }
    if (e.key === "Escape") { e.preventDefault(); S.mention = null; $("#mpop").hidden = true; return; }
  }
  if ((e.metaKey || e.ctrlKey) && (e.code === "Backslash" || e.key === "\\" || e.key.toLowerCase() === "b") && !e.target.closest?.("input, textarea")) { e.preventDefault(); return toggleRail(); }
  if (e.key === "Escape" && S.sheet) { S.sheet = null; return renderSheet(); }
  if (e.key === "Escape" && S.switching) { S.switching = false; return renderSide(); }
  if (e.key === "Escape" && S.pop) { S.pop = false; return renderPop(); }
  if (e.key === "Escape") { if (S.drawer) { S.drawer = null; renderDrawer(); } else if (S.addingDept) { S.addingDept = false; render(); } }
  if ((e.metaKey || e.ctrlKey) && e.key === "," && S.snap && !S.setup && !S.profile) { e.preventDefault(); return openSettings(); }
  if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k" && S.snap && !S.setup && !S.profile) { e.preventDefault(); S.drawer = { type: "new-ticket" }; renderDrawer(); $("#nt-title")?.focus(); }
});

// In a plain browser, #board, #inbox, #handbook, #ticket, #person, #new-ticket, #setup or #profile opens
// straight onto that screen, so each one can be looked at and screenshotted. Never used in the app.
async function previewScreen() {
  const screen = location.hash.slice(1);
  await refresh(true);
  if (screen === "settings") await openSettings();
  if (screen === "switcher") { S.switching = true; renderSide(); }
  if (screen === "plan") { S.view = "board"; S.drawer = { type: "plan" }; render(); }
  if (screen === "rail") toggleRail(true);
  if (screen === "start") { S.snap.tickets = []; S.snap.projects = []; S.snap.kickoff.brief = ""; render(); }
  if (screen === "wiki-edit") { S.view = "handbook"; S.hbEdit = true; await refresh(true); render(); }
  if (screen === "inbox-zero") { S.snap.inbox = []; S.snap.reviews = []; S.snap.failed = []; S.snap.with_leads = []; S.view = "inbox"; render(); }
  if (screen === "launching") { await startSetup(); S.setup.name = "snowztech"; await checkRepo("/Users/you/dev/snowztech"); S.setup.launch = { done: 2, steps: ["Founding snowztech", "Hiring 3 people", "Connecting snowztech", "Odile starts writing the handbook"] }; render(); }
  if (screen === "paused") { S.snap.paused = true; render(); }
  if (screen === "hire") openHire(1);
  if (screen === "person-edit") { await openPerson("tomas"); S.drawer.edit = true; renderDrawer(); }
  if (screen === "idle") { S.snap.employees.forEach(e => (e.status = "idle")); S.snap.inbox = []; S.snap.reviews = []; S.snap.failed = []; render(); }
  if (screen === "project") { S.view = "board"; S.project = "SNO-20"; render(); }
  if (["board", "inbox", "handbook", "chat"].includes(screen)) { S.view = screen; render(); }
  if (screen === "ticket") openTicket("SNO-11");
  if (screen === "person") openPerson("tomas");
  if (screen === "new-ticket") { S.view = "board"; S.drawer = { type: "new-ticket" }; render(); }
  if (screen === "setup") await startSetup();
  if (screen.startsWith("setup-")) { await startSetup(); S.setup.name = "snowztech"; S.setup.mission = "Open source tools for building AI agents"; await checkRepo("/Users/you/dev/snowztech"); S.setup.step = Number(screen.slice(6)); render(); }
  if (screen === "profile") { S.founder = null; S.profile = { name: "" }; render(); }
  if (screen === "popover") { S.pop = "profile"; renderPop(); }
  if (screen === "menu") { S.pop = "menu"; renderPop(); }
  if (screen === "health") await openSettings("health");
}
try { if (localStorage.getItem("onecorp.rail")) toggleRail(true); } catch {}
if (window.__TAURI__) refresh(true); else previewScreen();

/* Sample data for opening ui/index.html in a browser. Never used inside the app. */
function preview(cmd) {
  const team = [
    ["odile", "Odile Martin", "CTO", "claude", "opus", "working", "Reading the SNO-12 diff", 1], ["maya", "Maya Okafor", "Senior engineer", "codex", null, "blocked", "Need you on SNO-12", 1],
    ["tomas", "Tomás Reis", "Engineer", "codex", null, "working", "Bash cargo test billing", 1], ["rui", "Rui Alves", "Product lead", "claude", "sonnet", "working", "Created SNO-18", 2],
    ["lina", "Lina Park", "Product designer", "claude", "sonnet", "queued", "Up next: SNO-14", 2], ["sam", "Sam Duarte", "Writer", "claude", "haiku", "idle", "", 3],
  ].map(([id, name, role, runtime, model, status, bubble, department]) => ({ id, name, role, runtime, model, status, bubble, department, spend: "$0.40", ticket: null }));
  const data = {
    companies: { list: ["snowztech", "tvcask", "autoremind", "pingbuddy"], current: "snowztech", looks: { snowztech: { color: "#2b55c9" }, tvcask: { color: "#d97757" }, autoremind: { color: "#3a9b7a" }, pingbuddy: { color: "#8b5cf6" } } },
    snapshot: {
      company: "snowztech", serve: true, kickoff: { brief: "A notes app for writers.", planner: "rui", planned: true, planning: [] }, ship: { mode: "github", forge: "GitHub", pr_word: "pull request" }, employees: team, tickets: [
        { key: "SNO-9", plan: "SNO-20", title: "Rate limit the public API", assignee: "tomas", status: "failed", spend: "40k tok" },
        { key: "SNO-11", plan: "SNO-20", title: "Pricing page", assignee: "odile", status: "review", spend: "$0.31" },
        { key: "SNO-12", plan: "SNO-20", title: "Magic link login", assignee: "maya", status: "blocked", spend: "52k tok" },
        { key: "SNO-14", plan: "SNO-20", title: "Empty state for the notes list", assignee: "lina", status: "queued", spend: "$0.00" },
        { key: "SNO-15", plan: "SNO-20", title: "Retry failed Stripe webhooks", assignee: "tomas", status: "doing", spend: "75k tok" },
        { key: "SNO-8", plan: "SNO-20", title: "CI pipeline", assignee: "tomas", status: "done", spend: "$0.22" },
        { key: "SNO-16", title: "Export notes as Markdown", assignee: null, status: "backlog", spend: "$0.00", created_by: "Rui Alves" },
        { key: "SNO-17", title: "Notes API pagination", assignee: "maya", status: "waiting", spend: "$0.00", after: "SNO-12" }],
      projects: [{ key: "SNO-20", title: "First version", goal: "", status: "done", planner: "rui", planner_name: "Rui Alves", total: 8, done: 5 }, { key: "SNO-30", title: "Stripe billing", goal: "Pro plan at $6 a month with a free tier.", status: "review", planner: "rui", planner_name: "Rui Alves", total: 4, done: 0 }],
      departments: [{ id: 1, name: "Engineering", lead: "odile", members: ["odile", "maya", "tomas"] }, { id: 2, name: "Product", lead: "rui", members: ["rui", "lina"] }, { id: 3, name: "Growth", lead: "sam", members: ["sam"] }],
      with_leads: [{ id: 5, kind: "question", key: "SNO-15", employee: "tomas", name: "Tomás Reis", lead_name: "Odile Martin", question: "Retry webhooks 3 or 5 times?", section: "", options: ["3", "5"], default: "3", blocking: false }],
      inbox: [{ id: 2, kind: "question", key: "SNO-12", employee: "maya", name: "Maya Okafor", question: "Magic links: 15 or 60 minute expiry?", section: "", options: ["15 minutes", "60 minutes"], default: "15 minutes", blocking: true, via: "odile", via_name: "Odile Martin", note: "it changes what users see, so it's a product call" },
        { id: 3, kind: "handbook", key: "SNO-14", employee: "lina", name: "Lina Park", question: "Empty states are one sentence and one button", section: "Conventions", options: [], default: "", blocking: false }],
      failed: [{ key: "SNO-9", title: "Rate limit the public API", employee: "tomas", name: "Tomás Reis", why: "stopped before it finished" }],
      reviews: [{ key: "SNO-20", title: "First version", employee: "rui", name: "Rui Alves", kind: "plan", summary: "Setup first, then login and sync in parallel. Risk: magic links need an email provider.", spend: "$0.21",
          tickets: [{ key: "SNO-21", title: "Set up the project", description: "A skeleton that builds, with a README.", assignee: "maya", name: "Maya Okafor", after: null, status: "backlog" },
            { key: "SNO-22", title: "Magic link login", description: "Email a one-time link, 15 minute expiry.", assignee: "tomas", name: "Tomás Reis", after: "SNO-21", status: "backlog" }] },
        { key: "SNO-11", kind: "work", tickets: [], title: "Pricing page", employee: "odile", name: "Odile Martin", summary: "Added the pricing page with Free and Pro plans. Check the copy on mobile.", spend: "$0.31", pr_number: 41, pr_url: "https://github.com/snowztech/notes/pull/41" }],
      feed: [{ at: "2026-11-09 11:02:00", key: "SNO-11", employee: "odile", name: "Odile Martin", kind: "done", summary: "finished, $0.31", title: "Pricing page" }, { at: "2026-11-09 10:52:00", key: "SNO-15", employee: "tomas", name: "Tomás Reis", kind: "start", summary: "started", title: "Retry failed Stripe webhooks" }, { at: "2026-11-09 10:31:00", key: "SNO-12", employee: "maya", name: "Maya Okafor", kind: "block", summary: "asked you about magic link expiry", title: "Magic link login" }],
      spend: { cost: 3.24, tokens: 180000, label: "$3.24" },
      channels: [{ name: "general", title: "General", department: null, last: 9 }, { name: "engineering", title: "Engineering", department: 1, last: 8 }, { name: "product", title: "Product", department: 2, last: 4 }, { name: "growth", title: "Growth", department: 3, last: null }],
    },
    handbook: "# snowztech handbook\n\n## Mission\n\nOpen source tools for building AI agents.\n\n## Decisions\n\n- Standard library first. No new dependencies without asking the founder.\n- Auth tokens are hashed at rest with sha2.\n\n## Conventions\n\n- All program output is lowercase, with no trailing punctuation.\n- CLI flags are long-form, kebab-case.\n",
    setup_info: { runtimes: ["claude", "fake"], templates: [
      { key: "solo", name: "Solo builder", description: "One engineer who ships. Hire more when you need them.", runtime: null, hires: [{ name: "Maya Okafor", role: "Engineer", dept: "Engineering" }] },
      { key: "startup", name: "Startup", description: "A CTO, frontend and backend engineers, a PM, a designer, and a writer for the launch.", runtime: null, hires: [{ name: "Odile Martin", role: "CTO", dept: "Engineering" }, { name: "Maya Okafor", role: "Frontend engineer", dept: "Engineering" }, { name: "Tomás Reis", role: "Backend engineer", dept: "Engineering" }, { name: "Rui Alves", role: "Product manager", dept: "Product" }, { name: "Lina Park", role: "Product designer", dept: "Product" }, { name: "Sam Duarte", role: "Growth writer", dept: "Growth" }] },
      { key: "indie", name: "Indie maker", description: "Build it and launch it yourself: an engineer, a designer, and a growth marketer.", runtime: null, hires: [{ name: "Maya Okafor", role: "Engineer", dept: "Product" }, { name: "Lina Park", role: "Product designer", dept: "Product" }, { name: "Sam Duarte", role: "Growth marketer", dept: "Growth" }] },
      { key: "agency", name: "Agency", description: "Client work: a PM who plans, two engineers, a designer, and a QA reviewer who checks every change.", runtime: null, hires: [{ name: "Rui Alves", role: "Project manager", dept: "Delivery" }, { name: "Maya Okafor", role: "Engineer", dept: "Delivery" }, { name: "Tomás Reis", role: "Engineer", dept: "Delivery" }, { name: "Lina Park", role: "Product designer", dept: "Delivery" }, { name: "Odile Martin", role: "QA reviewer", dept: "Delivery" }] },
      { key: "oss", name: "Open source", description: "A maintainer who reviews every change, an engineer, and a writer for docs and changelogs.", runtime: null, hires: [{ name: "Odile Martin", role: "Maintainer", dept: "Engineering" }, { name: "Maya Okafor", role: "Engineer", dept: "Engineering" }, { name: "Sam Duarte", role: "Docs writer", dept: "Docs" }] },
      { key: "empty", name: "Start empty", description: "No one yet. Add departments and hire them yourself.", runtime: null, hires: [] }] },
    ticket_detail: { key: "SNO-11", title: "Pricing page", status: "review", description: "Free and Pro plans. Pro is $6 a month.", created_by: "Rui Alves", after: null, pr_number: 41, pr_url: "https://github.com/snowztech/notes/pull/41", employee: "odile", name: "Odile Martin", branch: "onecorp/snowztech/sno-11", base: "main", spend: "$0.31",
      log: [{ at: "2026-11-09 10:40:00", kind: "start", summary: "claude (opus), started" }, { at: "2026-11-09 10:58:00", kind: "commit", summary: "c3dbf20 SNO-11: Pricing page" }, { at: "2026-11-09 11:02:00", kind: "done", summary: "finished, $0.31" }] },
    new_repo_info: { parent: "/Users/you/dev", owners: ["you", "snowztech"] },
    readiness: [{ level: "ok", what: "git", detail: "git version 2.47.1", fix: null }, { level: "ok", what: "Claude Code", detail: "2.3.4", fix: null },
      { level: "warn", what: "Codex", detail: "not installed. Employees can't run on it", fix: "npm install -g @openai/codex" }, { level: "warn", what: "GitHub CLI", detail: "installed but not signed in", fix: "gh auth login" }],
    settings: { name: "snowztech", repo: "/Users/you/dev/snowztech", data: "/Users/you/.onecorp/companies/snowztech", home: "/Users/you/.onecorp", ship: "github", forge: "github", forge_name: "GitHub", origin: "git@github.com:snowztech/notes.git", tickets: 22, people: 6, version: "0.1.0" },
    health: [{ level: "ok", what: "git", detail: "git version 2.47.1", fix: null }, { level: "ok", what: "Claude Code", detail: "2.3.4 (Claude Code)", fix: null },
      { level: "warn", what: "Codex", detail: "not installed. Employees can't run on it", fix: "npm install -g @openai/codex, then run codex once to sign in" },
      { level: "ok", what: "company snowztech", detail: "opens pull requests on git@github.com:snowztech/notes.git (Logged in to github.com account you)", fix: null }],
    check_repo: { ok: true, path: "/Users/you/dev/snowztech", branch: "main", suggested: "snowztech", used_by: [], forge: "GitHub", pr_word: "pull request", signed_in: true, account: "Logged in to github.com account you" },
    founder: { name: "Lucas", photo: null },
    save_founder: null,
    create_company: "acme",
    chat: { channel: "general", messages: [
      { id: 1, at: "2026-11-09 09:12:00", author: "founder", name: "Lucas", founder: true, text: "Morning team. Goal this week: magic link login and the pricing page live.", key: null, pending: false },
      { id: 2, at: "2026-11-09 09:40:00", author: "rui", name: "Rui Alves", text: "Plan is in your inbox as SNO-20. Setup first, then login and sync in parallel.", key: "SNO-20", pending: false },
      { id: 3, at: "2026-11-09 10:05:00", author: "maya", name: "Maya Okafor", text: "Heads up: magic links need an email provider. I'll stub it behind a trait for now.", key: "SNO-12", pending: false },
      { id: 4, at: "2026-11-09 10:06:00", author: "maya", name: "Maya Okafor", text: "@odile can you look at the token hashing when you review?", key: "SNO-12", pending: false },
      { id: 5, at: "2026-11-09 11:02:00", author: "odile", name: "Odile Martin", text: "Pricing page is up for review in SNO-11. Copy needs a look on mobile.", key: "SNO-11", pending: false },
      { id: 6, at: "2026-11-09 11:20:00", author: "founder", name: "Lucas", founder: true, text: "@tomas how far are the Stripe webhook retries?", key: null, pending: false },
      { id: 7, at: "2026-11-09 11:20:00", author: "tomas", name: "Tomás Reis", text: "", key: null, pending: true }] },
    person: { id: "tomas", name: "Tomás Reis", role: "Engineer", runtime: "codex", model: null, bio: "Fast, likes refactors a bit too much.", department: 1, current: { key: "SNO-15", title: "Retry failed Stripe webhooks", status: "doing" },
      activity: [{ at: "2026-11-09 10:50:00", kind: "tool", summary: "Edit src/billing/webhook.rs" }, { at: "2026-11-09 10:52:00", kind: "tool", summary: "Bash cargo test billing" }], tickets: [{ key: "SNO-15", title: "Retry failed Stripe webhooks", status: "doing", spend: "75k tok" }],
      memory: ["2026-11-09 Stripe retries for 3 days, make handlers idempotent instead of queueing"] },
  };
  data.review = { key: "SNO-11", title: "Pricing page", base: "main", branch: "onecorp/snowztech/sno-11", added: 5, removed: 1,
    files: [{ path: "web/src/pages/pricing.tsx", added: 4, removed: 1 }, { path: "README.md", added: 1, removed: 0 }],
    commits: [{ hash: "c3dbf20", author: "Odile Martin", subject: "SNO-11: pricing page" }],
    patch: "diff --git a/web/src/pages/pricing.tsx b/web/src/pages/pricing.tsx\nindex 1..2 100644\n--- a/web/src/pages/pricing.tsx\n+++ b/web/src/pages/pricing.tsx\n@@ -1,3 +1,6 @@\n export function Pricing() {\n-  return null\n+  return <>\n+    <Plan name=\"Free\" price={0} />\n+    <Plan name=\"Pro\" price={6} />\n+  </>\n }\ndiff --git a/README.md b/README.md\nindex 3..4 100644\n--- a/README.md\n+++ b/README.md\n@@ -1 +1,2 @@\n # Notes\n+Pricing: free, or $6 a month.\n", truncated: false };
  if (cmd in data) return Promise.resolve(data[cmd]);
  return Promise.reject("This is a preview. Open the OneCorp app to make changes.");
}
