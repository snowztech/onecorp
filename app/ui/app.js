// OneCorp app UI. It polls the company snapshot and renders the Company page and the Inbox.
// Every action calls a Tauri command, which runs the same library code as the CLI.
// Opened in a plain browser (no Tauri), it runs on sample data so the design can be checked quickly.

const invoke = window.__TAURI__ ? window.__TAURI__.core.invoke : preview;
// Room for the window buttons in the app, and in screenshots that draw them (?native).
if (window.__TAURI__ || /[?&]native\b/.test(location.search)) document.documentElement.classList.add("native");
const $ = s => document.querySelector(s);
document.getElementById("boot")?.insertAdjacentHTML("afterbegin", logo());
// The layout appears once the first render has decided it. If loading stalls, show it anyway.
// The app's version, from Tauri in the app and the sample in the browser preview.
let version = "";
(window.__TAURI__?.app?.getVersion?.() || Promise.resolve("0.1.0")).then(v => { version = v; if (isBooted && (S.welcome || S.sheet)) { render(); renderSheet(); } }).catch(() => {});
// A bug report on GitHub, with the version already in it, so nobody has to look it up.
const enc = t => encodeURIComponent(t).replace(/[!'()*]/g, c => "%" + c.charCodeAt(0).toString(16).toUpperCase());
const bugUrl = () => "https://github.com/snowztech/onecorp/issues/new?labels=bug&body=" + enc(
  `**What happened**\n\n\n**What you expected**\n\n\n**Steps to get there**\n1. \n\n---\nOneCorp ${version || "unknown version"} on macOS`);
let isBooted = false;
function booted() {
  if (isBooted) return;
  isBooted = true;
  const app = document.querySelector(".app");
  app?.classList.remove("booting"); app?.classList.add("arrive");
  document.getElementById("boot")?.remove();
}
setTimeout(booted, 4000);
const esc = s => String(s ?? "").replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]));
const first = n => String(n || "").split(" ")[0];

const S = { companies: [], company: null, snap: null, last: "", view: "company", drawer: null, person: null, addingDept: false, open: {}, channel: "general", chat: null, seen: {}, diffs: {}, wiki: null, hbEdit: false, setup: null, founder: undefined, profile: null, project: null, rail: false, needs: {}, pop: false, settings: null, health: null, confirm: null, sheet: null, shown: "" };

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

/* Faces: drawn from the name, so everyone keeps the same face. The mood follows what they're doing.
   Each person gets one of eight hues, soft in light and deep in dark, so the team has colour that sits with the UI. */
const HUES = [215, 12, 158, 262, 38, 330, 192, 95];
const hash = s => [...String(s)].reduce((h, c) => (h * 31 + c.charCodeAt(0)) >>> 0, 7);
const SKIN = ["#f3d5bd", "#e6b590", "#c88e64", "#9a6542", "#6b442b"];
const HAIR = ["#1e1916", "#4b301e", "#8b5b2c", "#d0a660", "#b4442f", "#70747d"];
function face(name, status) {
  const e = S.snap?.employees?.find(x => x.name === name);
  if (e?.photo) {
    const url = loadPhoto(e);
    if (url) return `<span class="av"><img src="${esc(url)}" alt=""></span>`;
  }
  return drawFace(e?.face || name, status);
}
// Photos load once per change, then come from here: the snapshot only carries a stamp.
const photos = {};
function loadPhoto(e) {
  const key = `${S.company}/${e.id}/${e.photo}`;
  if (key in photos) return photos[key];
  photos[key] = null;
  invoke("employee_photo", { company: S.company, id: e.id }).then(url => { photos[key] = url; if (!typing()) render(); }).catch(() => {});
  return null;
}
// Other face sets, drawn once per person and kept. Ours is the default. Each person keeps their
// colour, light enough under the ink to read in both themes.
const FACE_SETS = [["ours", "OneCorp", "Drawn here, with moods that follow their work."], ["lorelei", "Lorelei", "Fine-line portraits by Lisa Wischofsky."], ["notionists", "Notionists", "Ink sketches by Zoish."]];
const faceSet = () => { try { const f = localStorage.getItem("onecorp.faces"); return FACE_SETS.some(([k]) => k === f) ? f : "ours"; } catch { return "ours"; } };
const drawn = {};
function drawFace(name, status, set = faceSet()) {
  if (set !== "ours" && window.Faces) {
    const key = set + "|" + name;
    drawn[key] ||= window.Faces(set, String(name));
    return `<span class="av set-${set}" style="--h:${HUES[(hash(name) >>> 12) % 8]}">${drawn[key]}</span>`;
  }
  const h = hash(name), L = { skin: h % 5, hair: (h >>> 3) % 6, style: (h >>> 6) % 5, acc: (h >>> 9) % 4 }, hue = HUES[(h >>> 12) % 8];
  const mood = { working: "working", blocked: "blocked", idle: "sleep", queued: "neutral", done: "happy" }[status] || "neutral";
  const sk = SKIN[L.skin], hr = HAIR[L.hair], ink = "#18181b";
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
    `<path d="M18.5 27q0-15 13.5-15t13.5 15z" fill="#3f3f46"/><rect x="18" y="24" width="28" height="5" rx="2.5" fill="#27272a"/>`][L.acc];
  return `<span class="av"><svg viewBox="0 0 64 64" aria-hidden="true"><rect width="64" height="64" style="fill:hsl(${hue} var(--av-tile))"/>
    <g fill="${hr}">${back}</g><path d="M11 64q2-17 21-17t21 17z" style="fill:hsl(${hue} var(--av-shirt))" stroke="${ink}" stroke-width="1.6"/><rect x="28.5" y="40" width="7" height="9" rx="2" fill="${sk}"/>
    <circle cx="32" cy="30" r="13" fill="${sk}" stroke="${ink}" stroke-width="1.6"/><g fill="${hr}">${front}</g><g fill="${ink}" stroke="${ink}" stroke-linecap="round">${E[mood]}${M[mood]}</g>${A}</svg></span>`;
}

const RUNTIME = { claude: ["Claude Code", "#d97757"], codex: ["Codex", "#3a9b7a"] };
// The model's own name, like Opus 5.5. With none set, the agent runs your own default, so say which.
function modelName(runtime, model) {
  if (!model) { const d = S.models?.defaults?.[runtime]; return d ? `${d} (default)` : "Default"; }
  return (S.models?.[runtime] || []).find(m => m.id === model || (!model.includes("-") && m.id.includes(model)))?.name || model;
}
function chip(e) {
  const [label, color] = RUNTIME[e.runtime] || [e.runtime, "#888"];
  return `<span class="chip"><i style="background:${color}"></i>${esc(label)}${RUNTIME[e.runtime] ? " · " + esc(modelName(e.runtime, e.model)) : ""}</span>`;
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
  wiki: '<path d="M5 4.5A1.5 1.5 0 0 1 6.5 3H19v15H6.5A1.5 1.5 0 0 0 5 19.5z"/><path d="M5 19.5A1.5 1.5 0 0 0 6.5 21H19M9 7h6"/>',
  settings: '<circle cx="12" cy="12" r="3"/><path d="M19 12a7 7 0 0 0-.1-1.2l2-1.6-2-3.4-2.4 1a7 7 0 0 0-2-1.2L14 3h-4l-.5 2.6a7 7 0 0 0-2 1.2l-2.4-1-2 3.4 2 1.6a7 7 0 0 0 0 2.4l-2 1.6 2 3.4 2.4-1a7 7 0 0 0 2 1.2L10 21h4l.5-2.6a7 7 0 0 0 2-1.2l2.4 1 2-3.4-2-1.6c.1-.4.1-.8.1-1.2z"/>',
  coin: '<circle cx="12" cy="12" r="9"/><path d="M15 9.5c-.5-1-1.6-1.5-3-1.5-1.7 0-3 .8-3 2s1.3 1.7 3 2 3 .8 3 2-1.3 2-3 2c-1.4 0-2.5-.5-3-1.5M12 6v2M12 16v2"/>',
  spark: '<path d="M12 3v4M12 17v4M3 12h4M17 12h4M6 6l2.5 2.5M15.5 15.5 18 18M6 18l2.5-2.5M15.5 8.5 18 6"/>',
  eye: '<path d="M2.5 12S6 5.5 12 5.5 21.5 12 21.5 12 18 18.5 12 18.5 2.5 12 2.5 12z"/><circle cx="12" cy="12" r="3"/>',
  info: '<circle cx="12" cy="12" r="9"/><path d="M12 11v6M12 7.5v.5"/>',
  copy: '<rect x="8" y="8" width="12" height="12" rx="2"/><path d="M16 8V5a1 1 0 0 0-1-1H5a1 1 0 0 0-1 1v10a1 1 0 0 0 1 1h3"/>',
  user: '<circle cx="12" cy="8" r="4"/><path d="M4 21a8 8 0 0 1 16 0"/>',
  more: '<circle cx="5" cy="12" r="1"/><circle cx="12" cy="12" r="1"/><circle cx="19" cy="12" r="1"/>',
  check: '<path d="M5 12.5l4.5 4.5L19 7.5"/>',
  chevrons: '<path d="M8 9.5l4-4 4 4M8 14.5l4 4 4-4"/>',
  pause: '<path d="M9 5v14M15 5v14"/>',
  plus: '<path d="M12 5v14M5 12h14"/>',
  trash: '<path d="M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3"/>',
  bug: '<rect x="7" y="7" width="10" height="13" rx="5"/><path d="M12 7V4M9.5 4.5 11 7M14.5 4.5 13 7M3 12h4M17 12h4M4 18l3-2M20 18l-3-2M4 6l3 2M20 6l-3 2"/>',
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
  const n = waiting(), pages = { company: "Company", board: "Board", inbox: "Inbox", chat: "Chat", wiki: "Wiki" };
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
  $("#title").textContent = S.profile ? (S.founder ? "Your profile" : "Welcome") : S.setup ? "New company" : { inbox: "Inbox", wiki: "Wiki", board: "Board", chat: "Chat" }[S.view] || (S.company ? "Company" : "OneCorp");
  $("#new-ticket").hidden = !!S.setup || !!S.profile || !s;
  // Today's spend, a glance from every page: the amount, a meter, the budget. Words only on hover.
  const sp = $("#top-spend");
  sp.hidden = $("#new-ticket").hidden;
  if (!sp.hidden) {
    const b = s.spend.budget, pct = b > 0 ? Math.min(100, (s.spend.today / b) * 100) : 0;
    sp.className = `top-spend ${pct >= 100 ? "over" : pct >= 80 ? "near" : ""}`;
    sp.title = b > 0 ? `Spent today, of a ${money(b)} daily budget` : "Spent today. Set a daily budget in Spend";
    sp.innerHTML = `<span>${money(s.spend.today)}</span>${b > 0 ? `<span class="meter"><i style="width:${pct}%"></i></span><span class="of">${money(b)}</span>` : ""}`;
  }
}

/* Company page: you on top, departments beneath as rooms of desks */
// The agent someone runs on, as a small badge on their avatar.
function agentBadge(runtime) {
  const a = AGENTS[runtime];
  return a ? `<span class="agent-badge" style="--c:${a.color}" title="Works on ${esc(a.name)}">${agentMark(runtime)}</span>` : "";
}
function desk(e, lead) {
  const show = e.bubble ? "show" : "";
  const works = (RUNTIME[e.runtime]?.[0] || e.runtime) + (RUNTIME[e.runtime] ? ` · ${modelName(e.runtime, e.model)}` : "");
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
  const facts = [`${s.employees.length} ${s.employees.length === 1 ? "person" : "people"}`, ...(working ? [`<span class="ok">${working} working</span>`] : [])];
  const items = needItems(s);
  const needs = items.slice(0, 3).map(i => `<button class="need" data-goto="${esc(i.to)}">${face(i.name, "neutral")}<span><b>${esc(first(i.name))}</b> ${esc(i.text)}</span></button>`).join("");
  const needsCard = `<section class="side-sec needs-sec"><h3 class="label">Needs you ${items.length ? `<span class="badge">${items.length}</span>` : ""}</h3>
    ${items.length ? `<div class="needs">${needs}</div><button class="link-btn more-in" data-view="inbox">Open Inbox →</button>` : `<p class="all-clear">${icon("check")}All clear, nothing waiting on you</p>`}</section>`;
  // Nobody working and nothing waiting: say what you could do next.
  const busy = s.employees.some(e => ["working", "queued", "blocked"].includes(e.status)) || s.kickoff.planning.length;
  const free = s.employees.find(e => e.status === "idle");
  const idle = !busy && s.tickets.length && free
    ? `<p class="idle">Your team is free · <button class="link-btn" data-plan-new="1">Start a project</button> · <button class="link-btn" data-new-ticket="${esc(free.id)}">Give ${esc(first(free.name))} a ticket</button></p>` : "";
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
/* Settings: sections on the left, the section on the right, and changes save as you make them. */
const SECTIONS = [["company", "Company", "company"], ["spend", "Spending", "coin"], ["agents", "Integrations", "spark"], ["appearance", "Appearance", "eye"], ["you", "You", "user"], ["about", "About", "info"]];
function renderSheet() {
  const el = $("#sheet");
  el.hidden = !S.sheet;
  if (!S.sheet) return (el.innerHTML = "");
  const tab = SECTIONS.some(([k]) => k === S.sheet) ? S.sheet : "company";
  const body = { company: companySection, spend: spendTab, agents: agentsSection, appearance: appearanceSection, you: youSection, about: aboutSection }[tab]();
  el.innerHTML = `<div class="sheet settings-sheet" role="dialog" aria-modal="true" aria-label="Settings">
    <nav class="set-nav"><h2>Settings</h2>${SECTIONS.map(([k, n, ic]) => `<button data-sheet="${k}" aria-current="${k === tab}">${icon(ic)}${n}</button>`).join("")}</nav>
    <div class="set-main"><header class="set-h"><h3>${SECTIONS.find(([k]) => k === tab)[1]}</h3><button class="close" data-sheet-close="1" aria-label="Close">×</button></header>
      <div class="sheet-body">${body}</div></div></div>`;
}
const saved = () => toast("Saved");
const theme = () => { try { return localStorage.getItem("onecorp.theme") || "system"; } catch { return "system"; } };
function setTheme(t) {
  try { t === "system" ? localStorage.removeItem("onecorp.theme") : localStorage.setItem("onecorp.theme", t); } catch {}
  if (t === "system") delete document.documentElement.dataset.theme; else document.documentElement.dataset.theme = t;
  // The title bar follows too, so the traffic lights sit on the right colour.
  window.__TAURI__?.window?.getCurrentWindow().setTheme(t === "system" ? null : t).catch(() => {});
}
function companySection() {
  const st = S.settings;
  if (!st) return `<p class="muted">Opening…</p>`;
  const look = S.looks?.[S.company] || {};
  const modes = [["local", "On this Mac"], ["github", "GitHub"], ["gitlab", "GitLab"]].filter(([m]) => m === "local" || m === st.forge || m === st.ship);
  const reveal = path => window.__TAURI__?.opener ? `<button class="link-btn" data-reveal="${esc(path)}">Show in Finder</button>` : "";
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
    <h3 class="label repos-label">Repos</h3>
    ${(st.repos || []).map(r => `<div class="repo-set">
      <div class="repo-h"><b>${esc(r.name)}</b>${r.main ? `<span class="pill-state">primary</span>` : ""}<span class="mono muted" title="${esc(r.path)}">${esc(r.path)}</span>${reveal(r.path)}${r.main ? "" : `<button class="link-btn" data-repo-rm="${esc(r.name)}">Remove</button>`}</div>
      <label class="cmd"><span>Build and test${r.custom ? ` · <button class="link-btn" type="button" data-commands-reset="${esc(r.name)}">use what the repo suggests</button>` : ""}</span>
        <input class="mono" data-commands="${esc(r.name)}" value="${esc(r.commands.join(", "))}" placeholder="cargo test, npm test" autocomplete="off" spellcheck="false" aria-label="Build and test commands for ${esc(r.name)}"></label></div>`).join("")}
    <div class="su-row add-repo">${window.__TAURI__?.dialog ? `<button class="btn" type="button" data-repo-add="pick">Add a repo…</button>` : ""}<input id="repo-add-path" class="mono" placeholder="/Users/you/dev/your-api" autocomplete="off" spellcheck="false"><button class="btn" type="button" data-repo-add="path">Add</button></div>
    <div class="row-set"><div><b>Data</b><span class="mono">${esc(st.data)}</span></div>${reveal(st.data)}</div>
    <h3 class="label danger-label">Danger zone</h3>${del}`;
}
// What fixes a tool: a button that opens Terminal running the command, and the command itself to copy.
const fixRow = (fix, state) => `<div class="fix"><button class="btn sm" type="button" data-term="${esc(fix)}">${state === "signin" ? "Sign in" : "Install"}</button><code>${esc(fix)}</code><button class="link-btn" type="button" data-copy="${esc(fix)}">Copy</button></div>`;
function agentsSection() {
  const r = S.ready, find = w => (r || []).find(h => h.what === w);
  const tools = [["Claude Code", "claude"], ["Codex", "codex"], ["GitHub CLI", "gh"]];
  const card = ([what, key]) => {
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
function aboutSection() {
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
function appearanceSection() {
  const team = (S.snap?.employees || []).slice(0, 5).map(e => e.face || e.name);
  const people = team.length ? team : ["Odile Martin", "Maya Okafor", "Tomás Reis", "Rui Alves", "Lina Park"];
  const cur = faceSet();
  return `<div class="row-set"><div><b>Theme</b><span>Follows your Mac unless you pick one.</span></div>
      <div class="seg">${[["system", "System"], ["light", "Light"], ["dark", "Dark"]].map(([k, n]) => `<button data-theme-pick="${k}" aria-pressed="${k === theme()}">${n}</button>`).join("")}</div></div>
    <div class="row-set stack"><div><b>Faces</b><span>How your team looks everywhere. Photos you picked stay.</span></div>
      <div class="face-sets">${FACE_SETS.map(([k, n, d]) => `<button type="button" class="face-set" data-face-set="${k}" aria-pressed="${k === cur}">
        <span class="fs-faces">${people.map(p => drawFace(p, "neutral", k)).join("")}</span><b>${n}</b><span>${d}</span></button>`).join("")}</div></div>`;
}
function youSection() {
  const f = S.founder;
  const forget = S.confirm === "forget"
    ? `<div class="confirm"><p>OneCorp forgets your name and photo and asks again. Your companies stay.</p><div class="acts"><button class="btn danger" data-forget="1">Forget me</button><button class="btn ghost" data-confirm="">Cancel</button></div></div>`
    : `<div class="row-set"><div><b>Forget your profile</b><span>Your companies stay.</span></div><button class="btn danger-ghost" data-confirm="forget">Forget…</button></div>`;
  const pic = window.__TAURI__?.dialog ? `<button type="button" class="pf-pic pick-pic" data-you-photo="1" title="Change photo">${me()}</button>` : `<span class="pf-pic">${me()}</span>`;
  return `<form id="you-form" class="row-set you-form"><div class="who-row">${pic}<div class="field"><label for="pf-name">Your name</label><input id="pf-name" value="${esc(f?.name || "")}" autocomplete="off"><span class="fine">Your team knows you by this name.</span></div></div>
      ${f?.photo ? `<button class="link-btn" type="button" data-you-nophoto="1">Remove photo</button>` : ""}</form>
    <h3 class="label danger-label">Danger zone</h3>${forget}`;
}
async function openSettings(tab = "company") {
  const opening = !S.sheet;
  S.sheet = tab === "health" ? "agents" : tab; S.confirm = null; S.customBudget = false; S.pop = false; renderPop(); renderSheet();
  // The entrance plays once, when the sheet opens, not each time its content changes.
  if (opening) { const el = $("#sheet"); el.classList.add("opening"); setTimeout(() => el.classList.remove("opening"), 300); }
  await loadSection(S.sheet);
}
async function loadSection(tab) {
  // Loaded fresh for the company you're on: a switch since last time must never show the old one.
  if (S.settings?.name !== S.company) { S.settings = null; renderSheet(); try { S.settings = await invoke("settings", { company: S.company }); } catch (e) { toast(String(e), true); } }
  if (tab === "spend" || tab === "agents") await loadSpend();
  if (tab === "agents" && !S.ready) S.ready = await invoke("readiness").catch(() => []);
  renderSheet();
}
async function loadSpend() {
  try { S.spend = await invoke("spend_report", { company: S.company }); } catch (e) { toast(String(e), true); }
  renderSheet();
}
/* Spend: today against the budget, two weeks of days, and where it goes, by person, project and agent. */
function spendTab() {
  const r = S.spend;
  if (!r) return `<p class="muted">Adding it up…</p>`;
  const t = r.totals, b = r.budget, pct = b > 0 ? Math.min(100, (t.today.cost / b) * 100) : 0;
  const any = r.days.filter(d => d.cost > 0).length >= 2, top = Math.max(...r.days.map(d => d.cost), b || 0, 0.01);
  const today = r.days[r.days.length - 1]?.day;
  const presets = [0, 5, 10, 25, 50], custom = b > 0 && !presets.includes(b);
  const row = (lead, w, extra = "") => `<tr><td>${lead}</td><td class="num">${money(w.today.cost)}</td><td class="num">${money(w.week.cost)}</td><td class="num">${money(w.all.cost)}</td>${extra}</tr>`;
  const head = (extra = "") => `<thead><tr><th></th><th class="num">Today</th><th class="num">7 days</th><th class="num">All time</th>${extra}</tr></thead>`;
  const people = r.people.filter(p => p.all.cost > 0 || p.cap > 0);
  return `<div class="spend-now"><span class="label">Today</span><b>${money(t.today.cost)}</b>
      ${b > 0 ? `<span class="meter big ${pct >= 100 ? "over" : pct >= 80 ? "near" : ""}"><i style="width:${pct}%"></i></span><span class="muted">${Math.round(pct)}% of ${money(b)}</span>` : ""}</div>
    <div class="row-set"><div><b>Daily budget</b><span>The company pauses there until tomorrow.</span></div>
      <div class="seg">${presets.map(p => `<button data-budget="${p}" aria-pressed="${p === b && !S.customBudget}">${p ? `$${p}` : "None"}</button>`).join("")}<button data-budget-custom="1" aria-pressed="${custom || !!S.customBudget}">${custom ? money(b) : "Other"}</button></div></div>
    ${S.customBudget ? `<form id="budget-form" class="row-set"><div><b>Your budget</b><span>Press Enter to save.</span></div><label class="money-in inline"><span>$</span><input id="budget-in" type="number" min="0" step="1" value="${b || ""}" placeholder="12"><span class="unit">a day</span></label></form>` : ""}
    ${S.settings ? (() => {
      // Says exactly which model runs, so there's nothing to guess about "lighter".
      const t = S.settings.talk || "balanced", names = (S.settings.talk_models?.[t] || []).join(" and ");
      const line = t === "same" ? "Each person's own model answers in chat and decides as a lead." : `${names || "A smaller model"} answers in chat and decides for leads. Tickets use each person's model.`;
      return `<div class="row-set"><div><b>Chat and lead decisions</b><span>${esc(line)}</span></div>
        <div class="seg">${[["cheapest", "Cheapest"], ["balanced", "Balanced"], ["same", "Their own"]].map(([m, n]) => `<button data-talk="${m}" aria-pressed="${m === t}">${n}</button>`).join("")}</div></div>`;
    })() : ""}
    ${any ? `<div class="chart" role="img" aria-label="Spend over the last 14 days">${r.days.map(d => `<div class="bar ${d.day === today ? "today" : ""}" title="${esc(d.day)}: ${money(d.cost)}"><i style="height:${Math.max(d.cost > 0 ? 3 : 0, (d.cost / top) * 100)}%"></i></div>`).join("")}${b > 0 && b < top ? `<span class="budget-line" style="bottom:${(b / top) * 100}%"></span>` : ""}</div>
      <div class="chart-axis"><span>2 weeks ago</span><span>Today</span></div>` : `<p class="muted sp-empty">Nothing spent yet. Your team's runs add up here.</p>`}
    ${people.length ? `<h3 class="label sp-h">People</h3><table class="sp">${head('<th class="num">Cap</th>')}<tbody>${people.map(p => row(`<span class="who">${face(p.name, "neutral")}${esc(first(p.name))}</span>`, p, `<td class="num muted">${p.cap > 0 ? money(p.cap) : "—"}</td>`)).join("")}</tbody></table>` : ""}
    ${r.projects.some(p => p.all.cost > 0) ? `<h3 class="label sp-h">Projects</h3><table class="sp">${head()}<tbody>${r.projects.filter(p => p.all.cost > 0).map(p => row(esc(p.title), p)).join("")}</tbody></table>` : ""}
    ${any && r.agents.length ? `<h3 class="label sp-h">Agents</h3><table class="sp">${head()}<tbody>${r.agents.map(a => row(`<span class="who"><span class="tool-ic sm" style="--c:${AGENTS[a.runtime]?.color || "#888"}">${agentMark(a.runtime)}</span>${esc(AGENTS[a.runtime]?.name || a.runtime)}${a.estimated ? ' <span class="est" title="Estimated from tokens">est.</span>' : ""}</span>`, a)).join("")}</tbody></table>` : ""}`;
}

/* Everything waiting on you, most urgent first. The Company page, the dock badge and notifications all use it. */
function needItems(s) {
  return [
    ...s.reviews.filter(r => r.kind === "plan").map(r => ({ to: r.key, name: r.name, text: `planned ${r.title}. Review the plan` })),
    ...s.inbox.filter(q => q.blocking).map(q => ({ to: "q" + q.id, name: q.name, text: q.question })),
    ...(s.failed || []).map(t => ({ to: t.key, name: t.name, text: `${t.key} stopped. Retry it?` })),
    ...s.reviews.filter(r => r.kind !== "plan").map(r => ({ to: r.key, name: r.name, text: `${r.title} is ready for review` })),
    ...s.inbox.filter(q => !q.blocking).map(q => ({ to: "q" + q.id, name: q.name, text: q.kind === "wiki" ? `proposes: ${q.question}` : q.question }))];
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
  budgetAlert(snap.company, snap.spend);
  badgeAll();
}
// Each budget moment is said once a day: the 80% heads-up, then the pause.
const budgetSaid = {};
function budgetAlert(company, sp) {
  if (!sp?.budget) return;
  const was = budgetSaid[company], now = sp.paused_by_budget ? "paused" : sp.warn ? "warn" : "";
  budgetSaid[company] = now;
  if (was === undefined || now === was || !now) return;
  notify(`${company} · budget`, now === "paused" ? `Reached the ${money(sp.budget)} daily budget. Nothing new starts until tomorrow.` : `80% of today's ${money(sp.budget)} budget is spent.`);
}
// Dollars the way people read them: cents under $100, whole dollars above.
const tokensText = n => !n ? "no tokens" : n >= 1e6 ? `${(n / 1e6).toFixed(1)}M tokens` : n >= 1e3 ? `${Math.round(n / 1e3)}k tokens` : `${n} tokens`;
const money = x => `$${x >= 100 ? Math.round(x).toLocaleString() : Number(x || 0).toFixed(2)}`;
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
    budgetAlert(a.company, a.spend);
  }
  S.spendAll = all.reduce((t, a) => t + (a.spend?.today || 0), 0) + (S.snap?.spend?.today || 0);
  badgeAll(); renderSide();
}
function badgeAll() {
  const total = Object.values(S.needs).reduce((a, b) => a + b, 0);
  window.__TAURI__?.window?.getCurrentWindow?.().setBadgeCount(total || undefined).catch(() => {});
  // The same count next to the menu bar icon, for when the window is closed.
  if (window.__TAURI__) invoke("set_tray_count", { count: total }).catch(() => {});
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
  // Only real work counts: planning and the wiki your lead writes at launch are not "work" yet.
  const s = S.snap, work = s.tickets.filter(t => !t.kind || t.kind === "work");
  const firstDept = s.departments.find(d => d.id !== null);
  const plan = (s.projects || [])[0];
  const steps = [
    { done: s.employees.length > 0, title: "Hire your team", sub: "Who works here, and on which AI.", act: firstDept ? `<button class="btn primary" data-hire="${firstDept.id}">Hire someone</button>` : `<button class="btn primary" data-add-dept="1">Add a department</button>` },
    { done: !!s.kickoff.brief, title: "Read your wiki", sub: "What we're building, the codebase and its conventions, written from your repo. Fix anything that's off.",
      act: s.tickets.some(t => t.kind === "wiki" && ["todo", "queued", "doing"].includes(t.status)) ? `<span class="muted">Being written…</span>` : `<button class="btn primary" data-view="wiki">Open the wiki</button>` },
    { done: !!plan, title: "Start your first project", sub: "Say what you want. Your planner breaks it into tickets with owners.", act: `<button class="btn primary" data-plan-new="1">New project</button>` },
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
  return `<div class="dhead"><div><h2>New project</h2><span class="muted">A feature or a goal. Your planner breaks it into tickets with owners, and you approve the plan in your Inbox.</span></div><button class="close" data-close="1" aria-label="Close">×</button></div>
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
  const blocking = s.inbox.filter(q => q.kind !== "wiki" && q.blocking), other = s.inbox.filter(q => q.kind !== "wiki" && !q.blocking), books = s.inbox.filter(q => q.kind === "wiki");
  const plans = s.reviews.filter(r => r.kind === "plan"), work = s.reviews.filter(r => r.kind !== "plan");
  const cards = [...plans.map(planReview), ...(s.failed || []).map(stopped), ...blocking.map(q => question(q)), ...work.map(review), ...other.map(q => question(q)), ...books.map(wiki)];
  const leads = (s.with_leads || []).map(q => question(q, true));
  const zero = `<div class="empty-state calm"><span class="empty-ic">${icon("check")}</span><b>You're all caught up</b><span>Questions, plans and reviews from your team land here when they need you.</span></div>`;
  return `<div class="inbox">${cards.length ? cards.join("") : zero}
    ${leads.length ? `<h2 class="section">With your leads <small class="muted">They decide what they can and pass up the rest. Step in anytime.</small></h2>${leads.join("")}` : ""}</div>`;
}
function stopped(t) {
  return `<article class="ask stopped" id="in-${esc(t.key)}">${face(t.name, "blocked")}<div>
    <span class="kind">Stopped · ${esc(t.key)}</span><h4>${esc(t.title)}</h4>
    <p>${esc(first(t.name))}'s run ${t.why && !/^stopped before/.test(t.why) ? `failed: ${esc(t.why)}` : "stopped before it finished."}</p>
    ${S.confirm === "ticket:" + t.key
      ? `<div class="confirm"><p>Delete ${esc(t.key)}? Its log and unfinished branch go with it.</p>
          <div class="acts"><button class="btn danger" data-remove="${esc(t.key)}">Delete</button><button class="btn ghost" data-confirm="">Cancel</button></div></div>`
      : `<div class="acts"><button class="btn primary" data-retry="${esc(t.key)}">Retry</button><button class="btn ghost" data-confirm="ticket:${esc(t.key)}">Delete…</button></div>`}</div></article>`;
}
function question(q, withLead) {
  const opts = q.options.map(o => `<button class="btn primary" data-answer="${q.id}" data-val="${esc(o)}">${esc(o)}</button>`).join("");
  // A long default goes in the sentence above, so its button stays short.
  const def = q.default && !q.options.includes(q.default) ? `<button class="btn" data-answer="${q.id}" data-val="${esc(q.default)}">${q.default.length > 40 ? `Keep ${esc(first(q.name))}'s choice` : `Go with ${esc(q.default)}`}</button>` : "";
  const kind = withLead ? `With ${esc(first(q.lead_name || "their lead"))}` : q.blocking ? "Blocking" : "Question";
  const via = q.via_name ? `<p class="via"><b>${esc(first(q.via_name))}</b> passed this to you${q.note ? `: ${esc(q.note)}` : "."}</p>` : "";
  return `<article class="ask ${q.blocking ? "blocking" : ""} ${withLead ? "lead" : ""}" id="in-q${q.id}">${face(q.name, q.blocking ? "blocked" : "neutral")}<div>
    <span class="kind">${kind} · ${esc(q.key || "")}</span><div class="q-text">${md(q.question).html}</div>${via}
    <p>${esc(first(q.name))} ${q.blocking ? `stopped until ${withLead ? "someone" : "you"} answer${withLead ? "s" : ""}.` : q.default ? `went ahead with “${esc(q.default)}” for now.` : "is carrying on meanwhile."}</p>
    <div class="acts">${opts}${def}${q.blocking || withLead ? "" : `<button class="btn ghost" data-dismiss-q="${q.id}">Dismiss</button>`}</div>
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
    <div class="pt-main"><b>${esc(t.title)}</b>${t.repo ? `<span class="repo-tag">${esc(t.repo)}</span>` : ""}${t.description ? (() => {
      // Short details show whole. Long ones fold to a few lines until you open them.
      const long = t.description.length > 220 || t.description.split("\n").length > 3, open = !long || S.open["pd-" + t.key];
      return `<div class="pt-desc ${open ? "open" : ""}">${md(t.description).html}</div>${open ? "" : `<button class="link-btn inline more-link" type="button" data-more="pd-${esc(t.key)}">Show details</button>`}`;
    })() : ""}${t.after ? `<span class="after">after ${esc(t.after)}</span>` : ""}</div>
    <select data-plan-assign="${esc(t.key)}" aria-label="Who does ${esc(t.key)}"><option value="">Nobody yet</option>${people.map(e => `<option value="${esc(e.id)}" ${e.id === t.assignee ? "selected" : ""}>${esc(first(e.name))} · ${esc(e.role)}</option>`).join("")}</select>
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
function wiki(q) {
  return `<article class="ask" id="in-q${q.id}">${face(q.name, "neutral")}<div>
    <span class="kind">Wiki · ${esc(q.section)}</span><h4>${esc(q.question)}</h4><p>${esc(first(q.name))} thinks everyone should follow this from now on.</p>
    <div class="acts"><button class="btn primary" data-answer="${q.id}" data-val="yes">Add to the wiki</button><button class="btn" data-answer="${q.id}" data-val="no">Not for everyone</button></div>
    ${more(`ans-${q.id}`, "Add it in your own words", `<form class="reply" data-reply="${q.id}"><input id="ans-${q.id}" placeholder="The rule, as you'd put it"><button class="btn">Add</button></form>`)}</div></article>`;
}

/* Chat: #general and a channel per department. Your team posts as they work; @mention someone to ask them. */
function viewChat() {
  const s = S.snap, chans = s.channels || [], dms = s.dms || [];
  const dmRow = dms.find(d => d.channel === S.channel), person = dmRow && byId(dmRow.id);
  const ch = person ? { name: dmRow.channel, last: dmRow.last } : chans.find(c => c.name === S.channel) || chans[0];
  if (!ch) return "";
  S.seen[ch.name] = ch.last || 0;
  const dept = person ? null : s.departments.find(d => d.id === ch.department);
  const people = person ? [person] : (dept ? dept.members.map(byId) : s.employees).filter(Boolean);
  const lead = dept?.lead && byId(dept.lead);
  const unread = c => c.last && c.last > (S.seen[c.name || c.channel] || 0) ? '<i class="dot"></i>' : "";
  const list = chans.map(c => `<button class="chan" data-channel="${esc(c.name)}" aria-current="${c.name === ch.name}"><span class="hash">#</span>${esc(c.name)}${unread(c)}</button>`).join("");
  const STATE = { working: "Working", blocked: "Waiting on you", queued: "Up next", idle: "Free" };
  const dmList = dms.map(d => byId(d.id) && `<button class="chan dm" data-channel="${esc(d.channel)}" aria-current="${d.channel === ch.name}"><span class="with-agent">${face(byId(d.id).name, byId(d.id).status)}<i class="st st-${byId(d.id).status}" title="${STATE[byId(d.id).status] || ""}"></i></span>${esc(first(byId(d.id).name))}${unread(d)}</button>`).filter(Boolean).join("");
  const msgs = S.chat?.channel === ch.name ? S.chat.messages : null;
  let prev = null;
  const rows = (msgs || []).map(m => {
    const same = prev && prev.author === m.author && !prev.pending && !m.pending && sameMinutes(prev.at, m.at, 5);
    prev = m;
    const who = m.founder ? me() : face(m.name, m.pending ? "working" : "neutral");
    const body = m.pending ? `<p class="typing">writing<span><i></i><i></i><i></i></span></p>` : `<div class="msg-text">${chatText(m.text)}</div>`;
    const meta = `<header><b>${esc(m.founder ? m.name : first(m.name))}</b>${m.founder ? '<span class="you">you</span>' : ""}<time>${hhmm(m.at)}</time>${m.key ? `<a class="tkey" data-ticket="${esc(m.key)}">${esc(m.key)}</a>` : ""}</header>`;
    const del = m.pending ? "" : `<button class="msg-del ${S.msgDel === m.id ? "sure" : ""}" data-msg-del="${m.id}" title="Delete message" aria-label="Delete message">${S.msgDel === m.id ? "Delete?" : icon("trash")}</button>`;
    return `<li class="msg ${same ? "cont" : ""}">${same ? "<span></span>" : who}<div>${same ? "" : meta}${body}</div>${del}</li>`;
  }).join("");
  const empty = person
    ? `<li class="chat-empty"><b>Message ${esc(first(person.name))}</b>Just between you two. ${esc(first(person.name))} replies to every message and reads this before their next ticket.</li>`
    : `<li class="chat-empty"><b>Nothing in #${esc(ch.name)} yet</b>Your team posts here as they work. Mention someone with @ to ask them something.</li>`;
  const sub = person ? `${esc(person.role)} · ${STATE[person.status] || ""}` : dept ? `${lead ? "Led by " + esc(first(lead.name)) + " · " : ""}${people.length} ${people.length === 1 ? "person" : "people"}` : ch.name === "general" ? "Everyone at " + esc(s.company) : "";
  return `<div class="chat">
    <nav class="chans" aria-label="Channels"><h6>Channels</h6>${list}${dmList ? `<h6 class="dm-h">Direct messages</h6>${dmList}` : ""}</nav>
    <section class="room-chat">
      <header class="chat-h">${person ? `<span class="with-agent dm-face">${face(person.name, person.status)}${agentBadge(person.runtime)}</span>` : ""}<div><h3>${person ? esc(person.name) : "#" + esc(ch.name)}</h3><span class="muted">${sub}</span></div>
        ${person ? `<button class="btn ghost" data-person="${esc(person.id)}">Profile</button>` : members(ch.name, people, lead)}</header>
      <ul class="msgs" id="msgs">${msgs === null ? `<li class="chat-empty muted">Loading…</li>` : rows || empty}</ul>
      <form class="composer" id="say"><div class="compose-box"><div class="mpop" id="mpop" role="listbox" hidden></div>
        <input id="say-text" placeholder="${person ? `Message ${esc(first(person.name))}` : `Message #${esc(ch.name)}. Type @ to ask someone, @all for everyone`}" autocomplete="off" role="combobox" aria-controls="mpop">
        <p class="calls" id="calls"></p></div><button class="btn primary">Send</button></form>
    </section></div>`;
}
// Who's in a channel, like Slack: the face stack opens the list, with what each person is up to.
function members(channel, people, lead) {
  const STATE = { working: "Working", blocked: "Waiting on you", queued: "Up next", idle: "Free" };
  const open = S.members === channel;
  const rows = people.map(e => `<li><button class="mem" data-mention="${esc(e.id)}" title="Mention ${esc(first(e.name))}">${face(e.name, e.status)}
      <span><b>${esc(e.name)}${lead?.id === e.id ? ' <small class="muted">lead</small>' : ""}</b><small class="muted">${esc(e.role)} · ${STATE[e.status] || ""}</small></span></button>
      <button class="btn ghost sm" data-dm="${esc(e.id)}">Message</button></li>`).join("");
  return `<button class="faces" data-members="${esc(channel)}" aria-expanded="${open}" title="${esc(people.map(e => first(e.name)).join(", "))}">
      ${people.slice(0, 5).map(e => `<span class="face-btn">${face(e.name, e.status)}</span>`).join("")}<span class="faces-n">${people.length}</span></button>
    ${open ? `<div class="members-pop" role="dialog" aria-label="People in #${esc(channel)}"><h6>${people.length} ${people.length === 1 ? "person" : "people"}</h6><ul>${rows}</ul></div>` : ""}`;
}
// The database keeps UTC. Shown in your own time.
const utc = at => new Date(String(at).replace(" ", "T") + "Z");
const hhmm = at => { const d = utc(at); return isNaN(d) ? "" : d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }); };
/* Mentions: type @ for a list of your team, and see who'll reply before you send. */
const fold = t => String(t).normalize("NFD").replace(/[\u0300-\u036f]/g, "").toLowerCase();
// Everyone in the channel you're in: the whole company in #general, a department's people in theirs.
function channelPeople() {
  const dm = (S.snap.dms || []).find(d => d.channel === S.channel);
  if (dm) return [byId(dm.id)].filter(Boolean);
  const ch = (S.snap.channels || []).find(c => c.name === S.channel), dept = S.snap.departments.find(d => d.id === ch?.department);
  return (dept ? dept.members.map(byId) : S.snap.employees).filter(Boolean);
}
// Who a message calls, the same way the core reads it: @firstname, or @all for the channel.
function calledBy(text) {
  const team = S.snap.employees, out = [];
  if (String(S.channel).startsWith("dm:")) out.push(...channelPeople());
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
  const quiet = !who.length && input.value.trim() && !String(S.channel).startsWith("dm:");
  calls.innerHTML = who.length ? `<span class="calls-faces">${who.slice(0, 4).map(e => face(e.name, "neutral")).join("")}</span><span>${names} will reply here</span>`
    : quiet ? `<span class="muted">No one is mentioned, so no one will reply. Add @ to ask someone.</span>` : "";
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
// Agents write Markdown, so chat reads it like Slack does: code, bold, italics, links and code
// blocks. Mentions and keys are only picked out of plain text, never inside code.
function chatText(text) {
  const asking = String(text).match(/\n\(Asking (.+)\.\)$/);
  if (asking) {
    const toYou = /founder/.test(asking[1]);
    const note = toYou ? `<span class="msg-meta">Asked you · <button class="link-btn inline" data-view="inbox">Answer in Inbox</button></span>` : `<span class="msg-meta">Asked ${chatText(asking[1])}</span>`;
    return chatText(String(text).slice(0, asking.index)) + note;
  }
  const blocks = String(text).split(/```(?:\w+)?\n?/);
  return blocks.map((part, i) => {
    if (i % 2) return `<pre class="chat-code"><code>${esc(part.replace(/\n$/, ""))}</code></pre>`;
    return inline(part).split(/(<code>[\s\S]*?<\/code>|<a [^>]*>[\s\S]*?<\/a>)/).map((bit, j) => j % 2 ? bit
      : bit.replace(/@([\p{L}\d]+)/gu, '<span class="mention">@$1</span>').replace(/\b([A-Z]{2,5}-\d+)\b/g, '<a class="tkey" data-ticket="$1">$1</a>')).join("");
  }).join("");
}

/* Wiki: the company's shared knowledge, the file every employee reads before each ticket */
// A small Markdown renderer for the wiki: headings, lists, tables, quotes, code, bold, italics, links.
// Enough for a wiki page, and nothing that needs a library.
function inline(t) {
  return esc(t)
    .replace(/`([^`]+)`/g, "<code>$1</code>")
    .replace(/\*\*([^*]+)\*\*/g, "<b>$1</b>")
    .replace(/(^|[^*])\*([^*\s][^*]*)\*/g, "$1<em>$2</em>")
    .replace(/\[([^\]]+)\]\((https?:[^)\s]+)\)/g, '<a class="link" data-open-url="$2">$1</a>');
}
// A table row's cells. A | inside `code` stays part of the cell.
function cells(line) {
  const out = [];
  let cur = "", tick = false;
  for (const ch of line.trim().replace(/^\|/, "").replace(/\|$/, "")) {
    if (ch === "`") tick = !tick;
    if (ch === "|" && !tick) { out.push(cur.trim()); cur = ""; } else cur += ch;
  }
  return [...out, cur.trim()];
}
// What OneCorp writes when you skip the mission. Shown as a hint, not as if it were yours.
const MISSION_HINT = "What the company is for, in a sentence or two.";
function md(text) {
  const out = [], toc = [];
  let list = null, para = [], code = null, table = null;
  const flush = () => {
    if (para.length) {
      const t = para.join(" ");
      out.push(t === MISSION_HINT ? `<p class="hint">No mission yet. Edit the wiki to add one.</p>` : `<p>${inline(t)}</p>`);
      para = [];
    }
    if (list) out.push(`<${list.tag}>${list.items.map(i => `<li>${inline(i)}</li>`).join("")}</${list.tag}>`), (list = null);
    if (table) {
      const [head, ...rows] = table;
      out.push(`<div class="table-wrap"><table class="md"><thead><tr>${head.map(c => `<th>${inline(c)}</th>`).join("")}</tr></thead><tbody>${rows.map(r => `<tr>${r.map(c => `<td>${inline(c)}</td>`).join("")}</tr>`).join("")}</tbody></table></div>`);
      table = null;
    }
  };
  for (const line of String(text || "").split("\n")) {
    if (code !== null) {
      if (line.startsWith("```")) { out.push(`<pre><code>${esc(code.join("\n"))}</code></pre>`); code = null; } else code.push(line);
      continue;
    }
    if (line.startsWith("```")) { flush(); code = []; continue; }
    // Tables: a row of | cells, then a |---| line, then more rows.
    if (line.trim().startsWith("|")) {
      const row = cells(line);
      if (row.every(c => /^:?-{2,}:?$/.test(c))) continue;
      if (!table) flush();
      (table ||= []).push(row);
      continue;
    }
    if (table) flush();
    if (/^\s*(-{3,}|\*{3,})\s*$/.test(line)) { flush(); out.push("<hr>"); continue; }
    const h = line.match(/^(#{1,4})\s+(.*)/);
    if (h) {
      flush();
      const n = h[1].length, id = "hb-" + toc.length;
      if (n === 2) toc.push([id, h[2]]);
      out.push(n === 1 ? `<h1>${inline(h[2])}</h1>` : n === 2 ? `<h2 id="${id}">${inline(h[2])}</h2>` : n === 3 ? `<h3>${inline(h[2])}</h3>` : `<h4>${inline(h[2])}</h4>`);
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
function viewWiki() {
  if (S.wiki === null) return `<p class="muted">Opening the wiki…</p>`;
  const proposals = S.snap.inbox.filter(q => q.kind === "wiki");
  const props = proposals.length ? `<section class="doc-props"><span class="label">Proposed by your team</span><div class="props">${proposals.map(q => `
      <div class="prop">${face(q.name, "neutral")}<div><b>${esc(first(q.name))}</b> <span class="muted">for ${esc(q.section)}</span><p>${esc(q.question)}</p>
        <div class="acts"><button class="btn primary" data-answer="${q.id}" data-val="yes">Add</button><button class="btn" data-answer="${q.id}" data-val="no">Not for everyone</button></div></div></div>`).join("")}</div></section>` : "";
  // Editing: Markdown on the left, the page as everyone will read it on the right.
  if (S.hbEdit) {
    return `<form class="wiki-edit" id="hb-raw">
      <header><div><span class="label">Editing the wiki</span><span class="muted">Markdown. ## starts a section. Everyone reads it from their next ticket.</span></div>
        <div class="acts"><button class="btn ghost" type="button" data-hb-edit="0">Cancel</button><button class="btn primary">Save</button></div></header>
      <div class="wiki-split"><textarea id="hb-text" spellcheck="false">${esc(S.wiki)}</textarea><article class="doc wiki" id="hb-preview">${md(S.wiki).html}</article></div></form>`;
  }
  // The page's own # title leads, then the page.
  const title = (S.wiki.match(/^#\s+(.*)$/m) || [])[1] || `${S.company} wiki`;
  const page = md(S.wiki.replace(/^#\s+.*$/m, ""));
  return `<div class="doc-wrap"><article class="doc">
      <div class="wiki-top"><span class="label">Wiki · everyone reads it before every ticket</span><button class="btn" data-hb-edit="1">Edit</button></div>
      <div class="wiki"><h1>${inline(title)}</h1></div>
      ${props}
      <div class="wiki">${page.html || `<p class="muted">Empty for now. Edit it to write your mission, decisions and conventions.</p>`}</div>
      <footer class="doc-foot"><button class="link-btn" data-hb-edit="1">Edit this page</button><button class="link-btn" data-learn="1">Update Codebase from the repo</button></footer>
    </article>
    <nav class="toc" aria-label="On this page"><span class="label">On this page</span>${page.toc.map(([id, n]) => `<button data-toc="${id}">${esc(n.replace(/[`*]/g, ""))}</button>`).join("")}</nav></div>`;
}

/* Board: every ticket by where it stands. Click one for its log and actions. */
const COLUMNS = [["Backlog", ["backlog"]], ["Up next", ["todo", "queued", "waiting"]], ["Working", ["doing"]], ["Needs you", ["blocked", "review", "failed"]], ["Done", ["done"]]];
const PILL = { blocked: ["Question", "warn"], review: ["Review", "info"], failed: ["Stopped", ""], waiting: ["Waiting", ""] };
const spent = t => t.spend && !/^(\$0\.00|0 tok)$/.test(t.spend);
function viewBoard() {
  const s = S.snap;
  if (!s.tickets.length) return `<div class="empty-state"><span class="empty-ic">${icon("board")}</span><b>No tickets yet</b>A project is a goal your planner breaks into tickets. A ticket is one task.<div class="acts center"><button class="btn primary" data-plan-new="1">New project</button><button class="btn" data-new-ticket="1">New ticket</button></div></div>`;
  const nameOf = id => byId(id)?.name || id || "";
  const projects = s.projects || [], current = projects.find(p => p.key === S.project);
  if (S.project && !current) S.project = null;
  const inView = t => !S.project || t.plan === S.project || t.key === S.project;
  const chip = p => {
    const pct = p.total ? Math.round((p.done / p.total) * 100) : 0;
    const state = p.status === "review" ? "to review" : ["todo", "queued", "doing"].includes(p.status) ? "planning" : `${p.done}/${p.total}`;
    return `<button class="pchip" data-project="${esc(p.key)}" aria-pressed="${S.project === p.key}">${esc(p.title)}<span class="prog"><i style="width:${pct}%"></i></span><span class="n">${state}</span></button>`;
  };
  const bar = `<div class="board-bar"><div class="pchips">${projects.length ? `<button class="pchip" data-project="" aria-pressed="${!S.project}">All work</button>${projects.map(chip).join("")}` : `<span class="muted">No projects yet. Start one to group its tickets here.</span>`}</div>
    <button class="btn" data-plan-new="1">${icon("plus")}New project</button></div>
    ${current?.goal ? `<p class="pgoal">${esc(current.goal)}</p>` : ""}`;
  const cols = COLUMNS.map(([title, statuses]) => {
    let ts = s.tickets.filter(t => statuses.includes(t.status) && inView(t));
    if (title === "Done") ts = ts.slice(-12);
    return `<section class="col"><h4>${title}<span>${ts.length}</span></h4>${ts.slice().reverse().map(t => `
      <button class="tk ${t.status} ${t.kind === "plan" ? "is-plan" : ""}" data-ticket="${esc(t.key)}"><span class="k">${esc(t.key)}${t.kind === "plan" ? " · plan" : ""}${PILL[t.status] ? `<span class="pill ${PILL[t.status][1]}">${PILL[t.status][0]}</span>` : ""}</span><span class="t">${esc(t.title)}</span>
        ${t.repo ? `<span class="repo-tag">${esc(t.repo)}</span>` : ""}${t.after && t.status !== "done" ? `<span class="after">after ${esc(t.after)}</span>` : ""}
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
        ${t.repo ? `<dt>Repo</dt><dd class="mono">${esc(t.repo)}</dd>` : ""}
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
      ${(S.snap?.repos || []).length > 1 ? `<div class="field"><label for="nt-repo">Repo</label><select id="nt-repo">${S.snap.repos.map(r => `<option value="${esc(r.name)}">${esc(r.name)}${r.main ? " (primary)" : ""}</option>`).join("")}</select></div>` : ""}
      <div class="field"><label for="nt-desc">Details <span class="muted">(optional)</span></label><textarea id="nt-desc" rows="5" placeholder="What done looks like, links, constraints"></textarea></div>
      ${(S.snap?.projects || []).length ? `<div class="field"><label for="nt-project">Project <span class="muted">(optional)</span></label><select id="nt-project"><option value="">None</option>${S.snap.projects.map(p => `<option value="${esc(p.key)}" ${p.key === S.project ? "selected" : ""}>${esc(p.title)}</option>`).join("")}</select></div>` : ""}
      <div class="field"><label for="nt-after">After</label><select id="nt-after"><option value="">Nothing, it can start right away</option>${open.map(t => `<option value="${esc(t.key)}">${esc(t.key)} ${esc(t.title)}</option>`).join("")}</select></div>
      <div class="acts"><button class="btn primary" data-nt="now">Start now</button><button class="btn" data-nt="later">Save for later</button></div>
    </form>`;
}
async function saveNewTicket(later) {
  const title = $("#nt-title").value.trim();
  if (!title) return $("#nt-title").focus();
  const args = { company: S.company, title, employee: $("#nt-who").value || null, description: $("#nt-desc").value, after: $("#nt-after").value || null, project: $("#nt-project")?.value || null, repo: $("#nt-repo")?.value || null, later };
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
  if (S.drawer.edit && S.drawer.form?.photoUrl) {
    d.innerHTML = head.replace(/<span class="with-agent">[\s\S]*?<\/span>(?=<div>)/, `<span class="with-agent"><span class="av"><img src="${esc(S.drawer.form.photoUrl)}" alt=""></span>${agentBadge(p.runtime)}</span>`);
  }
  if (S.drawer.edit) {
    d.innerHTML = `${head}<form class="dbody" id="emp-form">
      ${avatarPicker(S.drawer.form ||= { name: p.name, role: p.role, runtime: p.runtime, model: p.model, bio: p.bio, voice: p.voice, face: byId(p.id)?.face || p.name, photoUrl: null, photoPath: null, removePhoto: false, hasPhoto: !!byId(p.id)?.photo })}
      ${personFields(S.drawer.form)}
      <div class="field"><label for="pf-cap">Daily cap <span class="muted">(optional)</span></label><div class="money-in"><span>$</span><input id="pf-cap" type="number" min="0" step="0.5" value="${byId(p.id)?.cap > 0 ? byId(p.id).cap : ""}" placeholder="No cap"><span class="unit">a day. They stop picking up work when they reach it</span></div></div>
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
        <dt>Personality</dt><dd>${esc((p.voices || []).find(v => v.key === p.voice)?.label || "")}</dd>
        <dt>Works on</dt><dd>${chip(p)}</dd></dl>
      <div class="acts"><button class="btn" data-edit-person="1">Edit</button><button class="btn" data-new-ticket="${esc(p.id)}">Give ${esc(first(p.name))} a ticket</button><button class="btn" data-dm="${esc(p.id)}">Message</button></div>
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
  // OpenAI's mark, from simple-icons (CC0), so Codex reads as OpenAI at a glance.
  codex: '<g transform="translate(2.5 2.5) scale(.79)"><path d="M22.2819 9.8211a5.9847 5.9847 0 0 0-.5157-4.9108 6.0462 6.0462 0 0 0-6.5098-2.9A6.0651 6.0651 0 0 0 4.9807 4.1818a5.9847 5.9847 0 0 0-3.9977 2.9 6.0462 6.0462 0 0 0 .7427 7.0966 5.98 5.98 0 0 0 .511 4.9107 6.051 6.051 0 0 0 6.5146 2.9001A5.9847 5.9847 0 0 0 13.2599 24a6.0557 6.0557 0 0 0 5.7718-4.2058 5.9894 5.9894 0 0 0 3.9977-2.9001 6.0557 6.0557 0 0 0-.7475-7.0729zm-9.022 12.6081a4.4755 4.4755 0 0 1-2.8764-1.0408l.1419-.0804 4.7783-2.7582a.7948.7948 0 0 0 .3927-.6813v-6.7369l2.02 1.1686a.071.071 0 0 1 .038.052v5.5826a4.504 4.504 0 0 1-4.4945 4.4944zm-9.6607-4.1254a4.4708 4.4708 0 0 1-.5346-3.0137l.142.0852 4.783 2.7582a.7712.7712 0 0 0 .7806 0l5.8428-3.3685v2.3324a.0804.0804 0 0 1-.0332.0615L9.74 19.9502a4.4992 4.4992 0 0 1-6.1408-1.6464zM2.3408 7.8956a4.485 4.485 0 0 1 2.3655-1.9728V11.6a.7664.7664 0 0 0 .3879.6765l5.8144 3.3543-2.0201 1.1685a.0757.0757 0 0 1-.071 0l-4.8303-2.7865A4.504 4.504 0 0 1 2.3408 7.872zm16.5963 3.8558L13.1038 8.364 15.1192 7.2a.0757.0757 0 0 1 .071 0l4.8303 2.7913a4.4944 4.4944 0 0 1-.6765 8.1042v-5.6772a.79.79 0 0 0-.407-.667zm2.0107-3.0231l-.142-.0852-4.7735-2.7818a.7759.7759 0 0 0-.7854 0L9.409 9.2297V6.8974a.0662.0662 0 0 1 .0284-.0615l4.8303-2.7866a4.4992 4.4992 0 0 1 6.6802 4.66zM8.3065 12.863l-2.02-1.1638a.0804.0804 0 0 1-.038-.0567V6.0742a4.4992 4.4992 0 0 1 7.3757-3.4537l-.142.0805L8.704 5.459a.7948.7948 0 0 0-.3927.6813zm1.0976-2.3654l2.602-1.4998 2.6069 1.4998v2.9994l-2.5974 1.4997-2.6067-1.4997Z" fill="#fff"/></g>',
  gh: '<path d="M12 3.5a8.5 8.5 0 0 0-2.7 16.6c.4.1.6-.2.6-.4v-1.5c-2.4.5-2.9-1-2.9-1-.4-1-1-1.3-1-1.3-.8-.5.1-.5.1-.5.9.1 1.3.9 1.3.9.8 1.3 2 1 2.5.7.1-.6.3-1 .6-1.2-1.9-.2-3.9-.9-3.9-4.2 0-.9.3-1.7.9-2.3-.1-.2-.4-1.1.1-2.3 0 0 .7-.2 2.3.9a8 8 0 0 1 4.2 0c1.6-1.1 2.3-.9 2.3-.9.5 1.2.2 2.1.1 2.3.6.6.9 1.4.9 2.3 0 3.3-2 4-3.9 4.2.3.3.6.8.6 1.6v2.4c0 .2.2.5.6.4A8.5 8.5 0 0 0 12 3.5z" fill="#fff"/>',
};
// An agent's mark: its glyph, white on its color.
const agentMark = k => `<svg viewBox="0 0 24 24" aria-hidden="true">${GLYPHS[k] || ""}</svg>`;
function agentPicker(f) {
  const have = S.runtimes || Object.keys(AGENTS), a = AGENTS[f.runtime] || AGENTS.claude;
  // The exact models each agent offers, from the agent itself where it says. An older short name
  // like "opus" still lights up the model it stands for.
  const live = S.models?.[f.runtime];
  const def = S.models?.defaults?.[f.runtime];
  const list = live?.length ? [["", def ? `Default · ${def}` : "Default", "Your own setting for this agent"], ...live.map(m => [m.id, m.name, m.about])] : a.models;
  const same = v => v === (f.model || "") || (f.model && v && !f.model.includes("-") && v.includes(f.model));
  const models = list.some(([v]) => same(v)) ? list : [...list, [f.model, f.model]];
  return `<div class="field"><span class="flabel">Works on</span><div class="agents">${Object.entries(AGENTS).filter(([k]) => k !== "fake" || f.runtime === "fake").map(([k, x]) => {
      const ok = have.includes(k) || k === f.runtime;
      return `<button type="button" class="agent" data-agent="${k}" aria-pressed="${k === f.runtime}" ${ok ? "" : 'disabled title="Not installed on this Mac. Settings shows how to install it"'}>
        <span class="glyph" style="--c:${x.color}">${agentMark(k)}</span><span><b>${x.name}</b><small>${ok ? x.by : "Not installed"}</small></span></button>`;
    }).join("")}</div><input type="hidden" id="pf-rt" value="${esc(f.runtime)}"></div>
    <div class="field"><span class="flabel">Model</span><div class="models">${models.map(([v, n, about]) => `<button type="button" class="mchip" data-model="${esc(v)}" aria-pressed="${same(v)}" ${about ? `title="${esc(about)}"` : ""}>${esc(n)}</button>`).join("")}</div><input type="hidden" id="pf-m" value="${esc(f.model || "")}"></div>`;
}
// The fields for a person, shared by hiring and editing, so both look and behave the same.
function avatarPicker(f) {
  // Variations lead with a number: a change at the start reshapes the whole face, at the end it barely moves.
  const base = f.name || "New hire", seeds = [base, ...[1, 2, 3, 4, 5, 6, 7].map(n => `${n} ${base}`)];
  const photo = f.photoUrl || (f.hasPhoto && !f.removePhoto);
  return `<div class="field"><span class="flabel">Avatar</span><div class="faces-pick">${seeds.map(sd => `<button type="button" class="fpick" data-face="${esc(sd)}" aria-pressed="${!photo && sd === (f.face || base)}" aria-label="Face ${esc(sd)}">${drawFace(sd, "neutral")}</button>`).join("")}</div>
    <div class="photo-acts">${window.__TAURI__?.dialog ? `<button class="link-btn" type="button" data-avatar-photo="1">${photo ? "Change photo" : "Use a photo instead"}</button>` : ""}${photo ? `<button class="link-btn" type="button" data-avatar-clear="1">Remove photo</button>` : ""}</div></div>`;
}
function personFields(f) {
  // Their current runtime is always listed, so saving never switches it by accident.
  const runtimes = [...new Set([...(S.runtimes || ["claude", "codex", "fake"]), ...(f.runtime ? [f.runtime] : [])])];
  return `<div class="field"><label for="pf-n">Name</label><input id="pf-n" value="${esc(f.name)}" placeholder="Noor Haddad" autocomplete="off"></div>
    <div class="field"><label for="pf-r">Role</label><input id="pf-r" value="${esc(f.role)}" placeholder="Engineer, Designer, Writer…" autocomplete="off"></div>
    ${agentPicker({ ...f, runtime: runtimes.includes(f.runtime) ? f.runtime : runtimes[0] })}
    ${voicePicker(f)}
    <div class="field"><label for="pf-b">How they work <span class="muted">(instructions, optional)</span></label><textarea id="pf-b" rows="3" placeholder="Replies in French. Bullet points in summaries. Always adds tests.">${esc(f.bio || "")}</textarea></div>`;
}
// Personality as chips, with a reply in that voice underneath, so you hear it before you pick.
// A new hire can keep the one that fits their role.
function voicePicker(f) {
  const list = S.voices || [], auto = S.drawer?.type !== "person", cur = f.voice || (auto ? "" : list[0]?.key);
  const opts = [...(auto ? [{ key: "", label: "Fits role", sample: "Picked to fit their role. Change it any time." }] : []), ...list];
  const sample = opts.find(v => v.key === cur)?.sample || "";
  return `<div class="field"><span class="flabel">Personality</span><div class="models">${opts.map(v => `<button type="button" class="mchip vchip" data-vpick="${v.key}" aria-pressed="${v.key === cur}">${esc(v.label)}</button>`).join("")}</div>
    <div class="vsample"><span class="vs-who">${face(f.face || f.name || "New hire", "neutral")}<b>${esc(first(f.name) || "They")}</b><span>in chat</span></span><p id="pf-vs">${esc(sample)}</p></div>
    <input type="hidden" id="pf-v" value="${esc(cur || "")}"></div>`;
}
const readPerson = () => ({ name: $("#pf-n").value.trim(), role: $("#pf-r").value.trim() || "Engineer", runtime: $("#pf-rt").value, model: $("#pf-m").value.trim() || null, bio: $("#pf-b").value.trim(), voice: $("#pf-v")?.value || null });
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
// The first thing a new founder sees: what OneCorp is, in a line, and one way forward.
function viewWelcome() {
  return `<div class="focus-wrap"><div class="welcome-hero">
    <span class="hero-mark">${logo()}</span>
    <h1>OneCorp</h1>
    <p class="lede">Run a company of AI employees, on your Mac.</p>
    <ul class="hero-points">
      <li><b>Hire a team</b><span>on Claude Code or Codex, each with a role and a personality</span></li>
      <li><b>They work in your repo</b><span>each on their own branch, with tests</span></li>
      <li><b>You run the company</b><span>answer questions, review, and approve what ships</span></li>
    </ul>
    <button class="btn primary big" data-welcome="1">Get started</button>
    <p class="fine">Everything stays on this Mac. There's no account.</p>
    ${version ? `<p class="fine hero-version">Version ${esc(version)}</p>` : ""}
  </div></div>`;
}
function viewProfile() {
  const skip = S.companies.length ? `<button class="link-btn" type="button" data-skip-setup="1">Skip, go to ${esc(S.companies[0])}</button>` : `<span class="fine">Next: connect your AI</span>`;
  // Companies already here: a welcome back, not the first-run journey. Your teams are waiting.
  const back = S.companies.length > 0 && !S.profile.inSetup;
  const names = S.companies.slice(0, 3).join(", ") + (S.companies.length > 3 ? ` and ${S.companies.length - 3} more` : "");
  const wrap = form => back ? `<div class="focus-wrap">${focusTop(false)}<div class="onb solo">${form}</div></div>` : onboarding("You", false, form);
  return wrap(`<form class="onb-main welcome" id="profile">
      <h1>${back ? "Welcome back" : "Who's running the company?"}</h1>
      <p class="muted">${back ? `${esc(names)} ${S.companies.length === 1 ? "is" : "are"} here as you left ${S.companies.length === 1 ? "it" : "them"}. Tell your team who they work for.` : "Your team works for you and knows you by name. Everything stays on this Mac. There's no account."}</p>
      <div class="me-row"><button type="button" class="pf-pic pick-pic" data-pf="pick" title="Add a photo">${pfPic(S.profile.name)}<span class="pick-hint">${hasPhoto() ? "Change" : "Photo"}</span></button><div class="field grow"><label for="pf-name">Your name</label>
        <input id="pf-name" value="${esc(S.profile.name)}" placeholder="Lucas" autocomplete="off">
        <p class="field-err" id="pf-err" hidden>Say what your team should call you.</p></div></div>
      <div class="photo-acts">${photoActs() || `<span class="fine">Add a photo anytime.</span>`}</div>
      <div class="su-nav">${back ? "<span></span>" : skip}<button class="btn primary">${back ? `Open ${esc(S.company || S.companies[0])}` : "Continue"}</button></div>
    </form>`);
}
function renderPop() {
  const p = $("#pop");
  p.hidden = !S.pop;
  $(".side-foot")?.setAttribute("aria-expanded", String(!!S.pop));
  if (!S.pop) return;
  if (S.pop === "menu") {
    p.innerHTML = `<div class="pop-menu"><div class="pm-head">${me()}<div><b>${esc(myName())}</b><span>Founder</span></div></div>
      ${S.spendAll !== undefined && S.companies.length > 1 ? `<div class="pm-note">${money(S.spendAll)} spent today across your ${S.companies.length} companies</div>` : ""}
      <button class="pm-item" data-settings-tab="you">${icon("user")}Edit profile</button>
      ${S.company ? `<button class="pm-item" data-settings="1">${icon("settings")}Settings<kbd>⌘,</kbd></button>` : ""}
      <hr><button class="pm-item quiet" data-pop="forget">${icon("logout")}Forget my profile…</button></div>`;
    return;
  }
  if (S.pop === "forget") {
    p.innerHTML = `<div class="pop-form"><b>Forget your profile?</b><p class="fine">OneCorp forgets your name and photo and asks again. Your companies stay.</p>
      <div class="acts end"><button class="btn" data-pop="menu">Cancel</button><button class="btn danger" data-forget="1">Forget me</button></div></div>`;
    return;
  }
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
    invoke("readiness").then(r => { if (S.setup) { S.setup.ready = r; render(); recheckTools(); } }).catch(() => {});
    S.setup = { step: 0, mode: "existing", path: "", repo: null, name: "", mission: "", runtime: real[0] || "fake", info, repoInfo,
      folder: "", parent: repoInfo.parent || "", owner: "", private: true, people: [], extra: [], color: PALETTE[S.companies.length % PALETTE.length], logo: null, logoUrl: null };
    setTemplate("startup");
  } catch (e) { return toast(String(e), true); }
  S.drawer = null;
  render();
}
// Picking a template fills the team with its people, which you then edit freely.
const TIERS = [["strongest", "Strongest"], ["balanced", "Balanced"], ["cheapest", "Cheapest"]];
// A copied person keeps their model while it matches one of the tiers, so the picker shows it.
const tierOf = (runtime, model) => TIERS.find(([t]) => S.models?.tiers?.[runtime]?.[t]?.id === model)?.[0];
function setTemplate(key) {
  const s = S.setup;
  // Another company's team: the same people, roles, departments and personalities.
  if (key.startsWith("co:")) {
    const from = (s.info.teams_from || []).find(x => x.company === key.slice(3));
    const ok = s.info.runtimes.filter(r => r !== "fake");
    s.template = key;
    s.people = (from?.hires || []).map(h => { const runtime = ok.includes(h.runtime) ? h.runtime : s.runtime, tier = runtime === h.runtime && tierOf(runtime, h.model);
      return { name: h.name, role: h.role, dept: h.dept || "", runtime, voice: h.voice || "", tier: tier || "balanced", model: tier ? h.model : "" }; });
    return;
  }
  const t = s.info.templates.find(x => x.key === key);
  s.template = key;
  s.people = t.hires.map(h => ({ name: h.name, role: h.role, dept: h.dept || "", runtime: t.runtime || s.runtime, voice: h.voice || "", tier: h.tier || "balanced" }));
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
  (s.extra || []).forEach((x, i) => { const v = val(`su-r${i}-name`); if (v !== undefined) x.name = v.trim(); });
  if (val("su-rmain-name") !== undefined) s.mainName = val("su-rmain-name").trim();
  s.people.forEach((p, i) => {
    for (const k of ["name", "role", "dept", "runtime", "voice"]) {
      const v = val(`su-p${i}-${k}`);
      if (v !== undefined) p[k] = v.trim();
    }
    // Picking a model drops one kept from another team: the tier decides from then on.
    const t = val(`su-p${i}-tier`);
    if (t !== undefined && t !== p.tier) { p.tier = t; p.model = ""; }
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
    // A folder you picked for this repo is where you keep projects: the next ones go there too.
    if (s.parent && s.parent !== s.repoInfo?.parent) invoke("set_projects_folder", { path: s.parent }).then(p => { s.repoInfo.parent = p; }).catch(() => {});
    toast(s.owner ? `Created ${s.folder}, here and on GitHub` : `Created ${s.folder}`);
    await checkRepo(path);
  } catch (e) { s.creating = false; render(); toast(String(e), true); }
}
const RUNTIME_NAMES = { claude: "Claude Code", codex: "Codex", fake: "Stand-in, no AI" };
function shipLine(r) {
  if (!r.forge) return "Approved work merges into the repo on your Mac.";
  if (r.signed_in) return `Finished work opens ${esc(r.pr_word)}s on ${esc(r.forge)}, and approving merges them.`;
  return `On ${esc(r.forge)}, but ${r.forge === "GitHub" ? "gh" : "glab"} isn't signed in, so work merges on your Mac for now.`;
}
function shipNote(r) {
  if (!r.forge) return `<p class="su-warn">Approved work merges into this repo on your Mac. Add a GitHub or GitLab remote later to ship as pull requests.</p>`;
  if (r.signed_in) return `<p class="su-ok">Finished work opens ${esc(r.pr_word)}s on ${esc(r.forge)}, and approving merges them. ${esc(r.account || "")}</p>`;
  const cli = r.forge === "GitHub" ? "gh" : "glab";
  return `<p class="su-warn">This repo is on ${esc(r.forge)}, but ${cli} isn't signed in, so work merges locally for now. Run <code>${cli} auth login</code>, then <code>onecorp ship ${r.forge.toLowerCase()}</code>.</p>`;
}
// A company's badge: its logo if it has one, otherwise its first letter on its color.
// Graphite first, like the OneCorp icon, then muted tones that sit with the zinc theme.
const PALETTE = ["#27272a", "#4772ae", "#ae5c47", "#47ae88", "#6d47ae", "#ae8847", "#ae477a", "#4799ae"];
function badge(name, look, cls = "") {
  if (look?.logo) return `<span class="logo ${cls} img"><img src="${esc(look.logo)}" alt=""></span>`;
  // No name yet: a quiet building, not a question mark.
  const mark = name ? esc(name[0].toUpperCase()) : icon("company");
  return `<span class="logo ${cls}" style="background:${esc(look?.color || PALETTE[0])};color:#fff">${mark}</span>`;
}
// The whole journey, shown on every step. A first launch also counts you.
// Back to the You step from setup: your name and photo as they are, and setup waits where it was.
function backToYou() {
  S.profile = { name: S.founder?.name || "", photoUrl: S.founder?.photo || null, photo: null, inSetup: true };
  render();
  $("#pf-name")?.focus();
}
// On the Connect step, tools are checked again when you come back from Terminal and every few
// seconds while one isn't connected, so a login shows up without a button. Only a changed
// result redraws, and the step has no fields to lose.
let rechecking = false, toolTimer = null;
const onConnect = () => S.setup && S.setup.step === 0 && !S.setup.launch && !S.profile;
const toolMissing = r => (r || []).some(h => ["Claude Code", "Codex", "GitHub CLI"].includes(h.what) && h.level !== "ok");
async function recheckTools() {
  const s = S.setup;
  clearTimeout(toolTimer);
  if (!onConnect() || rechecking) return;
  rechecking = true;
  const r = await invoke("readiness").catch(() => null);
  rechecking = false;
  if (r && S.setup === s && onConnect() && JSON.stringify(r) !== JSON.stringify(s.ready)) { s.ready = r; render(); }
  // One timer at a time, and only while something here still isn't connected.
  if (onConnect() && toolMissing(S.setup.ready)) toolTimer = setTimeout(recheckTools, 4000);
}
window.addEventListener?.("focus", recheckTools);
// Back from Terminal with Settings on Integrations: show what changed.
window.addEventListener?.("focus", async () => { if (S.sheet === "agents") { S.ready = await invoke("readiness").catch(() => S.ready); renderSheet(); } });
function journey(at) {
  const steps = [...(S.companies.length && !S.onboarding && at !== "You" ? [] : ["You"]), "Connect", "Company", "Repo", "Team", "Launch"], i = steps.indexOf(at);
  // Steps you've done are links back, so you can change anything on the way.
  const item = (n, k) => `<span class="n">${k < i ? icon("check") : k + 1}</span>${n}`;
  return `<ol class="onb-steps">${steps.map((n, k) => `<li class="${k === i ? "on" : k < i ? "done" : ""}">${k < i && S.setup && !S.setup.launch ? `<button type="button" data-step-to="${n}">${item(n, k)}</button>` : item(n, k)}</li>`).join("")}</ol>`;
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
// Your GitHub repos, looked up once in the background when the Repo step opens, so the list is
// ready by the time you ask for it.
function loadGithub() {
  const s = S.setup;
  if (!s || s.gh || s.ghLoading) return;
  s.ghLoading = true;
  invoke("github_repos").then(repos => ({ repos })).catch(e => ({ error: String(e) })).then(g => {
    s.ghLoading = false; s.gh = g;
    if (S.setup === s && s.mode === "github") render();
  });
}
// A path you change by clicking it. The app opens the folder picker, the browser preview types.
const pathField = (id, value, action, label) => window.__TAURI__?.dialog
  ? `<button type="button" class="path-field" data-su="${action}" aria-label="${esc(label)}" title="Click to pick another folder"><span class="mono">${esc(value)}</span><span class="path-hint">Change</span></button>`
  : `<input id="${id}" class="mono path-field" value="${esc(value)}" autocomplete="off" spellcheck="false" aria-label="${esc(label)}">`;
// The first repo you add is the main one. Every one after joins it.
async function addRepo(path) {
  path = String(path || "").trim();
  if (!path) return $("#su-add")?.focus();
  if (!S.setup.repo?.ok) return checkRepo(path);
  return addExtraRepo(path);
}
async function addExtraRepo(path) {
  const s = S.setup;
  path = String(path || "").trim();
  if (!path) return $("#su-add")?.focus();
  let r;
  try { r = await invoke("check_repo", { path }); } catch (e) { return toast(String(e), true); }
  if (!r.ok) return toast(r.problem, true);
  if (r.path === s.repo?.path || s.extra.some(x => x.path === r.path)) return toast("That repo is already in the list", true);
  // Named after its folder, like notes-api. Tickets and the planner call it that.
  const name = r.path.split("/").pop().toLowerCase().replace(/[^a-z0-9_]+/g, "-").replace(/^-|-$/g, "");
  s.extra.push({ path: r.path, name, branch: r.branch });
  render();
}
function viewSetup() {
  if (S.setup.launch) return viewLaunching();
  const s = S.setup, names = ["Connect", "Company", "Repo", "Team", "Launch"];
  // On the first step, Back goes to you, when you're part of this journey.
  const toYou = !s.step && (S.onboarding || !S.companies.length);
  const back = s.step || toYou ? `<button class="btn" type="button" data-su="back">Back</button>` : "<span></span>";
  const nav = (label, disabled) => `<div class="su-nav">${back}<button class="btn primary" ${disabled ? "disabled" : ""}>${label}</button></div>`;
  const picker = window.__TAURI__?.dialog;
  let body = "";
  const look = { color: s.color, logo: s.logoUrl };
  if (s.step === 0) {
    const r = s.ready, find = w => (r || []).find(h => h.what === w);
    const tools = [["Claude Code", "Anthropic's coding agent", "claude"], ["Codex", "OpenAI's coding agent", "codex"], ["GitHub CLI", "Ships finished work as pull requests", "gh"]];
    const card = ([what, about, key]) => {
      const h = find(what), state = !r ? "checking" : !h ? "missing" : h.level === "ok" ? "ok" : /not signed in/.test(h.detail) ? "signin" : "missing";
      const pill = { checking: "Checking…", ok: "Connected", signin: "Not signed in", missing: "Not installed" }[state];
      return `<div class="tool t-${state}"><span class="tool-ic" style="--c:${AGENTS[key]?.color || "#18181b"}">${agentMark(key)}</span>
        <div class="tool-main"><b>${esc(what)}</b><span>${esc(about)}</span>${h?.fix && state !== "ok" ? fixRow(h.fix, state) : ""}</div>
        <span class="pill-state">${pill}</span></div>`;
    };
    const git = find("git"), agents = r && tools.slice(0, 2).some(([w]) => find(w)?.level === "ok");
    body = `<h1>Connect your tools</h1><p class="muted">Signed in as you, on this Mac. OneCorp never sees your keys.</p>
      <h3 class="label tools-h">AI agents <span class="muted">· one is enough</span></h3><div class="tools">${tools.slice(0, 2).map(card).join("")}</div>
      <h3 class="label tools-h">Code hosting <span class="muted">· optional</span></h3><div class="tools">${tools.slice(2).map(card).join("")}</div>
      ${git && git.level !== "ok" ? `<p class="su-bad">git isn't installed. Run <code>${esc(git.fix || "xcode-select --install")}</code></p>` : ""}
      ${r && !agents ? `<p class="su-warn">Your employees need Claude Code or Codex to work. Install one, sign in, then check again.</p>` : ""}
      ${r && toolMissing(r) ? `<p class="fine watching"><i class="pulse"></i>Finish in Terminal and come back. This page updates by itself. <button class="link-btn inline" type="button" data-recheck="1">Check again</button></p>` : ""}
      ${nav("Continue", !r || !agents)}`;
  } else if (s.step === 1) {
    const prefix = (s.name || "acme").replace(/[^a-z0-9]/gi, "").slice(0, 3).toUpperCase() || "ACM";
    body = `<h1>Found your company</h1><p class="muted">Its name, its look, and what it's for. Your team sees all of it.</p>
      <div class="co-card">${badge(s.name || "?", look, "xl")}<div class="field grow"><label for="su-name">Name</label><input id="su-name" class="big" value="${esc(s.name)}" placeholder="snowztech" autocomplete="off" spellcheck="false">
        <span class="fine">Letters, digits, - and _. Tickets are numbered from it, like ${esc(prefix)}-1.</span></div></div>
      <div class="field"><span class="flabel">Logo</span><div class="look-row"><div class="swatches">${PALETTE.map(c => `<button type="button" class="swatch" data-su="color-${c}" aria-pressed="${c === s.color && !s.logoUrl}" style="--c:${c}" aria-label="Color ${c}"></button>`).join("")}</div>
        ${picker ? `<button class="btn" type="button" data-su="logo">${s.logoUrl ? "Change logo…" : "Upload a logo…"}</button>` : ""}${s.logoUrl ? `<button class="link-btn" type="button" data-su="logo-remove">Use the letter</button>` : ""}</div></div>
      <div class="field"><label for="su-mission">Mission <span class="muted">(optional, it opens the wiki)</span></label><input id="su-mission" value="${esc(s.mission)}" placeholder="Open source tools for building AI agents" autocomplete="off"></div>
      <div class="field"><label for="su-brief">What are you building? <span class="muted">(optional, it starts your wiki. Your lead fills in the rest from the repo)</span></label><textarea id="su-brief" rows="3" placeholder="A notes app for writers. Magic link login, sync, export to Markdown.">${esc(s.brief || "")}</textarea></div>
      ${nav("Continue")}`;
  } else if (s.step === 2) {
    const toList = `<button class="btn" type="button" data-su="mode-existing">Back to your repos</button>`;
    if (s.mode === "github") {
      const g = s.gh;
      const list = !g ? `<p class="gh-loading"><i class="spin"></i>Looking up your repos on GitHub…</p>`
        : g.error ? `<p class="su-warn">Sign in to GitHub with <code>gh auth login</code>, then come back here.</p>`
        : `<input id="gh-q" placeholder="Search ${g.repos.length} repos" autocomplete="off" spellcheck="false" aria-label="Search your repos">
           <ul class="gh-list">${g.repos.map(r => `<li data-name="${esc(r.name.toLowerCase())} ${esc((r.description || "").toLowerCase())}"><button type="button" data-su="clone:${esc(r.name)}" ${s.cloning ? "disabled" : ""}>
             <b>${esc(r.name)}</b>${r.private ? `<span class="pill-state">Private</span>` : ""}<span>${esc(r.description || "")}</span>${s.cloning === r.name ? `<em>Cloning…</em>` : ""}</button></li>`).join("")}</ul>`;
      const status = s.cloning ? `<p class="gh-loading"><i class="spin"></i>Cloning <b>${esc(s.cloning)}</b> into ${esc(s.repoInfo?.parent || "your dev folder")}… It shows in your list when it's done.</p>` : "";
      const where = `<p class="muted">Pick a repo, then choose where it goes. If it's already on this Mac, OneCorp uses that copy.</p>
        <div class="projects-dir"><span>Your projects folder</span>${pathField("projects-path", s.repoInfo?.parent || "", "projects-dir", "Your projects folder")}</div>`;
      const pick = s.pick ? `<div class="clone-pick"><b>Clone ${esc(s.pick.name)} into</b>
          ${pathField("clone-path", s.pick.path, "clone-dir", `Where to clone ${s.pick.name}`)}
          <div class="acts"><button class="btn primary" type="button" data-su="clone-go">Clone</button><button class="btn ghost" type="button" data-su="clone-cancel">Cancel</button></div></div>` : "";
      body = `<h1>Add from GitHub</h1>${where}${pick}${status}${s.pick ? "" : list}
        <div class="su-nav">${toList}</div>`;
    } else if (s.mode === "new") {
      const owners = s.repoInfo.owners || [];
      const github = owners.length
        ? `<label class="su-check" for="su-gh"><input type="checkbox" id="su-gh" data-su="toggle-gh" ${s.owner ? "checked" : ""}> Also create it on GitHub</label>
           ${s.owner ? `<div class="su-row"><select id="su-owner" aria-label="GitHub account or org">${owners.map(o => `<option ${o === s.owner ? "selected" : ""}>${esc(o)}</option>`).join("")}</select>
             <span class="mono muted">/${esc((s.folder || "your-repo").toLowerCase())}</span>
             <label class="su-check" for="su-private"><input type="checkbox" id="su-private" ${s.private ? "checked" : ""}> Private</label></div>` : ""}`
        : `<p class="su-warn">Sign in to GitHub with <code>gh auth login</code> to also create the repo there. You can add a remote later too.</p>`;
      body = `<h1>Start ${esc(s.name)} from scratch</h1><p class="muted">A new folder with git, a README and a first commit, ready for your team.</p>
        <div class="field"><label for="su-folder">Folder name</label><input id="su-folder" value="${esc(s.folder)}" placeholder="notes-app" autocomplete="off" spellcheck="false"></div>
        <div class="field"><label for="su-parent">Inside</label><div class="su-row">${picker ? `<button class="btn" type="button" data-su="parent">Choose…</button>` : ""}<input id="su-parent" value="${esc(s.parent)}" autocomplete="off" spellcheck="false"></div></div>
        ${github}
        <div class="su-nav">${toList}<button class="btn primary" type="button" data-su="create-repo" ${s.creating ? "disabled" : ""}>${s.creating ? "Creating…" : "Create the repo"}</button></div>`;
    } else {
      loadGithub();
      // One list: the first repo is the main one, the others sit beside it. Adding works the same
      // for both, so there's one way to do it.
      const r = s.repo, main = r?.ok ? r : null;
      const folder = p => p.split("/").pop();
      const odd = b => b && !["main", "master", "trunk", "develop"].includes(b);
      const row = (x, i) => `<li class="repo-row ${x.path === s.justAdded ? "just" : ""}">
          <span class="repo-name"><input id="${x.main ? "su-rmain-name" : `su-r${i}-name`}" class="mono" value="${esc(x.main ? (s.mainName ?? folder(x.path)) : x.name)}" aria-label="Name for ${esc(x.path)}" autocomplete="off" spellcheck="false">${x.main ? `<span class="pill-state">primary</span>` : ""}</span>
          <span class="mono muted repo-path" title="${esc(x.path)}">${esc(x.path)}</span>
          ${x.branch ? `<span class="repo-branch ${odd(x.branch) ? "odd" : ""}" title="New tickets start from this branch, whatever you have checked out">from ${esc(x.branch)}</span>` : "<span></span>"}
          <button class="x" type="button" data-su="rdel-${x.main ? "main" : i}" aria-label="Remove ${esc(folder(x.path))}">×</button></li>`;
      const rows = [main && { ...main, main: true }, ...s.extra].filter(Boolean);
      const offBranch = rows.filter(x => odd(x.branch));
      body = `<h1>Where does ${esc(s.name)} work?</h1><p class="muted">The repos your team works in. The first is the primary one, where planning starts. Your own checkouts are never touched.</p>
        <ul class="repo-list">${rows.length ? (main ? row(rows[0]) : "") + s.extra.map(row).join("") : `<li class="repo-empty">No repo yet. Add the one your team works in.</li>`}</ul>
        <div class="su-row repo-add">${picker ? `<button class="btn" type="button" data-su="pick">Choose a folder…</button>` : ""}<button class="btn" type="button" data-su="mode-github">From GitHub…</button>
          <input id="su-add" placeholder="or paste a path, like /Users/you/dev/app" autocomplete="off" spellcheck="false"><button class="btn" type="button" data-su="add-path">Add</button></div>
        ${r && !r.ok ? `<p class="su-bad">${esc(r.problem)}</p>` : ""}
        ${offBranch.map(x => `<p class="su-warn"><b>${esc(x.main ? (s.mainName ?? folder(x.path)) : x.name)}</b> has no main or master branch, so new tickets start from <code>${esc(x.branch)}</code>.</p>`).join("")}
        ${main ? `<p class="fine repo-ship">${shipLine(main)}</p>` : ""}
        ${main?.used_by?.length ? `<p class="fine">${esc(main.used_by.join(", "))} already works on this repo. They'll share it.</p>` : ""}
        <p class="fine"><button class="link-btn inline" type="button" data-su="mode-new">Start a new repo instead</button></p>
        ${nav("Continue")}`;
    }
  } else if (s.step === 3) {
    const real = s.info.runtimes.filter(x => x !== "fake");
    const rows = s.people.map((p, i) => `<li class="person-row"><span class="with-agent">${face(p.name || "?", "neutral")}${agentBadge(p.runtime)}</span>
      <input id="su-p${i}-name" value="${esc(p.name)}" placeholder="Name" aria-label="Name" autocomplete="off">
      <input id="su-p${i}-role" value="${esc(p.role)}" placeholder="Role" aria-label="Role" autocomplete="off">
      <input id="su-p${i}-dept" value="${esc(p.dept)}" placeholder="Department" aria-label="Department" list="su-depts" autocomplete="off">
      <select id="su-p${i}-voice" class="mini-voice" aria-label="Personality" title="How they write"><option value="">Fits role</option>${(s.info.voices || []).map(v => `<option value="${v.key}" ${v.key === p.voice ? "selected" : ""}>${esc(v.label)}</option>`).join("")}</select>
      <select id="su-p${i}-tier" class="mini-voice mini-model" aria-label="Model" title="Model">${TIERS.map(([t, n]) => `<option value="${t}" ${t === (p.tier || "balanced") ? "selected" : ""}>${esc(S.models?.tiers?.[p.runtime]?.[t]?.name || n)}</option>`).join("")}</select>
      <span class="mini-agents" role="group" aria-label="Works on">${real.map(k => `<button type="button" class="mini-agent" data-su-agent="${i}:${k}" aria-pressed="${k === p.runtime}" title="${esc(AGENTS[k]?.name || k)}" style="--c:${AGENTS[k]?.color || "#888"}">${agentMark(k)}</button>`).join("")}<input type="hidden" id="su-p${i}-runtime" value="${esc(p.runtime)}"></span>
      <button class="x" type="button" data-su="del-${i}" aria-label="Remove ${esc(p.name)}">×</button></li>`).join("");
    const depts = [...new Set(s.people.map(p => p.dept).filter(Boolean).concat(["Engineering", "Product", "Design", "Growth"]))];
    body = `<h1>Who's on the team to start?</h1><p class="muted">Start from a template, then change anything: names, roles, departments. The first person in a department leads it.</p>
      <div class="tpl-cards">${s.info.templates.map(x => {
        const faces = x.hires.length ? x.hires.slice(0, 5).map(h => face(h.name, "neutral")).join("") : `<span class="seat ghost"></span>`;
        return `<button type="button" class="tpl-card" data-tpl="${x.key}" aria-pressed="${x.key === s.template}">
          <span class="tpl-faces">${faces}</span><b>${esc(x.name)}</b><span>${esc(x.description)}</span></button>`;
      }).join("")}</div>
      ${(s.info.teams_from || []).length ? `<div class="copy-team"><span class="flabel">Or the same people as</span>${s.info.teams_from.map(t => `<button type="button" class="chip-btn" data-tpl="co:${esc(t.company)}" aria-pressed="${s.template === "co:" + t.company}"><span class="tpl-faces">${t.hires.slice(0, 4).map(h => face(h.name, "neutral")).join("")}</span>${esc(t.company)}</button>`).join("")}</div>` : ""}
      <ul class="people">${rows || `<li class="muted">Nobody yet. Add someone, or start empty and hire later.</li>`}</ul>
      <datalist id="su-depts">${depts.map(d => `<option value="${esc(d)}">`).join("")}</datalist>
      <button class="btn" type="button" data-su="add-person">+ Add someone</button>
      ${nav("Continue")}`;
  } else {
    const ships = s.repo.forge && s.repo.signed_in ? `As ${esc(s.repo.pr_word)}s on ${esc(s.repo.forge)}` : "By merging on your Mac";
    body = `<h1>Ready to launch</h1><p class="muted">Everything can change later. This is just where you start.</p>
      <div class="launch">
        <header>${badge(s.name, look, "xl")}<div><b>${esc(s.name)}</b><span>${esc(s.mission || "No mission yet")}</span></div></header>
        <dl class="kv"><dt>${s.extra.length ? "Repos" : "Repo"}</dt><dd class="mono">${esc(s.repo.path)} <span class="muted">on ${esc(s.repo.branch)}</span>${s.extra.map(x => `<br>${esc(x.path)} <span class="muted">as ${esc(x.name)}</span>`).join("")}</dd>
          <dt>Ships</dt><dd>${ships}</dd>
          <dt>Wiki</dt><dd>Your lead writes it from the repo</dd>
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
  if (!banner.hidden && !S.snap.paused) banner.innerHTML = `<span>${icon("pause")}<b>${limits.map(l => `${esc(AGENTS[l.runtime]?.name || l.runtime)} reached its plan's usage limit`).join(", ")}.</b> That work waits and starts again by itself at ${esc(limits.map(l => l.back).join(" and "))}.</span>`;
  else if (!banner.hidden) banner.innerHTML = S.snap.spend?.paused_by_budget
    ? `<span>${icon("pause")}<b>${esc(S.company)} reached its ${money(S.snap.spend.budget)} daily budget.</b> Running work finishes, and nothing new starts until tomorrow.</span><button class="btn" data-settings-tab="spend">Raise budget</button><button class="btn" data-keep-going="1">Keep going today</button>`
    : `<span>${icon("pause")}<b>${esc(S.company)} is paused.</b> Running work finishes, nothing new starts, and nobody replies in chat.</span><button class="btn" data-pause="0">Resume</button>`;
  // Setting up is full screen: no sidebar or top bar for things that don't exist yet.
  document.querySelector(".app").classList.toggle("focus", !!(S.welcome || S.profile || S.setup));
  v.classList.toggle("flush", !S.profile && !S.setup && S.view === "chat");
  const where = S.welcome ? "welcome" : S.profile ? "profile" : S.setup ? "setup-" + S.setup.step : `${S.company}/${S.view}`;
  if (where !== S.shown) {
    S.shown = where;
    v.classList.remove("enter"); void v.offsetWidth; v.classList.add("enter");
    clearTimeout(render.t); render.t = setTimeout(() => v.classList.remove("enter"), 400);
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
    v.innerHTML = { inbox: viewInbox, wiki: viewWiki, board: viewBoard, chat: viewChat }[S.view]?.() ?? viewCompany();
    const m = $("#msgs"); if (m) m.scrollTop = m.scrollHeight;
  }
  renderDrawer();
}
function typing() {
  const a = document.activeElement;
  return a && a.matches("input, textarea") && a.closest("#view, #drawer") && a.value.trim() !== "";
}
async function refresh(force) {
  if (!S.voices) S.voices = await invoke("voices").catch(() => null);
  if (!S.models) S.models = await invoke("models").catch(() => null);
  // Ignore a workspace tick already queued when onboarding stopped the timer.
  if (!force && (S.profile || S.setup)) return;
  try {
    if (S.founder === undefined) S.founder = await invoke("founder");
    if (!S.runtimes) S.runtimes = (await invoke("setup_info").catch(() => null))?.runtimes || null;
    // A first launch says hello before anything else: who you are, then your first company.
    if (S.founder === null && !S.profile && !S.setup) S.profile = { name: "" };
    const c = await invoke("companies");
    S.companies = c.list; S.looks = c.looks || {};
    // The very first launch opens on a welcome. Anyone with a company goes straight in.
    if (S.welcome === undefined) S.welcome = S.founder === null && !c.list.length;
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
    // The wiki is a small file and can change outside the app, so it's read with every refresh.
    const book = S.view === "wiki" ? await invoke("wiki", { company: S.company }) : S.wiki;
    if (S.view === "chat") S.chat = await invoke("chat", { company: S.company, channel: S.channel }).catch(() => S.chat);
    const json = JSON.stringify(snap) + (book ?? "") + (S.view === "chat" ? JSON.stringify(S.chat) : "");
    if (!force && (json === S.last || typing() || S.profile || S.setup)) return;
    S.snap = snap; S.last = json; S.wiki = book;
    alertNew(snap);
    // Channels count as read the first time they're seen, so opening the app doesn't light up every dot.
    for (const ch of snap.channels || []) if (!(ch.name in S.seen)) S.seen[ch.name] = ch.last || 0;
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

/* Actions */
document.addEventListener("mousedown", e => {
  const o = e.target.closest?.("[data-mpick]");
  if (o) { e.preventDefault(); pickMention(Number(o.dataset.mpick)); }
});
document.addEventListener("click", async e => {
  if (S.pop && !e.target.closest("#pop, [data-profile], [data-pop]")) { S.pop = false; renderPop(); }
  if (e.target.id === "sheet") { S.sheet = null; return renderSheet(); }
  if (S.switching && !e.target.closest(".switch")) { S.switching = false; renderSide(); }
  if (S.members && !e.target.closest(".members-pop, [data-members]")) { S.members = null; render(); }
  if (e.target.id === "scrim") { S.drawer = null; return renderDrawer(); }
  const el = e.target.closest("[data-plan-del],[data-talk],[data-commits],[data-welcome],[data-step-to],[data-repo-rm],[data-repo-add],[data-face-set],[data-commands-reset],[data-vpick],[data-msg-del],[data-theme-pick],[data-budget],[data-budget-custom],[data-look-color],[data-look-logo],[data-you-photo],[data-you-nophoto],[data-dismiss],[data-dismiss-q],[data-members],[data-replan],[data-new-ticket],[data-nt],[data-start],[data-more],[data-switch],[data-settings-tab],[data-keep-going],[data-dm],[data-face],[data-avatar-photo],[data-avatar-clear],[data-skip-setup],[data-su-agent],[data-hb-edit],[data-learn],[data-agent],[data-model],[data-copy],[data-term],[data-recheck],[data-hide-start],[data-brief],[data-pause],[data-pick-fill],[data-edit-person],[data-fire],[data-make-lead],[data-dept-cancel],[data-remove],[data-goto],[data-rail],[data-plan-new],[data-project],[data-pop],[data-settings],[data-sheet],[data-sheet-close],[data-toc],[data-close-open],[data-ship],[data-reveal],[data-health],[data-confirm],[data-forget],[data-channel],[data-mention],[data-profile],[data-pf],[data-open-url],[data-ticket],[data-stop],[data-new-co],[data-su],[data-tpl],[data-retry],[data-hb-del],[data-hb-raw],[data-diff],[data-hide-diff],[data-co],[data-view],[data-person],[data-hire],[data-add-dept],[data-close],[data-answer],[data-approve],[data-pick]");
  if (!el) return;
  const d = el.dataset;
  if (d.planDel) { await call("delete_ticket", { company: S.company, key: d.planDel }, `Removed ${d.planDel} from the plan`).catch(() => {}); return; }
  if (d.dismissQ) { el.disabled = true; await call("dismiss_question", { company: S.company, id: Number(d.dismissQ) }, "Dismissed").catch(() => (el.disabled = false)); return; }
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
  if (d.members) { S.members = S.members === d.members ? null : d.members; return render(); }
  if (d.mention) {
    if (S.members) { S.members = null; render(); }
    const input = $("#say-text"), name = first(byId(d.mention)?.name || d.mention);
    input.value = (input.value.trim() ? input.value.trimEnd() + " " : "") + "@" + name + " ";
    return input.focus();
  }
  if (d.skipSetup) { if (await saveProfile(S.profile.photo, S.profile.noPhoto)) { S.profile = null; toast(`Welcome back, ${myName()}`); refresh(true); } return; }
  if (d.hideStart) { try { localStorage.setItem("onecorp.started." + S.company, "hide"); } catch {} return render(); }
  if (d.brief) { S.view = "wiki"; S.open.brief = true; render(); await refresh(true); return $("#brief-text")?.focus(); }
  if (d.pause) {
    const on = d.pause === "1";
    await call("set_paused", { company: S.company, paused: on }, on ? `${S.company} is paused. Running work finishes, nothing new starts` : `${S.company} is back at work`).catch(() => {});
    return;
  }
  if (d.suAgent) { readSetup(); const [i, k] = d.suAgent.split(":"); const p = S.setup.people[Number(i)]; if (p.runtime !== k) p.model = ""; p.runtime = k; return render(); }
  if (d.agent) { Object.assign(S.drawer.form ||= {}, readPerson(), { runtime: d.agent, model: null }); return renderDrawer(); }
  if (d.welcome) { S.welcome = false; render(); return $("#pf-name")?.focus(); }
  if (d.stepTo) {
    if (d.stepTo === "You") return backToYou();
    readSetup(); S.setup.step = ["Connect", "Company", "Repo", "Team"].indexOf(d.stepTo); return render();
  }
  if ("vpick" in d) {
    $("#pf-v").value = d.vpick;
    el.parentElement.querySelectorAll(".vchip").forEach(b => b.setAttribute("aria-pressed", String(b === el)));
    $("#pf-vs").textContent = (S.voices || []).find(v => v.key === d.vpick)?.sample || "Picked to fit their role. Change it any time.";
    return;
  }
  if ("model" in d && el.classList.contains("mchip") && !el.classList.contains("vchip")) {
    $("#pf-m").value = d.model;
    el.parentElement.querySelectorAll(".mchip").forEach(b => b.setAttribute("aria-pressed", String(b === el)));
    return;
  }
  if (d.copy) { navigator.clipboard?.writeText(d.copy).then(() => toast("Copied. Paste it in Terminal")).catch(() => toast(d.copy)); return; }
  if (d.term) { await invoke("in_terminal", { command: d.term }).catch(e => toast(String(e))); recheckTools(); return; }
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
  if (d.dm) { S.view = "chat"; S.channel = "dm:" + d.dm; S.chat = null; S.drawer = null; render(); await refresh(true); return $("#say-text")?.focus(); }
  if (d.face && S.drawer?.form) {
    S.drawer.form.face = d.face;
    el.parentElement.querySelectorAll(".fpick").forEach(b => b.setAttribute("aria-pressed", String(b === el)));
    if (!S.drawer.form.photoUrl) $("#drawer .dhead .with-agent").firstElementChild.outerHTML = drawFace(d.face, "neutral");
    return;
  }
  if (d.avatarPhoto) {
    const path = await window.__TAURI__?.dialog?.open({ multiple: false, title: "Pick a photo", filters: [{ name: "Photos", extensions: ["png", "jpg", "jpeg", "webp", "gif"] }] }).catch(() => null);
    if (!path) return;
    try { Object.assign(S.drawer.form, readPerson(), { photoUrl: await invoke("image_preview", { path: String(path) }), photoPath: String(path), removePhoto: false }); } catch (err) { return toast(String(err), true); }
    return renderDrawer();
  }
  if (d.avatarClear) { Object.assign(S.drawer.form, readPerson(), { photoUrl: null, photoPath: null, removePhoto: true }); return renderDrawer(); }
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
  if (d.health) { S.health = null; renderSheet(); S.ready = await invoke("readiness").catch(() => S.ready); S.health = await invoke("health").catch(() => []); return renderSheet(); }
  if ("confirm" in d) { S.confirm = d.confirm || null; if (S.drawer?.type === "ticket") renderDrawer(); else if (S.view === "inbox") render(); renderSheet(); return $("#del-name")?.focus(); }
  if (d.remove) {
    el.disabled = true;
    try { await invoke("remove_ticket", { company: S.company, key: d.remove }); }
    catch (err) { el.disabled = false; return toast(String(err), true); }
    toast(`Deleted ${d.remove}`); S.confirm = null; S.drawer = null; renderDrawer(); return refresh(true);
  }
  if (d.forget) {
    el.disabled = true;
    try { await invoke("forget_founder"); } catch (err) { el.disabled = false; return toast(String(err), true); }
    S.founder = null; S.confirm = null; S.pop = false; S.sheet = null; S.profile = { name: "" }; renderPop(); renderSheet(); return render();
  }
  if (d.more) { S.open[d.more] = true; render(); return document.getElementById(d.more)?.focus(); }
  if (d.profile) return openSettings("you");
  if (d.pop) { S.pop = d.pop === "menu" && S.pop === "menu" ? false : d.pop; renderPop(); return $("#pf-name")?.focus(); }
  if (d.settings) return openSettings();
  if (d.sheet) { S.sheet = d.sheet; S.confirm = null; S.customBudget = false; renderSheet(); return loadSection(d.sheet); }
  if (d.budget !== undefined) { S.customBudget = false; await invoke("set_budget", { company: S.company, dollars: Number(d.budget) }).then(saved).catch(err => toast(String(err), true)); await loadSpend(); return refresh(true); }
  if (d.budgetCustom) { S.customBudget = true; renderSheet(); return $("#budget-in")?.focus(); }
  if (d.lookColor) {
    await invoke("set_look", { company: S.company, color: d.lookColor, logo: null, removeLogo: true }).then(saved).catch(err => toast(String(err), true));
    await refresh(true); return renderSheet();
  }
  if (d.lookLogo) {
    const path = await window.__TAURI__?.dialog?.open({ multiple: false, title: "Pick a logo", filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg", "webp", "svg", "gif"] }] }).catch(() => null);
    if (!path) return;
    await invoke("set_look", { company: S.company, color: S.looks?.[S.company]?.color || PALETTE[0], logo: String(path), removeLogo: false }).then(saved).catch(err => toast(String(err), true));
    await refresh(true); return renderSheet();
  }
  if (d.youPhoto) {
    const path = await window.__TAURI__?.dialog?.open({ multiple: false, title: "Pick a photo of you", filters: [{ name: "Photos", extensions: ["png", "jpg", "jpeg", "webp", "gif"] }] }).catch(() => null);
    if (path && await saveProfile(String(path))) { saved(); render(); renderSheet(); }
    return;
  }
  if (d.msgDel) {
    const id = Number(d.msgDel);
    // First click asks, second deletes. Nothing else to confirm in chat, so no dialog.
    if (S.msgDel !== id) { S.msgDel = id; return render(); }
    S.msgDel = null;
    await invoke("delete_message", { company: S.company, id }).catch(err => toast(String(err), true));
    return refresh(true);
  }
  if (d.talk) {
    try { await invoke("set_talk", { company: S.company, mode: d.talk }); S.settings.talk = d.talk; saved(); } catch (err) { toast(String(err), true); }
    return renderSheet();
  }
  if (d.commits) {
    try { await invoke("set_commits", { company: S.company, mode: d.commits }); S.settings.commits = d.commits; saved(); } catch (err) { toast(String(err), true); }
    return renderSheet();
  }
  if (d.commandsReset) {
    S.settings.repos = await invoke("set_commands", { company: S.company, repo: d.commandsReset, text: null }).catch(err => (toast(String(err), true), S.settings.repos));
    saved(); return renderSheet();
  }
  if (d.repoRm) {
    try { S.settings.repos = await invoke("remove_repo", { company: S.company, name: d.repoRm }); toast(`Removed ${d.repoRm}. Its files and finished tickets stay`); } catch (err) { toast(String(err), true); }
    renderSheet(); return refresh(true);
  }
  if (d.repoAdd) {
    const path = d.repoAdd === "pick" ? await window.__TAURI__?.dialog?.open({ directory: true, multiple: false, title: "Pick another repo" }).catch(() => null) : $("#repo-add-path")?.value.trim();
    if (!path) return $("#repo-add-path")?.focus();
    try { S.settings.repos = await invoke("add_repo", { company: S.company, path: String(path), name: null }); toast(`Added ${S.settings.repos.at(-1).name}. Tickets can work in it now`); } catch (err) { toast(String(err), true); }
    renderSheet(); return refresh(true);
  }

  if (d.faceSet) {
    try { localStorage.setItem("onecorp.faces", d.faceSet); } catch {}
    render(); return renderSheet();
  }
  if (d.themePick) { setTheme(d.themePick); return renderSheet(); }
  if (d.youNophoto) { if (await saveProfile(null, true)) { saved(); render(); renderSheet(); } return; }
  if (d.settingsTab) return openSettings(d.settingsTab);
  if (d.keepGoing) { await call("keep_going_today", { company: S.company }, "Going on for today. The budget applies again tomorrow").catch(() => {}); return; }
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
    return window.__TAURI__ ? invoke("open_link", { url: d.openUrl }).catch(err => toast(String(err), true)) : window.open(d.openUrl, "_blank");
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
    if (d.su === "back") { if (!s.step) return backToYou(); s.step--; return render(); }
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
    if (d.su === "mode-github") {
      s.mode = "github";
      if (s.gh?.error) s.gh = null;
      loadGithub(); render();
      return $("#gh-q")?.focus();
    }
    // Picking a repo asks where it goes first. Clone does the rest.
    if (d.su.startsWith("clone:")) {
      const name = d.su.slice(6);
      s.pick = { name, path: await invoke("clone_target", { name }).catch(() => `${s.repoInfo?.parent || ""}/${name.split("/").pop()}`) };
      render();
      return $("[data-su='clone-go']")?.focus();
    }
    if (d.su === "clone-cancel") { s.pick = null; return render(); }
    if (d.su === "clone-dir") {
      const dir = await window.__TAURI__?.dialog?.open({ directory: true, multiple: false, title: `Where should ${s.pick.name} go?`, defaultPath: s.repoInfo?.parent }).catch(() => null);
      if (dir) { s.pick.path = `${String(dir).replace(/\/$/, "")}/${s.pick.name.split("/").pop()}`; render(); }
      return;
    }
    if (d.su === "clone-go") {
      const path = $("#clone-path")?.value.trim() || s.pick.path;
      s.cloning = s.pick.name; s.pick = null; render();
      try {
        const name = s.cloning;
        const done = await invoke("clone_repo", { name, path }); s.cloning = null;
        const path2 = done;
        // The first pick is the main repo. Picks after that join it as more repos.
        // Back to the list either way, so you see it land there.
        s.mode = "existing"; s.justAdded = path2;
        if (s.repo?.ok) await addExtraRepo(path2);
        else await checkRepo(path2);
        setTimeout(() => { if (S.setup === s) s.justAdded = null; }, 2500);
      }
      catch (e) { s.cloning = null; render(); toast(String(e), true); }
      return;
    }
    if (d.su === "create-repo") return makeRepo();
    if (d.su === "toggle-gh") { s.owner = s.owner ? "" : (s.repoInfo.owners[0] || ""); return render(); }
    if (d.su === "add-person") { s.people.push({ name: "", role: "Engineer", dept: "Engineering", runtime: s.runtime, tier: "balanced" }); render(); return document.getElementById(`su-p${s.people.length - 1}-name`)?.focus(); }
    if (d.su.startsWith("del-")) { s.people.splice(Number(d.su.slice(4)), 1); return render(); }
    if (d.su === "parent") {
      const path = await window.__TAURI__.dialog.open({ directory: true, multiple: false, title: "Where should the new repo go?" }).catch(() => null);
      if (path) { s.parent = String(path); render(); }
      return;
    }
    if (d.su === "projects-dir") {
      const dir = await window.__TAURI__?.dialog?.open({ directory: true, multiple: false, title: "Where do you keep your projects?", defaultPath: s.repoInfo?.parent }).catch(() => null);
      if (!dir) return;
      try { const saved = await invoke("set_projects_folder", { path: String(dir) }); s.repoInfo.parent = s.parent = saved; toast("Clones go there from now on"); } catch (err) { toast(String(err), true); }
      return render();
    }
    if (d.su === "add-path") { const v = $("#su-add")?.value; await addRepo(v); return; }
    if (d.su === "rdel-main") {
      s.mainName = undefined;
      // The next repo steps up as the main one.
      const next = s.extra.shift();
      if (next) return checkRepo(next.path);
      s.repo = null; s.path = ""; s.checked = null; return render();
    }
    if (d.su.startsWith("rdel-")) { s.extra.splice(Number(d.su.slice(5)), 1); return render(); }
    if (d.su === "pick") {
      const path = await window.__TAURI__.dialog.open({ directory: true, multiple: false, title: "Pick the company's repo" }).catch(() => null);
      if (path) addRepo(String(path));
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
  else if (d.view) { S.view = d.view; S.hbEdit = false; if (d.close) S.drawer = null; render(); if (d.view === "wiki") refresh(true); }
  else if (d.person) openPerson(d.person);
  else if (d.hire) { openHire(Number(d.hire)); }
  else if (d.addDept) { S.addingDept = true; S.view = "company"; render(); $("#dept-name")?.focus(); }
  else if (d.close) { S.drawer = null; renderDrawer(); }
  else if (d.answer) { el.disabled = true; await call("answer", { company: S.company, id: Number(d.answer), text: d.val }, d.val === "no" ? "Left out of the wiki" : "Answered").catch(() => (el.disabled = false)); }
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
    // Back from setup: pick up where you were.
    if (S.profile.inSetup) { S.profile = null; return render(); }
    S.profile = null;
    // Coming back to your companies, not starting a new one.
    if (S.companies.length) { toast(`Welcome back, ${myName()}`); return refresh(true); }
    S.onboarding = true;
    return startSetup();
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
      // A path still in the box gets added first, so typing it and pressing Continue just works.
      const typed = $("#su-add")?.value.trim();
      if (typed) await addRepo(typed);
      if (!s.repo?.ok) { if (!typed) toast("Add the repo your team works in", true); return $("#su-add")?.focus(); }
      s.step = 3; render();
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
      `Connecting ${s.repo.path.split("/").pop()}`, real && lead ? `${first(lead.name)} starts writing the wiki` : "Reading the repo",
      ...(s.brief && people.length ? ["Planning the first version"] : [])] };
    render();
    const tick = setInterval(() => { if (s.launch.done < s.launch.steps.length - 1) { s.launch.done++; render(); } }, 420);
    const pace = new Promise(r => setTimeout(r, 420 * s.launch.steps.length));
    try {
      const [name] = await Promise.all([invoke("create_company", { name: s.name, repo: s.repo.path, repoName: s.mainName || null, repos: s.extra, mission: s.mission, people, brief: s.brief || null, color: s.color, logo: s.logo }), pace]);
      clearInterval(tick); s.launch.done = s.launch.steps.length; render();
      await new Promise(r => setTimeout(r, 450));
      S.setup = null; S.onboarding = false; S.company = name; S.view = "company"; S.snap = null; S.last = "";
      await refresh(true);
    } catch (e) { clearInterval(tick); s.launch = null; render(); toast(String(e), true); }
    return;
  }
  if (f.id === "budget-form") {
    const v = Number($("#budget-in").value || 0);
    S.customBudget = false;
    await call("set_budget", { company: S.company, dollars: v }, "Saved").catch(() => {});
    return loadSpend();
  }
  if (f.id === "you-form") { $("#pf-name").blur(); return; }
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
    await call("save_wiki", { company: S.company, text: $("#hb-text").value }, "Saved. Everyone reads it from their next ticket").then(() => { S.hbEdit = false; render(); }).catch(() => {});
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
        const f = S.drawer.form;
        await invoke("set_avatar", { company: S.company, id: S.drawer.id, face: f.face === p.name ? null : f.face, photo: f.photoPath, removePhoto: !!f.removePhoto });
        await invoke("set_cap", { company: S.company, id: S.drawer.id, dollars: Number($("#pf-cap")?.value || 0) });
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
  // Filters the repo list in place: redrawing would take the cursor out of the search box.
  if (e.target.id === "gh-q") {
    const q = e.target.value.trim().toLowerCase();
    document.querySelectorAll(".gh-list li").forEach(li => (li.hidden = q && !li.dataset.name.includes(q)));
    return;
  }
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
  if (e.target.id === "projects-path" && S.setup) {
    try { const saved = await invoke("set_projects_folder", { path: e.target.value }); S.setup.repoInfo.parent = S.setup.parent = saved; toast("Clones go there from now on"); } catch (err) { toast(String(err), true); }
    return render();
  }
  if (e.target.dataset?.commands) {
    try { S.settings.repos = await invoke("set_commands", { company: S.company, repo: e.target.dataset.commands, text: e.target.value }); saved(); } catch (err) { toast(String(err), true); }
    return renderSheet();
  }

  if (e.target.dataset?.price) {
    await invoke("set_price", { company: S.company, runtime: e.target.dataset.price, perMillion: Math.max(0, Number(e.target.value) || 0) }).then(saved).catch(err => toast(String(err), true));
    return loadSpend();
  }
  if (e.target.id === "pf-name" && e.target.closest("#you-form")) {
    if (e.target.value.trim() && e.target.value.trim() !== S.founder?.name && await saveProfile()) { saved(); render(); }
    return;
  }
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
  if (e.key === "Escape" && S.members) { S.members = null; return render(); }
  if (e.key === "Escape") { if (S.drawer) { S.drawer = null; renderDrawer(); } else if (S.addingDept) { S.addingDept = false; render(); } }
  if ((e.metaKey || e.ctrlKey) && e.key === "," && S.snap && !S.setup && !S.profile) { e.preventDefault(); return openSettings(); }
  if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k" && S.snap && !S.setup && !S.profile) { e.preventDefault(); S.drawer = { type: "new-ticket" }; renderDrawer(); $("#nt-title")?.focus(); }
});

// In a plain browser, #board, #inbox, #wiki, #ticket, #person, #new-ticket, #setup or #profile opens
// straight onto that screen, so each one can be looked at and screenshotted. Never used in the app.
async function previewScreen() {
  const screen = location.hash.slice(1);
  // ?light or ?dark, for screenshots in either theme.
  if (/[?&](light|dark)\b/.test(location.search)) document.documentElement.dataset.theme = location.search.includes("light") ? "light" : "dark";
  await refresh(true);
  if (screen === "settings") await openSettings();
  if (screen === "switcher") { S.switching = true; renderSide(); }
  if (screen === "plan") {
    S.view = "board"; S.drawer = { type: "plan" }; render();
    $("#pj-name").value = "Stripe billing";
    $("#pj-goal").value = "A Pro plan at $6 a month next to the free tier. Checkout with Stripe, webhooks that survive retries, and a billing page to see invoices or cancel.";
  }
  if (screen === "rail") toggleRail(true);
  if (screen === "start") { S.snap.tickets = []; S.snap.projects = []; S.snap.kickoff.brief = ""; render(); }
  if (screen === "wiki-edit") { S.view = "wiki"; S.hbEdit = true; await refresh(true); render(); }
  if (screen === "inbox-zero") { S.snap.inbox = []; S.snap.reviews = []; S.snap.failed = []; S.snap.with_leads = []; S.view = "inbox"; render(); }
  if (screen === "launching") { await startSetup(); S.setup.name = "snowztech"; await checkRepo("/Users/you/dev/snowztech"); S.setup.launch = { done: 2, steps: ["Founding snowztech", "Hiring 3 people", "Connecting snowztech", "Odile starts writing the wiki"] }; render(); }
  if (screen === "connect") await startSetup();
  if (screen === "team") { await startSetup(); setTemplate("startup"); S.setup.step = 3; render(); }
  if (screen === "dm") { S.view = "chat"; S.channel = "dm:tomas"; await refresh(true); }
  if (screen === "edit-avatar") { await openPerson("tomas"); S.drawer.edit = true; renderDrawer(); }
  if (screen === "spend") await openSettings("spend");
  if (screen === "paused") { S.snap.paused = true; render(); }
  if (screen === "hire") openHire(1);
  if (screen === "person-edit") { await openPerson("tomas"); S.drawer.edit = true; renderDrawer(); }
  if (screen === "idle") { S.snap.employees.forEach(e => (e.status = "idle")); S.snap.inbox = []; S.snap.reviews = []; S.snap.failed = []; render(); }
  if (screen === "project") { S.view = "board"; S.project = "SNO-20"; render(); }
  if (["board", "inbox", "wiki", "chat"].includes(screen)) { S.view = screen; render(); }
  if (screen === "ticket") openTicket("SNO-11");
  if (screen === "person") openPerson("tomas");
  if (screen === "edit-person") { await openPerson("tomas"); S.drawer.edit = true; S.person.runtime = "codex"; renderDrawer(); }
  if (screen === "new-ticket") { S.view = "board"; S.drawer = { type: "new-ticket" }; render(); }
  if (screen === "setup") await startSetup();
  if (screen === "repos") { await startSetup(); S.setup.name = "snowztech"; await checkRepo("/Users/you/dev/notes"); S.setup.extra = [{ path: "/Users/you/dev/notes-api", name: "notes-api", branch: "feat/search" }]; S.setup.step = 2; render(); }
  if (screen === "github-pick") { await startSetup(); S.setup.name = "snowztech"; S.setup.step = 2; S.setup.mode = "github"; S.setup.gh = { repos: await invoke("github_repos") }; S.setup.pick = { name: "snowztech/notes-api", path: "/Users/you/dev/notes-api" }; render(); }
  if (screen === "github") { await startSetup(); S.setup.name = "snowztech"; S.setup.step = 2; S.setup.mode = "github"; S.setup.gh = { repos: await invoke("github_repos") }; render(); }
  if (screen.startsWith("setup-")) { await startSetup(); S.setup.name = "snowztech"; S.setup.mission = "Open source tools for building AI agents"; await checkRepo("/Users/you/dev/snowztech"); S.setup.step = Number(screen.slice(6)); render(); }
  if (screen === "profile") { S.founder = null; S.companies = []; S.profile = { name: "" }; render(); }
  if (screen === "welcome") { S.founder = null; S.companies = []; S.welcome = true; S.profile = { name: "" }; render(); }
  if (screen === "welcome-back") { S.founder = null; S.profile = { name: "" }; render(); }
  if (["popover", "you", "agents", "appearance", "about"].includes(screen)) await openSettings(screen === "popover" ? "you" : screen);
  if (screen === "menu") { S.pop = "menu"; renderPop(); }
  if (screen.startsWith("faces-")) { try { localStorage.setItem("onecorp.faces", screen.slice(6)); } catch {} render(); }
  if (screen === "limit") { S.snap.limits = [{ runtime: "claude", back: "15:00" }]; render(); }
  if (screen === "health") await openSettings("health");
}
try { if (localStorage.getItem("onecorp.rail")) toggleRail(true); } catch {}
if (window.__TAURI__) refresh(true); else previewScreen();

/* Sample data for opening ui/index.html in a browser. Never used inside the app. */
let previewFounder = null;
function preview(cmd, args) {
  const team = [
    ["odile", "Odile Martin", "CTO", "claude", "opus", "working", "Reading the SNO-12 diff", 1], ["maya", "Maya Okafor", "Senior engineer", "codex", null, "blocked", "Need you on SNO-12", 1],
    ["tomas", "Tomás Reis", "Engineer", "codex", null, "working", "Bash cargo test billing", 1], ["rui", "Rui Alves", "Product lead", "claude", "sonnet", "working", "Created SNO-18", 2],
    ["lina", "Lina Park", "Product designer", "claude", "sonnet", "queued", "Up next: SNO-14", 2], ["sam", "Sam Duarte", "Writer", "claude", "haiku", "idle", "", 3],
  ].map(([id, name, role, runtime, model, status, bubble, department]) => ({ id, name, role, runtime, model, status, bubble, department, spend: "$0.40", ticket: null }));
  const VOICES = [["terse","Terse","Done. Login works, tests pass."],["warm","Warm","Login's in! Thanks for the clear ticket. Tests are green."],["thorough","Thorough","Login is in. I went with magic links over passwords: less to store. Tests cover both paths."],["dry","Dry","Login works. The tests agree, which is a first this week."],["blunt","Blunt","Login works. The old session code was broken, so I replaced it."],["curious","Curious","Login works. Should links expire after 15 minutes or an hour?"],["upbeat","Upbeat","Login shipped! On to signup next."],["calm","Calm","Login is done. 1. Link sent. 2. Token checked. 3. Session set."]].map(([key, label, sample]) => ({ key, label, sample }));
  const data = {
    voices: VOICES,
    models: { claude: [["claude-opus-5-5", "Opus 5.5"], ["claude-sonnet-5-5", "Sonnet 5.5"], ["claude-haiku-5-5", "Haiku 5.5"], ["claude-fable-5-1", "Fable 5.1"]].map(([id, name]) => ({ id, name, about: "" })), codex: [["gpt-6.1-sol", "GPT-6.1-Sol"], ["gpt-6-astra", "GPT-6-Astra"], ["gpt-6-luna", "GPT-6-Luna"]].map(([id, name]) => ({ id, name, about: "" })),
      tiers: { claude: { strongest: { id: "claude-opus-5-5", name: "Opus 5.5" }, balanced: { id: "claude-sonnet-5-5", name: "Sonnet 5.5" }, cheapest: { id: "claude-haiku-5-5", name: "Haiku 5.5" } },
        codex: { strongest: { id: "gpt-6-astra", name: "GPT-6-Astra" }, balanced: { id: "gpt-6.1-sol", name: "GPT-6.1-Sol" }, cheapest: { id: "gpt-6-luna", name: "GPT-6-Luna" } } },
      defaults: { claude: "Opus 5.5", codex: "GPT-6-Astra" } },
    companies: { list: ["snowztech", "tvcask", "autoremind", "pingbuddy"], current: "snowztech", looks: { snowztech: { color: "#4772ae" }, tvcask: { color: "#ae5c47" }, autoremind: { color: "#47ae88" }, pingbuddy: { color: "#27272a" } } },
    snapshot: {
      repos: [{ name: "notes", path: "/Users/you/dev/notes", main: true }, { name: "notes-web", path: "/Users/you/dev/notes-web", main: false }],
      company: "snowztech", serve: true, kickoff: { brief: "A notes app for writers.", planner: "rui", planned: true, planning: [] }, ship: { mode: "github", forge: "GitHub", pr_word: "pull request" }, employees: team, tickets: [
        { key: "SNO-9", plan: "SNO-20", title: "Rate limit the public API", assignee: "tomas", status: "failed", spend: "40k tok" },
        { key: "SNO-11", plan: "SNO-20", title: "Pricing page", assignee: "odile", status: "review", spend: "$0.31" },
        { key: "SNO-12", plan: "SNO-20", title: "Magic link login", assignee: "maya", status: "blocked", spend: "52k tok" },
        { key: "SNO-14", repo: "notes-web", plan: "SNO-20", title: "Empty state for the notes list", assignee: "lina", status: "queued", spend: "$0.00" },
        { key: "SNO-15", plan: "SNO-20", title: "Retry failed Stripe webhooks", assignee: "tomas", status: "doing", spend: "75k tok" },
        { key: "SNO-8", plan: "SNO-20", title: "CI pipeline", assignee: "tomas", status: "done", spend: "$0.22" },
        { key: "SNO-16", title: "Export notes as Markdown", assignee: null, status: "backlog", spend: "$0.00", created_by: "Rui Alves" },
        { key: "SNO-17", title: "Notes API pagination", assignee: "maya", status: "waiting", spend: "$0.00", after: "SNO-12" }],
      projects: [{ key: "SNO-20", title: "First version", goal: "", status: "done", planner: "rui", planner_name: "Rui Alves", total: 8, done: 5 }, { key: "SNO-30", title: "Stripe billing", goal: "Pro plan at $6 a month with a free tier.", status: "review", planner: "rui", planner_name: "Rui Alves", total: 4, done: 0 }],
      departments: [{ id: 1, name: "Engineering", lead: "odile", members: ["odile", "maya", "tomas"] }, { id: 2, name: "Product", lead: "rui", members: ["rui", "lina"] }, { id: 3, name: "Growth", lead: "sam", members: ["sam"] }],
      with_leads: [{ id: 5, kind: "question", key: "SNO-15", employee: "tomas", name: "Tomás Reis", lead_name: "Odile Martin", question: "Retry webhooks 3 or 5 times?", section: "", options: ["3", "5"], default: "3", blocking: false }],
      // A question with a long default and no options, like most real ones.
      inbox_extra: null,
      inbox: [{ id: 7, kind: "question", key: "SNO-20", employee: "rui", name: "Rui Alves", question: "Which K-drama and region did you see Viki missing on? The API already pulls TMDB's subscription tiers, so I suspect a data gap, not a filter on our side.", section: "", options: [], default: "I'll plan with an investigation ticket first, using popular K-dramas in US, KR and FR.", blocking: false }, { id: 2, kind: "question", key: "SNO-12", employee: "maya", name: "Maya Okafor", question: "Magic links: 15 or 60 minute expiry?", section: "", options: ["15 minutes", "60 minutes"], default: "15 minutes", blocking: true, via: "odile", via_name: "Odile Martin", note: "it changes what users see, so it's a product call" },
        { id: 3, kind: "wiki", key: "SNO-14", employee: "lina", name: "Lina Park", question: "Empty states are one sentence and one button", section: "Conventions", options: [], default: "", blocking: false }],
      failed: [{ key: "SNO-9", title: "Rate limit the public API", employee: "tomas", name: "Tomás Reis", why: "stopped before it finished" }],
      reviews: [{ key: "SNO-20", title: "First version", employee: "rui", name: "Rui Alves", kind: "plan", summary: "Setup first, then login and sync in parallel. Risk: magic links need an email provider.", spend: "$0.21",
          tickets: [{ key: "SNO-21", title: "Set up the project", description: "A skeleton that builds, with a README.\n\nDone looks like:\n- `cargo build` and `cargo test` pass\n- A README that says how to run it\n- CI runs the tests on every push", assignee: "maya", name: "Maya Okafor", after: null, status: "backlog" },
            { key: "SNO-22", title: "Magic link login", description: "Email a one-time link, 15 minute expiry.", assignee: "tomas", name: "Tomás Reis", after: "SNO-21", status: "backlog" }] },
        { key: "SNO-11", kind: "work", tickets: [], title: "Pricing page", employee: "odile", name: "Odile Martin", summary: "Added the pricing page with Free and Pro plans. Check the copy on mobile.", spend: "$0.31", pr_number: 41, pr_url: "https://github.com/snowztech/notes/pull/41" }],
      feed: [{ at: "2026-11-09 11:02:00", key: "SNO-11", employee: "odile", name: "Odile Martin", kind: "done", summary: "finished, $0.31", title: "Pricing page" }, { at: "2026-11-09 10:52:00", key: "SNO-15", employee: "tomas", name: "Tomás Reis", kind: "start", summary: "started", title: "Retry failed Stripe webhooks" }, { at: "2026-11-09 10:31:00", key: "SNO-12", employee: "maya", name: "Maya Okafor", kind: "block", summary: "asked you about magic link expiry", title: "Magic link login" }],
      spend: { cost: 3.24, tokens: 180000, label: "$3.24", today: 3.24, today_label: "$3.24", budget: 5, warn: false, paused_by_budget: false },
      dms: [["odile", 12], ["maya", null], ["tomas", 14], ["rui", null], ["lina", null], ["sam", null]].map(([id, last]) => ({ id, channel: "dm:" + id, last })),
      channels: [{ name: "general", title: "General", department: null, last: 9 }, { name: "engineering", title: "Engineering", department: 1, last: 8 }, { name: "product", title: "Product", department: 2, last: 4 }, { name: "growth", title: "Growth", department: 3, last: null }],
    },
    wiki: "# snowztech wiki\n\n## Mission\n\nOpen source tools for building AI agents.\n\n## Codebase\n\n| Repo | What it is | Stack |\n| --- | --- | --- |\n| `notes` (primary) | The app | Rust, Axum, SQLite |\n| `notes-web` | Marketing site | Next.js, Tailwind |\n\n## Decisions\n\n- Standard library first. No new dependencies without asking the founder.\n- Auth tokens are hashed at rest with sha2.\n\n## Conventions\n\n- All program output is lowercase, with no trailing punctuation.\n- CLI flags are long-form, kebab-case.\n",
    github_repos: [["snowztech/vikusha", "A Go framework for building AI agents", false], ["snowztech/onecorp", "Run a company of AI employees", false], ["lucas/notes", "Notes app for writers", true], ["snowztech/website", "", false]].map(([name, description, priv]) => ({ name, description, private: priv })),
    setup_info: { runtimes: ["claude", "fake"], voices: VOICES,
      teams_from: [{ company: "tvcask", hires: [{ name: "Inès Costa", role: "CTO", dept: "Engineering", runtime: "claude", voice: "calm" }, { name: "Kenji Sato", role: "Engineer", dept: "Engineering", runtime: "codex", voice: "terse" }] }],
      templates: [
      { key: "solo", name: "Solo builder", description: "One engineer who ships. Hire more when you need them.", runtime: null, hires: [{ name: "Maya Okafor", role: "Engineer", dept: "Engineering" }] },
      { key: "startup", name: "Startup", description: "A CTO, frontend and backend engineers, a PM, a designer, and a writer for the launch.", runtime: null, hires: [{ name: "Odile Martin", voice: "calm", role: "CTO", dept: "Engineering", tier: "strongest" }, { name: "Maya Okafor", voice: "upbeat", role: "Frontend engineer", dept: "Engineering" }, { name: "Tomás Reis", voice: "terse", role: "Backend engineer", dept: "Engineering" }, { name: "Rui Alves", voice: "curious", role: "Product manager", dept: "Product" }, { name: "Lina Park", voice: "warm", role: "Product designer", dept: "Product" }, { name: "Sam Duarte", voice: "dry", role: "Growth writer", dept: "Growth", tier: "cheapest" }] },
      { key: "indie", name: "Indie maker", description: "Build it and launch it yourself: an engineer, a designer, and a growth marketer.", runtime: null, hires: [{ name: "Maya Okafor", role: "Engineer", dept: "Product" }, { name: "Lina Park", role: "Product designer", dept: "Product" }, { name: "Sam Duarte", role: "Growth marketer", dept: "Growth" }] },
      { key: "agency", name: "Agency", description: "Client work: a PM who plans, two engineers, a designer, and a QA reviewer who checks every change.", runtime: null, hires: [{ name: "Rui Alves", role: "Project manager", dept: "Delivery" }, { name: "Maya Okafor", role: "Engineer", dept: "Delivery" }, { name: "Tomás Reis", role: "Engineer", dept: "Delivery" }, { name: "Lina Park", role: "Product designer", dept: "Delivery" }, { name: "Odile Martin", role: "QA reviewer", dept: "Delivery" }] },
      { key: "oss", name: "Open source", description: "A maintainer who reviews every change, an engineer, and a writer for docs and changelogs.", runtime: null, hires: [{ name: "Odile Martin", role: "Maintainer", dept: "Engineering" }, { name: "Maya Okafor", role: "Engineer", dept: "Engineering" }, { name: "Sam Duarte", role: "Docs writer", dept: "Docs" }] },
      { key: "empty", name: "Start empty", description: "No one yet. Add departments and hire them yourself.", runtime: null, hires: [] }] },
    ticket_detail: { key: "SNO-11", title: "Pricing page", status: "review", description: "Free and Pro plans. Pro is $6 a month.", created_by: "Rui Alves", after: null, pr_number: 41, pr_url: "https://github.com/snowztech/notes/pull/41", employee: "odile", name: "Odile Martin", branch: "onecorp/snowztech/sno-11", base: "main", spend: "$0.31",
      log: [["10:40", "start", "claude (opus), started"], ["10:40", "tool", "Read src/routes/mod.rs"], ["10:41", "say", "Pricing lives next to the landing page, reusing the plan cards from settings."],
        ["10:44", "tool", "Write src/routes/pricing.rs"], ["10:47", "tool", "Edit templates/pricing.html"], ["10:49", "tool", "Bash cargo test pricing"], ["10:50", "error", "test pricing::pro_monthly ... FAILED"],
        ["10:52", "tool", "Edit src/routes/pricing.rs"], ["10:54", "tool", "Bash cargo test"], ["10:55", "say", "All 48 tests pass. Pro shows $6 a month, yearly saves two months."],
        ["10:56", "tool", "post_message Pricing page is up for review"], ["10:58", "commit", "c3dbf20 SNO-11: Pricing page"], ["11:02", "done", "finished, $0.31"]].map(([t, kind, summary]) => ({ at: `2026-11-09 ${t}:00`, kind, summary })) },
    new_repo_info: { parent: "/Users/you/dev", owners: ["you", "snowztech"] },
    readiness: [{ level: "ok", what: "git", detail: "git version 2.47.1", fix: null }, { level: "ok", what: "Claude Code", detail: "2.3.4", fix: null },
      { level: "warn", what: "Codex", detail: "not installed. Employees can't run on it", fix: "npm install -g @openai/codex" }, { level: "warn", what: "GitHub CLI", detail: "installed but not signed in", fix: "gh auth login" }],
    settings: { talk: "balanced", talk_models: { cheapest: ["Haiku 5.5"], balanced: ["Sonnet 5.5"] }, commits: "team", git_you: { name: "Lucas", email: "lucas@example.com" }, repos: [{ name: "notes", path: "/Users/you/dev/notes", main: true, commands: ["cargo build", "cargo test", "cargo check", "cargo clippy", "cargo fmt"], custom: false }, { name: "notes-web", path: "/Users/you/dev/notes-web", main: false, commands: ["pnpm install", "pnpm test", "pnpm run"], custom: false }], name: "snowztech", repo: "/Users/you/dev/snowztech", data: "/Users/you/.onecorp/companies/snowztech", home: "/Users/you/.onecorp", ship: "github", forge: "github", forge_name: "GitHub", origin: "git@github.com:snowztech/notes.git", tickets: 22, people: 6, version: "0.1.0" },
    health: [{ level: "ok", what: "git", detail: "git version 2.47.1", fix: null }, { level: "ok", what: "Claude Code", detail: "2.3.4 (Claude Code)", fix: null },
      { level: "warn", what: "Codex", detail: "not installed. Employees can't run on it", fix: "npm install -g @openai/codex, then run codex once to sign in" },
      { level: "ok", what: "company snowztech", detail: "opens pull requests on git@github.com:snowztech/notes.git (Logged in to github.com account you)", fix: null }],
    check_repo: { ok: true, path: "/Users/you/dev/snowztech", branch: "main", suggested: "snowztech", used_by: [], forge: "GitHub", pr_word: "pull request", signed_in: true, account: "Logged in to github.com account you" },
    founder: { name: "Lucas", photo: null },
    save_founder: null,
    create_company: "acme",
    spend_report: { budget: 5, estimated: true,
      totals: { today: { cost: 3.24, tokens: 180000, label: "$3.24" }, week: { cost: 14.8, tokens: 900000, label: "$14.80" }, all: { cost: 31.5, tokens: 2100000, label: "$31.50" } },
      days: [0.4, 1.2, 0, 2.1, 3.8, 1.1, 0, 0.6, 2.9, 4.6, 1.8, 2.2, 3.1, 3.24].map((cost, i) => ({ day: new Date(Date.UTC(2026, 9, 27 + i)).toISOString().slice(0, 10), cost })),
      people: [["odile", "Odile Martin", 1.31, 6.2, 12.4, 0], ["tomas", "Tomás Reis", 1.12, 4.1, 9.8, 3], ["maya", "Maya Okafor", 0.52, 3.0, 6.1, 0], ["rui", "Rui Alves", 0.29, 1.5, 3.2, 0]].map(([id, name, t, w, a, cap]) => ({ id, name, cap, today: { cost: t }, week: { cost: w }, all: { cost: a } })),
      projects: [{ key: "SNO-20", title: "First version", today: { cost: 2.1 }, week: { cost: 11.2 }, all: { cost: 24.9 } }],
      agents: [{ runtime: "claude", estimated: false, today: { cost: 2.12 }, week: { cost: 10.7 }, all: { cost: 21.7 } }, { runtime: "codex", estimated: true, today: { cost: 1.12 }, week: { cost: 4.1 }, all: { cost: 9.8 } }],
      prices: [{ runtime: "codex", price: 5 }, { runtime: "claude", price: 9 }] },
    chat: { channel: "general", messages: [
      { id: 1, at: "2026-11-09 09:12:00", author: "founder", name: "Lucas", founder: true, text: "Morning team. Goal this week: magic link login and the pricing page live.", key: null, pending: false },
      { id: 2, at: "2026-11-09 09:40:00", author: "rui", name: "Rui Alves", text: "Plan is in your inbox as SNO-20. Setup first, then login and sync in parallel.", key: "SNO-20", pending: false },
      { id: 3, at: "2026-11-09 10:05:00", author: "maya", name: "Maya Okafor", text: "Heads up: magic links need an **email provider**. I'll stub it behind the `Mailer` trait for now, see [the Resend docs](https://resend.com/docs).", key: "SNO-12", pending: false },
      { id: 4, at: "2026-11-09 10:06:00", author: "maya", name: "Maya Okafor", text: "@odile can you look at the token hashing when you review?", key: "SNO-12", pending: false },
      { id: 5, at: "2026-11-09 11:02:00", author: "odile", name: "Odile Martin", text: "Pricing page is up for review in SNO-11. Copy needs a look on mobile.", key: "SNO-11", pending: false },
      { id: 6, at: "2026-11-09 11:20:00", author: "founder", name: "Lucas", founder: true, text: "@tomas how far are the Stripe webhook retries?", key: null, pending: false },
      { id: 7, at: "2026-11-09 11:20:00", author: "tomas", name: "Tomás Reis", text: "", key: null, pending: true }] },
    person: { id: "tomas", name: "Tomás Reis", role: "Engineer", runtime: "codex", model: null, bio: "Fast, likes refactors a bit too much.", department: 1, voice: "dry", voices: VOICES, current: { key: "SNO-15", title: "Retry failed Stripe webhooks", status: "doing" },
      activity: [{ at: "2026-11-09 10:50:00", kind: "tool", summary: "Edit src/billing/webhook.rs" }, { at: "2026-11-09 10:52:00", kind: "tool", summary: "Bash cargo test billing" }], tickets: [{ key: "SNO-15", title: "Retry failed Stripe webhooks", status: "doing", spend: "75k tok" }, { key: "SNO-9", title: "Rate limit the public API", status: "failed", spend: "40k tok" }, { key: "SNO-8", title: "CI pipeline", status: "merged", spend: "$0.22" }, { key: "SNO-4", title: "Notes API: create and list", status: "merged", spend: "$0.48" }],
      memory: ["2026-11-09 Stripe retries for 3 days, make handlers idempotent instead of queueing", "2026-11-08 Lucas wants errors logged with the ticket key, never swallowed", "2026-11-07 Billing code lives in src/billing, one file per provider", "2026-11-06 Run cargo test billing before the full suite, it's 10x faster"] },
  };
  data.review = { key: "SNO-11", title: "Pricing page", base: "main", branch: "onecorp/snowztech/sno-11", added: 5, removed: 1,
    files: [{ path: "web/src/pages/pricing.tsx", added: 4, removed: 1 }, { path: "README.md", added: 1, removed: 0 }],
    commits: [{ hash: "c3dbf20", author: "Odile Martin", subject: "SNO-11: pricing page" }],
    patch: "diff --git a/web/src/pages/pricing.tsx b/web/src/pages/pricing.tsx\nindex 1..2 100644\n--- a/web/src/pages/pricing.tsx\n+++ b/web/src/pages/pricing.tsx\n@@ -1,3 +1,6 @@\n export function Pricing() {\n-  return null\n+  return <>\n+    <Plan name=\"Free\" price={0} />\n+    <Plan name=\"Pro\" price={6} />\n+  </>\n }\ndiff --git a/README.md b/README.md\nindex 3..4 100644\n--- a/README.md\n+++ b/README.md\n@@ -1 +1,2 @@\n # Notes\n+Pricing: free, or $6 a month.\n", truncated: false };
  if (cmd === "chat" && String(args?.channel).startsWith("dm:")) return Promise.resolve({ channel: args.channel, messages: args.channel === "dm:tomas"
    ? [{ id: 13, at: "2026-11-09 11:40:00", author: "founder", name: "Lucas", founder: true, text: "How are the webhook retries going?", key: null, pending: false },
       { id: 14, at: "2026-11-09 11:41:00", author: "tomas", name: "Tomás Reis", text: "Close. Retries back off over 3 days now, and handlers are idempotent. Tests next, then it's in your Inbox as SNO-15.", key: "SNO-15", pending: false }] : [] });
  // Saving your name sticks for the rest of the preview, like it does in the app.
  if (cmd === "clone_target") return Promise.resolve(`/Users/you/dev/${String(args.name).split("/").pop()}`);
  if (cmd === "save_founder") { previewFounder = { name: args.name, photo: null }; return Promise.resolve(null); }
  if (cmd === "founder" && previewFounder) return Promise.resolve(previewFounder);
  if (cmd === "check_repo" && args?.path) return Promise.resolve({ ...data.check_repo, path: args.path, suggested: args.path.split("/").pop() });
  if (cmd in data) return Promise.resolve(data[cmd]);
  return Promise.reject("This is a preview. Open the OneCorp app to make changes.");
}
