// OneCorp app UI. It polls the company snapshot and renders the Company page and the Inbox.
// Every action calls a Tauri command, which runs the same library code as the CLI.
// Opened in a plain browser (no Tauri), it runs on sample data so the design can be checked quickly.

const invoke = window.__TAURI__ ? window.__TAURI__.core.invoke : preview;
if (window.__TAURI__) document.documentElement.classList.add("native");
const $ = s => document.querySelector(s);
const esc = s => String(s ?? "").replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]));
const first = n => String(n || "").split(" ")[0];

const S = { companies: [], company: null, snap: null, last: "", view: "company", drawer: null, person: null, addingDept: false, open: {}, channel: "general", chat: null, seen: {}, diffs: {}, handbook: null, hbRaw: false, setup: null, founder: undefined, profile: null, pop: false };

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
  const h = hash(name), L = { skin: h % 5, hair: (h >> 3) % 6, style: (h >> 6) % 5, acc: (h >> 9) % 4 }, hue = h % 360;
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
  return f?.photo ? `<span class="av"><img src="${esc(f.photo)}" alt=""></span>` : face(f?.name || "You", "neutral");
}
const myName = () => S.founder?.name || "You";
const byId = id => (S.snap?.employees || []).find(e => e.id === id);
const waiting = () => (S.snap ? S.snap.inbox.length + S.snap.reviews.length + (S.snap.failed || []).length : 0);

/* Sidebar and top bar */
function renderSide() {
  const n = waiting(), pages = { company: "Company", board: "Board", inbox: "Inbox", chat: "Chat", handbook: "Handbook" };
  const unread = (S.snap?.channels || []).some(ch => ch.last && ch.last > (S.seen[ch.name] || 0) && !(S.view === "chat" && S.channel === ch.name));
  $("#side").innerHTML = `<div class="side-drag" data-tauri-drag-region></div>
    <div class="cos">${S.companies.map(c => `<button class="cobtn" data-co="${esc(c)}" aria-current="${!S.setup && c === S.company}"><span class="logo">${esc(c[0].toUpperCase())}</span><span class="coname">${esc(c)}</span></button>`).join("")}
      <button class="cobtn add" data-new-co="1" aria-current="${!!S.setup}"><span class="logo">+</span><span class="coname">New company</span></button></div>
    ${S.company ? `<h6>${esc(S.company)}</h6>${Object.entries(pages).map(([k, v]) =>
      `<button class="navbtn" data-view="${k}" ${S.view === k ? 'aria-current="page"' : ""}>${v}${k === "inbox" && n ? `<span class="badge">${n}</span>` : ""}${k === "chat" && unread ? `<i class="dot" title="New messages"></i>` : ""}</button>`).join("")}` : ""}
    ${S.snap ? `<div class="serve ${S.snap.serve ? "" : "off"}"><i></i>${S.snap.serve ? "Your team is working" : "Nobody is running work"}</div>` : ""}
    <button class="side-foot" data-profile="1" title="Edit your profile">${me()}<div><b>${esc(myName())}</b><span>Founder</span></div></button>`;
}
function renderTop() {
  const s = S.snap;
  $("#title").textContent = S.profile ? (S.founder ? "Your profile" : "Welcome") : S.setup ? "New company" : { inbox: "Inbox", handbook: "Handbook", board: "Board", chat: "Chat" }[S.view] || S.company || "OneCorp";
  const depts = s ? s.departments.filter(d => d.id !== null).length : 0;
  const ships = s?.ship?.forge ? ` · ships to ${s.ship.forge}` : s ? " · merges locally" : "";
  $("#sub").textContent = s ? `${s.employees.length} ${s.employees.length === 1 ? "person" : "people"} · ${depts} ${depts === 1 ? "department" : "departments"}${ships}` : "";
  const off = !!S.setup || !!S.profile || !s;
  $("#spend").textContent = !off && (s.spend.cost > 0 || s.spend.tokens > 0) ? `${s.spend.label} spent` : "";
  $("#new-ticket").hidden = off;
  if (off) $("#sub").textContent = "";
}

/* Company page: you on top, departments beneath as rooms of desks */
function desk(e, lead) {
  const show = e.bubble ? "show" : "";
  const works = (RUNTIME[e.runtime]?.[0] || e.runtime) + (e.model ? ` · ${e.model}` : "");
  return `<button class="desk ${e.status}" data-person="${esc(e.id)}" title="${esc(e.name)} works on ${esc(works)}">
    <span class="bubble ${show}">${esc(e.bubble)}</span>
    <span class="station"><span class="seat">${face(e.name, e.status)}</span><span class="monitor"><i></i><i></i><i></i><i></i></span></span>
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
    return `<li><section class="room"><header class="room-h"><div><h3>${esc(d.name)}</h3><p>${meta.join(" · ")}</p></div></header>
      <div class="room-floor">${ms.map(e => desk(e, d.lead === e.id)).join("")}${empty}</div></section></li>`;
  }).join("");
  const add = S.addingDept
    ? `<li><form class="room-new" id="dept-form"><label for="dept-name"><b>Name the department</b></label><div class="row"><input id="dept-name" placeholder="Engineering, Design, Growth…" autocomplete="off"><button class="btn primary">Add</button></div></form></li>`
    : `<li><button class="room-new" data-add-dept="1"><b>+ New department</b><span>Engineering, Design, Growth. Hire into it right after.</span></button></li>`;
  const working = s.employees.filter(e => e.status === "working").length;
  const items = [...(s.failed || []).map(t => ({ who: t.employee, name: t.name, text: `${t.key} stopped. Retry it?` })), ...s.reviews.map(r => ({ who: r.employee, name: r.name, text: `${r.key} is ready for review` })), ...s.inbox.map(q => ({ who: q.employee, name: q.name, text: q.kind === "handbook" ? "Handbook proposal" : q.question }))];
  const needs = items.slice(0, 4).map(i => `<button class="need" data-view="inbox">${face(i.name, "neutral")}<span><b>${esc(first(i.name))}</b> ${esc(i.text)}</span></button>`).join("");
  return `<div class="hq"><div>
      ${kickoffCard()}
      <div class="founder"><button class="me-btn" data-profile="1" title="Edit your profile">${me()}</button><div><b>${esc(myName())}</b><span>Founder${working ? ` · ${working} working now` : ""}</span></div><button class="btn ghost" data-add-dept="1">+ Department</button></div>
      <ul class="depts">${rooms}${add}</ul></div>
    <aside class="sidecol">
      ${items.length ? `<section class="panel"><h3>Needs you <span class="badge">${items.length}</span></h3><div class="needs">${needs}</div></section>` : ""}
      <section class="panel"><h3>Live <small class="muted">${s.serve ? "streaming" : "paused"}</small></h3><ul class="feed">${feed(s.feed)}</ul></section>
    </aside></div>`;
}
/* Kickoff: from what you're building to a plan of tickets */
function kickoffCard() {
  const k = S.snap.kickoff, team = S.snap.employees;
  if (k.planning.length) {
    const p = k.planning[0];
    return `<section class="kick busy">${face(p.name, "working")}<div><b>${esc(first(p.name))} is planning the first version</b>
      <span class="muted">Reading your brief and the repo. The plan shows up in your Inbox to review.</span></div>
      <button class="btn" data-ticket="${esc(p.key)}">Follow along</button></section>`;
  }
  if (k.planned || !team.length) return "";
  return `<form class="kick" id="kick"><div class="kick-body"><b>What are you building?</b>
      <span class="muted">The product, who it's for, what the first version needs. Your team plans it into tickets, and you start them when you're happy.</span>
      <textarea id="ko-brief" rows="4" placeholder="A notes app for writers. Magic link login, notes that sync, and export to Markdown. Free to start, $6 a month for more.">${esc(k.brief)}</textarea>
      <div class="acts"><select id="ko-by" aria-label="Who plans it">${team.map(e => `<option value="${esc(e.id)}" ${e.id === k.planner ? "selected" : ""}>${esc(first(e.name))}, ${esc(e.role)}</option>`).join("")}</select>
        <button class="btn primary">Plan it</button></div></div></form>`;
}

function feed(items) {
  if (!items.length) return `<li><span></span><span class="muted">Nothing yet. Start a ticket and it shows up here.</span></li>`;
  return items.map(f => `<li>${face(f.name, "neutral")}<div><b>${esc(first(f.name))}</b> <span class="key">${esc(f.key)}</span> ${esc(f.summary)}<time>${hhmm(f.at)}</time></div></li>`).join("");
}

/* Inbox: everything that needs you, answerable in one click */
function viewInbox() {
  const s = S.snap;
  const blocking = s.inbox.filter(q => q.kind !== "handbook" && q.blocking), other = s.inbox.filter(q => q.kind !== "handbook" && !q.blocking), books = s.inbox.filter(q => q.kind === "handbook");
  const plans = s.reviews.filter(r => r.kind === "plan"), work = s.reviews.filter(r => r.kind !== "plan");
  const cards = [...plans.map(planReview), ...(s.failed || []).map(stopped), ...blocking.map(q => question(q)), ...work.map(review), ...other.map(q => question(q)), ...books.map(handbook)];
  const leads = (s.with_leads || []).map(q => question(q, true));
  const zero = `<div class="empty-state"><b>Inbox zero</b>Your team puts questions, reviews and handbook ideas here when they need you.</div>`;
  return `<div class="inbox">${cards.length ? cards.join("") : zero}
    ${leads.length ? `<h2 class="section">With your leads <small class="muted">They decide what they can and pass up the rest. Step in anytime.</small></h2>${leads.join("")}` : ""}</div>`;
}
function stopped(t) {
  return `<article class="ask stopped">${face(t.name, "blocked")}<div>
    <span class="kind">Stopped · ${esc(t.key)}</span><h4>${esc(t.title)}</h4>
    <p>${esc(first(t.name))}'s run ${t.why && !/^stopped before/.test(t.why) ? `failed: ${esc(t.why)}` : "stopped before it finished."}</p>
    <div class="acts"><button class="btn primary" data-retry="${esc(t.key)}">Retry</button></div></div></article>`;
}
function question(q, withLead) {
  const opts = q.options.map(o => `<button class="btn primary" data-answer="${q.id}" data-val="${esc(o)}">${esc(o)}</button>`).join("");
  const def = q.default && !q.options.includes(q.default) ? `<button class="btn" data-answer="${q.id}" data-val="${esc(q.default)}">Go with ${esc(q.default)}</button>` : "";
  const kind = withLead ? `With ${esc(first(q.lead_name || "their lead"))}` : q.blocking ? "Blocking" : "Question";
  const via = q.via_name ? `<p class="via"><b>${esc(first(q.via_name))}</b> passed this to you${q.note ? `: ${esc(q.note)}` : "."}</p>` : "";
  return `<article class="ask ${q.blocking ? "blocking" : ""} ${withLead ? "lead" : ""}">${face(q.name, q.blocking ? "blocked" : "neutral")}<div>
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
  return `<article class="ask plan">${face(r.name, "done")}<div>
    <span class="kind">Plan · ${esc(r.key)} · by ${esc(first(r.name))}</span><h4>The first version, in ${r.tickets.length} tickets</h4>
    ${r.summary ? `<p>${esc(r.summary)}</p>` : ""}
    <ol class="plan-list">${rows || `<li class="muted">No tickets left. Plan again, or dismiss it.</li>`}</ol>
    <div class="acts"><button class="btn good" data-approve="${esc(r.key)}" ${r.tickets.length ? "" : "disabled"}>Start the plan</button><button class="btn" data-dismiss="${esc(r.key)}">Dismiss</button></div>
    ${more(`chg-${r.key}`, "Plan it again…", `<form class="reply" data-changes="${esc(r.key)}"><input id="chg-${esc(r.key)}" placeholder="What should change?"><button class="btn">Plan again</button></form>`)}</div></article>`;
}
function review(r) {
  return `<article class="ask review">${face(r.name, "done")}<div>
    <span class="kind">Review · ${esc(r.key)}</span><h4>${esc(r.title)}</h4>${r.pr_url ? `<p class="prline">${prLink(r.pr_url, r.pr_number)}</p>` : ""}<p>${esc(r.summary || `${first(r.name)} finished and is waiting for your review.`)}</p>
    <div class="acts"><button class="btn good" data-approve="${esc(r.key)}">${mergeLabel()}</button>${S.diffs[r.key] ? "" : `<button class="btn" data-diff="${esc(r.key)}">Show changes</button>`}</div>
    ${changes(r.key)}
    ${more(`chg-${r.key}`, "Ask for changes", `<form class="reply" data-changes="${esc(r.key)}"><input id="chg-${esc(r.key)}" placeholder="What should change?"><button class="btn">Send back</button></form>`)}</div></article>`;
}
function handbook(q) {
  return `<article class="ask">${face(q.name, "neutral")}<div>
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
      <form class="composer" id="say"><input id="say-text" placeholder="Message #${esc(ch.name)}. Use @name to ask someone" autocomplete="off"><button class="btn primary">Send</button></form>
    </section></div>`;
}
// The database keeps UTC. Shown in your own time.
const utc = at => new Date(String(at).replace(" ", "T") + "Z");
const hhmm = at => { const d = utc(at); return isNaN(d) ? "" : d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }); };
const sameMinutes = (a, b, n) => Math.abs(utc(b) - utc(a)) <= n * 60000;
// Mentions and ticket keys stand out, and a ticket key opens the ticket.
function chatText(text) {
  return esc(text).replace(/@([\p{L}\d]+)/gu, '<span class="mention">@$1</span>').replace(/\b([A-Z]{2,5}-\d+)\b/g, '<a class="tkey" data-ticket="$1">$1</a>');
}

/* Handbook: the file every employee reads before each ticket, shown as sections you can edit in place */
function parseHandbook(doc) {
  const book = { title: "", sections: [] };
  let cur = null;
  for (const line of String(doc || "").split("\n")) {
    if (line.startsWith("# ")) book.title = line.slice(2).trim();
    else if (line.startsWith("## ")) book.sections.push((cur = { name: line.slice(3).trim(), text: [], items: [] }));
    else if (cur && /^\s*[-*] /.test(line)) cur.items.push(line.replace(/^\s*[-*] /, ""));
    else if (cur && line.trim()) cur.text.push(line);
  }
  return book;
}
function writeHandbook(book) {
  const parts = book.sections.map(s => {
    const lines = [...s.text, ...s.items.map(i => "- " + i)];
    return `## ${s.name}\n` + (lines.length ? "\n" + lines.join("\n") + "\n" : "");
  });
  return `# ${book.title || S.company + " handbook"}\n\n` + parts.join("\n");
}
function viewHandbook() {
  if (S.handbook === null) return `<p class="muted">Opening the handbook…</p>`;
  const proposals = S.snap.inbox.filter(q => q.kind === "handbook");
  const props = proposals.length ? `<section class="panel"><h3>Proposed by your team <span class="badge">${proposals.length}</span></h3><div class="props">${proposals.map(q => `
      <div class="prop">${face(q.name, "neutral")}<div><b>${esc(first(q.name))}</b> <span class="muted">for ${esc(q.section)}</span><p>${esc(q.question)}</p>
        <div class="acts"><button class="btn primary" data-answer="${q.id}" data-val="yes">Add</button><button class="btn" data-answer="${q.id}" data-val="no">Not for everyone</button></div></div></div>`).join("")}</div></section>` : "";
  if (S.hbRaw) {
    return `<div class="book">${props}<form class="panel raw" id="hb-raw"><h3>Edit as text <small class="muted">Markdown. Sections start with ##</small></h3>
      <textarea id="hb-text" spellcheck="false">${esc(S.handbook)}</textarea>
      <div class="acts"><button class="btn primary">Save</button><button class="btn" type="button" data-hb-raw="0">Cancel</button></div></form></div>`;
  }
  const book = parseHandbook(S.handbook);
  const sections = book.sections.map((s, i) => `<section class="panel hb">
      <h3>${esc(s.name)}<small class="muted">${s.items.length ? s.items.length + (s.items.length === 1 ? " rule" : " rules") : ""}</small></h3>
      ${s.text.length ? `<p class="hb-text">${s.text.map(esc).join("<br>")}</p>` : ""}
      ${s.items.length ? `<ul class="rules">${s.items.map((it, k) => `<li><span>${esc(it)}</span><button class="x" data-hb-del="${i}:${k}" aria-label="Remove">×</button></li>`).join("")}</ul>` : ""}
      <form class="reply" data-hb-add="${esc(s.name)}"><input id="hb-add-${i}" placeholder="Add to ${esc(s.name)}…"><button class="btn">Add</button></form></section>`).join("");
  const brief = `<form class="panel hb" id="brief-form"><h3>What we're building<small class="muted">Everyone reads it. Planning starts from it.</small></h3>
      <textarea id="brief-text" class="brief-text" rows="5" placeholder="The product, who it's for, what the first version needs.">${esc(S.snap.kickoff.brief)}</textarea>
      <div class="acts brief-acts"><button class="btn">Save</button><button class="btn" type="button" data-replan="1">Save and plan it</button></div></form>`;
  return `<div class="book">
    ${brief}
    <p class="lede">Everyone reads this before every ticket. Decide something once, write it here, and the whole team follows it.</p>
    ${props}${sections || `<div class="empty-state"><b>Empty handbook</b>Add a section below, like Decisions or Conventions.</div>`}
    <form class="room-new" id="hb-section"><label for="hb-new"><b>+ New section</b></label><div class="row"><input id="hb-new" placeholder="Glossary, Tone of voice, Security…" autocomplete="off"><button class="btn">Add section</button></div></form>
    <div class="acts"><button class="btn" data-hb-raw="1">Edit as text</button></div></div>`;
}

/* Board: every ticket by where it stands. Click one for its log and actions. */
const COLUMNS = [["Backlog", ["backlog"]], ["Up next", ["todo", "queued", "waiting"]], ["Working", ["doing"]], ["Needs you", ["blocked", "review", "failed"]], ["Done", ["done"]]];
const PILL = { blocked: ["Question", "warn"], review: ["Review", "info"], failed: ["Stopped", ""], waiting: ["Waiting", ""] };
const spent = t => t.spend && !/^(\$0\.00|0 tok)$/.test(t.spend);
function viewBoard() {
  const s = S.snap;
  if (!s.tickets.length) return `<div class="empty-state"><b>No tickets yet</b>Plan the first version from the Company page, or add one with New ticket.</div>`;
  const nameOf = id => byId(id)?.name || id || "";
  const cols = COLUMNS.map(([title, statuses]) => {
    let ts = s.tickets.filter(t => statuses.includes(t.status));
    if (title === "Done") ts = ts.slice(-12);
    return `<section class="col"><h4>${title}<span>${ts.length}</span></h4>${ts.slice().reverse().map(t => `
      <button class="tk ${t.status} ${t.kind === "plan" ? "is-plan" : ""}" data-ticket="${esc(t.key)}"><span class="k">${esc(t.key)}${t.kind === "plan" ? " · plan" : ""}${PILL[t.status] ? `<span class="pill ${PILL[t.status][1]}">${PILL[t.status][0]}</span>` : ""}</span><span class="t">${esc(t.title)}</span>
        ${t.after && t.status !== "done" ? `<span class="after">after ${esc(t.after)}</span>` : ""}
        <span class="row">${t.assignee ? `${face(nameOf(t.assignee), "neutral")}<span class="muted">${esc(first(nameOf(t.assignee)))}</span>` : `<span class="muted">Nobody yet</span>`}${spent(t) ? `<span class="cost">${esc(t.spend)}</span>` : ""}</span></button>`).join("") || `<p class="muted none">Nothing here</p>`}</section>`;
  }).join("");
  return `<div class="board">${cols}</div>`;
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
  const log = t.log.map(a => `<div class="${esc(a.kind)}"><span class="at">${esc(String(a.at).slice(5, 16))}</span>${esc(a.kind)} ${esc(a.summary)}</div>`).join("");
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
    </div>`;
}
/* New ticket: details, who, what it waits for, and start now or later */
function drawerNewTicket() {
  const open = (S.snap?.tickets || []).filter(t => t.status !== "done"), team = S.snap?.employees || [];
  const pick = (team.find(e => e.status === "idle") || team[0])?.id;
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
  S.drawer = { type: "ticket", key }; S.ticket = null; renderDrawer();
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
  if (S.drawer.type === "ticket") { d.innerHTML = drawerTicket(); const term = $("#term"); if (term) term.scrollTop = term.scrollHeight; return; }
  const p = S.person;
  if (!p) return (d.innerHTML = `<div class="dhead"><h2>Loading</h2><button class="close" data-close="1" aria-label="Close">×</button></div>`);
  const e = byId(p.id) || { status: "idle" };
  const depts = S.snap.departments.filter(x => x.id !== null);
  const activity = p.activity.map(a => `<div class="${esc(a.kind)}"><span class="at">${esc(String(a.at).slice(11, 16))}</span>${esc(a.kind)} ${esc(a.summary)}</div>`).join("");
  d.innerHTML = `<div class="dhead">${face(p.name, e.status)}<div><h2>${esc(p.name)}</h2><span class="muted">${esc(p.role)}</span></div><button class="close" data-close="1" aria-label="Close">×</button></div>
    <div class="dbody">
      ${p.bio ? `<p style="margin:0">${esc(p.bio)}</p>` : ""}
      <dl class="kv"><dt>Department</dt><dd><select id="move" data-move="${esc(p.id)}"><option value="">None</option>${depts.map(x => `<option value="${x.id}" ${p.department === x.id ? "selected" : ""}>${esc(x.name)}</option>`).join("")}</select></dd>
        <dt>Works on</dt><dd>${chip(p)}</dd></dl>
      <div><h5>${p.current ? `Now · ${esc(p.current.key)} · ${esc(p.current.title)}` : "Now"}</h5>${p.current?.status === "doing" ? `<div class="acts" style="margin-bottom:8px"><button class="btn" data-stop="${esc(p.current.key)}">Stop this run</button></div>` : ""}${activity ? `<div class="term" id="term">${activity}</div>` : `<p class="muted" style="margin:0">Free. Give ${esc(first(p.name))} a ticket with New ticket.</p>`}</div>
      <div><h5>What ${esc(first(p.name))} has learned</h5>${p.memory.length ? `<ul class="notes">${p.memory.map(m => `<li>${esc(m)}</li>`).join("")}</ul>` : `<p class="muted" style="margin:0">Nothing yet. Notes build up as they work and as you review them.</p>`}</div>
      <div><h5>Tickets</h5>${p.tickets.length ? `<ul class="tlist">${p.tickets.map(t => `<li><button class="tl" data-ticket="${esc(t.key)}"><span class="k">${esc(t.key)}</span>${esc(t.title)}<span class="s chip">${esc(STATUS[t.status] || t.status)}</span></button></li>`).join("")}</ul>` : `<p class="muted" style="margin:0">None yet.</p>`}</div>
    </div>`;
  const term = $("#term"); if (term) term.scrollTop = term.scrollHeight;
}

// Candidates differ in how they work, not just in name: cheap and fast, senior, or one that can build and test.
const NAMES = ["Noor Haddad", "Jonas Berg", "Inês Costa", "Kwame Mensah", "Priya Raman", "Léa Moreau", "Mateo Silva", "Hana Sato", "Oskar Lind", "Amara Obi", "Rafael Duarte", "Mei Lin"];
const BIOS = [
  [/design/i, "Designs in code. Keeps screens simple and copy short, and follows the styles already in the repo."],
  [/review|cto|lead/i, "Reads diffs carefully. Flags risks, missing tests and new dependencies that don't earn their place."],
  [/writ|doc|market|growth|content/i, "Writes docs, changelogs and posts. Short sentences, plain words, no hype."],
  [/./, "Reads the code around a change before touching it. Makes the smallest diff that does the job and says what to check."],
];
function candidates(role) {
  const taken = new Set((S.snap?.employees || []).map(e => first(e.name).toLowerCase()));
  const pool = NAMES.filter(n => !taken.has(first(n).toLowerCase()));
  const start = hash(role) % Math.max(pool.length - 2, 1);
  const tiers = [
    { tier: "Quick and cheap", runtime: "claude", model: "haiku", line: "Does exactly what the ticket says. Great for small, clear tasks." },
    { tier: "Senior", runtime: "claude", model: "sonnet", line: "Pushes back on vague tickets and ships small, careful changes." },
    { tier: "Builds and tests", runtime: "codex", model: "", line: "Runs your build and tests in a sandbox before handing work back." },
  ];
  return tiers.map((t, i) => ({ ...t, name: pool[(start + i) % pool.length] || "New hire " + (i + 1) }));
}
function drawerHire() {
  const dept = S.snap.departments.find(x => x.id === S.drawer.dept), role = S.drawer.role;
  return `<div class="dhead"><div><h2>Hire into ${esc(dept?.name || "the company")}</h2><span class="muted">Pick how they work. They get a desk right away.</span></div><button class="close" data-close="1" aria-label="Close">×</button></div>
    <div class="dbody">
      <form class="field" id="role-form"><label for="role">Role</label><input id="role" value="${esc(role)}" autocomplete="off"></form>
      ${candidates(role).map((c, i) => `<div class="cand"><div class="who">${face(c.name, "neutral")}<div><b>${esc(c.name)}</b><div class="tier">${esc(c.tier)}</div></div></div>
        <q>${esc(c.line)}</q><div>${chip(c)}</div>
        <div><button class="btn primary" data-pick="${i}">Hire ${esc(first(c.name))}</button></div></div>`).join("")}
    </div>`;
}
async function openPerson(id) {
  S.drawer = { type: "person", id }; S.person = null; renderDrawer();
  try { S.person = await invoke("person", { company: S.company, id }); } catch (e) { toast(String(e), true); }
  renderDrawer();
}

/* Your profile: a name and a photo, kept on this Mac. No account. The first launch asks full screen,
   after that it's a small popover from your name in the sidebar. */
const photoActs = () => `${window.__TAURI__?.dialog ? `<button class="link-btn" type="button" data-pf="pick">${S.founder?.photo ? "Change photo" : "Choose a photo"}</button>` : ""}
  ${S.founder?.photo ? `<button class="link-btn" type="button" data-pf="face">Use a drawn face</button>` : ""}`.trim();
const pfPic = name => (S.founder?.photo ? me() : face(name || "You", "neutral"));
function focusTop(cancel) {
  return `<div class="focus-top" data-tauri-drag-region><span class="brand"><span class="mark">1</span>OneCorp</span>${cancel ? `<button class="btn ghost" type="button" data-su="cancel">Cancel</button>` : ""}</div>`;
}
function viewProfile() {
  return `<div class="focus-wrap">${focusTop(false)}
    <form class="welcome" id="profile">
      <span class="pf-pic big">${pfPic(S.profile.name)}</span>
      <h1>Who's running the company?</h1>
      <p class="muted">Your team works for you and knows you by name. Everything stays on this Mac. There's no account.</p>
      <input class="big-input" id="pf-name" value="${esc(S.profile.name)}" placeholder="Your name" autocomplete="off" aria-label="Your name">
      <div class="photo-acts">${photoActs() || `<span class="muted">This face is drawn from your name.</span>`}</div>
      <button class="btn primary lg">Continue</button>
      <p class="fine">Next: your first company.</p>
    </form></div>`;
}
function renderPop() {
  const p = $("#pop");
  p.hidden = !S.pop;
  if (!S.pop) return;
  p.innerHTML = `<form id="profile" class="pop-form">
    <div class="pop-head"><span class="pf-pic">${pfPic(myName())}</span><div class="acts">${photoActs()}</div></div>
    <div class="field"><label for="pf-name">Your name</label><input id="pf-name" value="${esc(S.founder?.name || "")}" autocomplete="off"></div>
    <p class="fine">Your team knows you by this name. It stays on this Mac.</p>
    <div class="acts end"><button class="btn" type="button" data-pf="cancel">Cancel</button><button class="btn primary">Save</button></div></form>`;
}
async function saveProfile(photo, removePhoto) {
  const name = ($("#pf-name")?.value ?? S.profile.name).trim();
  if (!name) { toast("Say what to call you first", true); $("#pf-name")?.focus(); return false; }
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
    S.setup = { step: 0, mode: "existing", path: "", repo: null, name: "", mission: "", runtime: real[0] || "fake", info, repoInfo,
      folder: "", parent: repoInfo.parent || "", owner: "", private: true, people: [] };
    setTemplate(real.length ? "small" : "demo");
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
function viewSetup() {
  const s = S.setup, steps = ["Repo", "Name", "Team", "Review"];
  const dots = `<ol class="su-steps">${steps.map((n, i) => `<li class="${i === s.step ? "on" : i < s.step ? "done" : ""}">${n}</li>`).join("")}</ol>`;
  const back = s.step ? `<button class="btn" type="button" data-su="back">Back</button>` : "<span></span>";
  const nav = (label, disabled) => `<div class="su-nav">${back}<button class="btn primary" ${disabled ? "disabled" : ""}>${label}</button></div>`;
  const picker = window.__TAURI__?.dialog;
  let body = "";
  if (s.step === 0) {
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
      body = `<h2>Start from just an idea</h2><p class="muted">A new folder with git, a README and a first commit, ready for your team.</p>${tabs}
        <div class="field"><label for="su-folder">Folder name</label><input id="su-folder" value="${esc(s.folder)}" placeholder="notes-app" autocomplete="off" spellcheck="false"></div>
        <div class="field"><label for="su-parent">Inside</label><div class="su-row">${picker ? `<button class="btn" type="button" data-su="parent">Choose…</button>` : ""}<input id="su-parent" value="${esc(s.parent)}" autocomplete="off" spellcheck="false"></div></div>
        ${github}
        <div class="su-nav">${back}<button class="btn primary" type="button" data-su="create-repo" ${s.creating ? "disabled" : ""}>${s.creating ? "Creating…" : "Create the repo"}</button></div>`;
    } else {
      const r = s.repo;
      const result = !r ? "" : r.ok
        ? `<p class="su-ok">✓ A git repo on <b>${esc(r.branch)}</b>.</p>${shipNote(r)}${r.used_by.length ? `<p class="su-warn">${esc(r.used_by.join(", "))} already works on this repo. That's fine, they'll just share it.</p>` : ""}`
        : `<p class="su-bad">${esc(r.problem)}</p>`;
      body = `<h2>Which repo does this company work on?</h2><p class="muted">Employees work in their own copies of it, so your checkout is never touched.</p>${tabs}
        <div class="su-row">${picker ? `<button class="btn" type="button" data-su="pick">Choose a folder…</button>` : ""}<input id="su-path" value="${esc(s.path)}" placeholder="/Users/you/dev/your-repo" autocomplete="off" spellcheck="false"><button class="btn" type="button" data-su="check">Check</button></div>
        ${result}${nav("Continue", !(r && r.ok))}`;
    }
  } else if (s.step === 1) {
    body = `<h2>Name your company</h2><p class="muted">Tickets are numbered from its first letters, like ${esc((s.name || "acme").replace(/[^a-z0-9]/gi, "").slice(0, 3).toUpperCase() || "ACM")}-1.</p>
      <div class="field"><label for="su-name">Name</label><input id="su-name" value="${esc(s.name)}" placeholder="snowztech" autocomplete="off" spellcheck="false"></div>
      <div class="field"><label for="su-mission">Mission <span class="muted">(optional, it opens the handbook)</span></label><input id="su-mission" value="${esc(s.mission)}" placeholder="Open source tools for building AI agents" autocomplete="off"></div>
      <div class="field"><label for="su-brief">What are you building? <span class="muted">(optional. Your team plans it as soon as the company exists)</span></label><textarea id="su-brief" rows="4" placeholder="A notes app for writers. Magic link login, sync, export to Markdown.">${esc(s.brief || "")}</textarea></div>
      ${nav("Continue")}`;
  } else if (s.step === 2) {
    const real = s.info.runtimes.filter(x => x !== "fake");
    const runtimeOptions = sel => s.info.runtimes.map(x => `<option value="${x}" ${x === sel ? "selected" : ""}>${RUNTIME_NAMES[x] || x}</option>`).join("");
    const rows = s.people.map((p, i) => `<li class="person-row">${face(p.name || "?", "neutral")}
      <input id="su-p${i}-name" value="${esc(p.name)}" placeholder="Name" aria-label="Name" autocomplete="off">
      <input id="su-p${i}-role" value="${esc(p.role)}" placeholder="Role" aria-label="Role" autocomplete="off">
      <input id="su-p${i}-dept" value="${esc(p.dept)}" placeholder="Department" aria-label="Department" list="su-depts" autocomplete="off">
      <select id="su-p${i}-runtime" aria-label="Works on">${runtimeOptions(p.runtime)}</select>
      <button class="x" type="button" data-su="del-${i}" aria-label="Remove ${esc(p.name)}">×</button></li>`).join("");
    const depts = [...new Set(s.people.map(p => p.dept).filter(Boolean).concat(["Engineering", "Product", "Design", "Growth"]))];
    body = `<h2>Who's on the team to start?</h2><p class="muted">Start from a template, then change anything: names, roles, departments. The first person in a department leads it.</p>
      <div class="tpl-chips">${s.info.templates.map(x => {
        const off = !real.length && !x.runtime && x.hires.length;
        return `<button type="button" class="chip-btn" data-tpl="${x.key}" aria-pressed="${x.key === s.template}" ${off ? "disabled" : ""} title="${esc(x.description)}">${esc(x.name)}</button>`;
      }).join("")}</div>
      ${!real.length ? `<p class="su-warn">Neither Claude Code nor Codex is installed here, so only the free stand-in works. Install one later and rehire on it.</p>` : ""}
      <ul class="people">${rows || `<li class="muted">Nobody yet. Add someone, or start empty and hire later.</li>`}</ul>
      <datalist id="su-depts">${depts.map(d => `<option value="${esc(d)}">`).join("")}</datalist>
      <button class="btn" type="button" data-su="add-person">+ Add someone</button>
      ${nav("Continue")}`;
  } else {
    body = `<h2>Ready to open ${esc(s.name)}</h2>
      <dl class="kv su-sum"><dt>Repo</dt><dd class="mono">${esc(s.repo.path)} <span class="muted">on ${esc(s.repo.branch)}</span></dd>
        <dt>Mission</dt><dd>${esc(s.mission || "None yet")}</dd>
        <dt>Ships</dt><dd>${s.repo.forge && s.repo.signed_in ? `As ${esc(s.repo.pr_word)}s on ${esc(s.repo.forge)}` : "By merging on your Mac"}</dd>
        ${s.brief && s.people.length ? `<dt>First plan</dt><dd>Planning starts as soon as it's created</dd>` : ""}
        <dt>Team</dt><dd>${s.people.length ? s.people.map(p => `${esc(p.name)} <span class="muted">${esc(p.role)}${p.dept ? ", " + esc(p.dept) : ""}</span>`).join("<br>") : "Nobody yet"}</dd></dl>
      ${nav("Create the company")}`;
  }
  return `<div class="focus-wrap">${focusTop(S.companies.length)}<form class="setup ${s.step === 2 ? "wide" : ""}" id="setup">${dots}${body}</form></div>`;
}

/* Rendering and polling */
function render() {
  // Background refreshes re-render too, so keep whatever is typed in the new company flow.
  if (S.setup && $("#setup")) readSetup();
  if (S.profile && $("#pf-name")) S.profile.name = $("#pf-name").value;
  renderSide(); renderTop();
  const v = $("#view");
  // Setting up is full screen: no sidebar or top bar for things that don't exist yet.
  document.querySelector(".app").classList.toggle("focus", !!(S.profile || S.setup));
  v.classList.toggle("flush", !S.profile && !S.setup && S.view === "chat");
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
  try {
    if (S.founder === undefined) S.founder = await invoke("founder");
    // A first launch says hello before anything else: who you are, then your first company.
    if (S.founder === null && !S.profile && !S.setup) S.profile = { name: "" };
    const c = await invoke("companies");
    S.companies = c.list;
    if (!S.company || !c.list.includes(S.company)) S.company = c.current;
    if (!S.company) {
      S.snap = null;
      if (!S.setup && !S.profile) return startSetup(); // No company yet: create one.
      return render();
    }
    const snap = await invoke("snapshot", { company: S.company });
    // The handbook is a small file and can change outside the app, so it's read with every refresh.
    const book = S.view === "handbook" ? await invoke("handbook", { company: S.company }) : S.handbook;
    if (S.view === "chat") S.chat = await invoke("chat", { company: S.company, channel: S.channel }).catch(() => S.chat);
    const json = JSON.stringify(snap) + (book ?? "") + (S.view === "chat" ? JSON.stringify(S.chat) : "");
    if (!force && (json === S.last || typing())) return;
    S.snap = snap; S.last = json; S.handbook = book;
    // Channels count as read the first time they're seen, so opening the app doesn't light up every dot.
    for (const ch of snap.channels || []) if (!(ch.name in S.seen)) S.seen[ch.name] = ch.last || 0;
    if (S.drawer?.type === "person") S.person = await invoke("person", { company: S.company, id: S.drawer.id }).catch(() => S.person);
    if (S.drawer?.type === "ticket") S.ticket = await invoke("ticket_detail", { company: S.company, key: S.drawer.key }).catch(() => S.ticket);
    render();
  } catch (e) {
    toast(String(e), true);
  }
}

/* Actions */
document.addEventListener("click", async e => {
  if (S.pop && !e.target.closest("#pop, [data-profile]")) { S.pop = false; renderPop(); }
  if (e.target.id === "scrim") { S.drawer = null; return renderDrawer(); }
  const el = e.target.closest("[data-plan-del],[data-dismiss],[data-replan],[data-new-ticket],[data-nt],[data-start],[data-more],[data-channel],[data-mention],[data-profile],[data-pf],[data-open-url],[data-ticket],[data-stop],[data-new-co],[data-su],[data-tpl],[data-retry],[data-hb-del],[data-hb-raw],[data-diff],[data-hide-diff],[data-co],[data-view],[data-person],[data-hire],[data-add-dept],[data-close],[data-answer],[data-approve],[data-pick]");
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
  if (d.newTicket) { S.drawer = { type: "new-ticket" }; renderDrawer(); return $("#nt-title")?.focus(); }
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
  if (d.more) { S.open[d.more] = true; render(); return document.getElementById(d.more)?.focus(); }
  if (d.profile) { S.pop = !S.pop; renderPop(); return $("#pf-name")?.focus(); }
  if (d.pf) {
    if (d.pf === "cancel") { S.pop = false; return renderPop(); }
    if (d.pf === "face") { if (await saveProfile(null, true)) { render(); renderPop(); } return; }
    if (d.pf === "pick") {
      const path = await window.__TAURI__.dialog.open({ multiple: false, title: "Pick a photo of you", filters: [{ name: "Photos", extensions: ["png", "jpg", "jpeg", "webp", "gif", "heic"] }] }).catch(() => null);
      // Saved straight away, so the preview shows the copy OneCorp keeps.
      if (path && await saveProfile(String(path))) { render(); renderPop(); }
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
  if (d.newCo) return startSetup();
  if (d.tpl) { readSetup(); setTemplate(d.tpl); return render(); }
  if (d.su) {
    readSetup();
    const s = S.setup;
    if (d.su === "cancel") { S.setup = null; S.view = "company"; return refresh(true); }
    if (d.su === "back") { s.step--; return render(); }
    if (d.su === "check") return checkRepo(s.path);
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
  if (d.hbDel) {
    const [i, k] = d.hbDel.split(":").map(Number), book = parseHandbook(S.handbook);
    const gone = book.sections[i].items.splice(k, 1)[0];
    await call("save_handbook", { company: S.company, text: writeHandbook(book) }, `Removed: ${gone}`).catch(() => {});
    return;
  }
  if (d.hbRaw) { S.hbRaw = d.hbRaw === "1"; return render(); }
  if (d.diff) {
    S.diffs[d.diff] = "loading"; render();
    try { S.diffs[d.diff] = await invoke("review", { company: S.company, key: d.diff }); }
    catch (err) { delete S.diffs[d.diff]; toast(String(err), true); }
    return render();
  }
  if (d.hideDiff) { delete S.diffs[d.hideDiff]; return render(); }
  if (d.co) { S.setup = null; S.company = d.co; S.snap = null; S.last = ""; render(); await call("use_company", { name: d.co }); }
  else if (d.view) { S.view = d.view; S.hbRaw = false; if (d.close) S.drawer = null; render(); if (d.view === "handbook") refresh(true); }
  else if (d.person) openPerson(d.person);
  else if (d.hire) { const dept = S.snap.departments.find(x => String(x.id) === d.hire); S.drawer = { type: "hire", dept: dept.id, role: /eng/i.test(dept.name) ? "Engineer" : dept.name + " specialist" }; renderDrawer(); }
  else if (d.addDept) { S.addingDept = true; S.view = "company"; render(); $("#dept-name")?.focus(); }
  else if (d.close) { S.drawer = null; renderDrawer(); }
  else if (d.answer) { el.disabled = true; await call("answer", { company: S.company, id: Number(d.answer), text: d.val }, d.val === "no" ? "Left out of the handbook" : "Answered").catch(() => (el.disabled = false)); }
  else if (d.approve) { el.disabled = true; el.textContent = "Merging…"; await call("approve", { company: S.company, key: d.approve }, `${d.approve} merged. New tickets start from it.`).catch(() => { el.disabled = false; el.textContent = mergeLabel(); }); }
  else if (d.pick) {
    const role = $("#role").value.trim() || S.drawer.role, c = candidates(role)[Number(d.pick)];
    const bio = BIOS.find(([re]) => re.test(role))[1];
    el.disabled = true;
    try {
      await call("hire", { company: S.company, name: c.name, role, bio, runtime: c.runtime, model: c.model || null, department: S.drawer.dept }, `${c.name} joined. Give them a ticket with New ticket.`);
      S.drawer = null; renderDrawer();
    } catch { el.disabled = false; }
  }
});
document.addEventListener("submit", async e => {
  e.preventDefault();
  const f = e.target;
  if (f.id === "kick") {
    const brief = $("#ko-brief").value.trim();
    if (!brief) { toast("Say what you're building first", true); return $("#ko-brief").focus(); }
    const btn = f.querySelector(".btn.primary");
    btn.disabled = true;
    await call("kickoff", { company: S.company, brief, by: $("#ko-by").value || null }, "Planning started. The plan shows up in your Inbox").catch(() => (btn.disabled = false));
    return;
  }
  if (f.id === "brief-form") {
    await call("save_brief", { company: S.company, brief: $("#brief-text").value }, "Brief saved. Everyone reads it from their next ticket").catch(() => {});
    return;
  }
  if (f.id === "profile") {
    const firstTime = !!S.profile;
    if (!(await saveProfile())) return;
    if (!firstTime) { S.pop = false; renderPop(); render(); return toast("Saved"); }
    S.profile = null;
    toast(`Welcome, ${myName()}.`);
    if (!S.companies.length) return startSetup();
    return refresh(true);
  }
  if (f.id === "setup") {
    readSetup();
    const s = S.setup;
    if (s.step === 0) {
      if (!s.repo || s.checked !== s.path) await checkRepo(s.path);
      if (s.repo?.ok) { s.step = 1; render(); $("#su-name")?.focus(); }
      return;
    }
    if (s.step === 1) {
      if (!/^[A-Za-z0-9_-]+$/.test(s.name)) return toast("A company name can use letters, digits, - and _, like snowztech or my-notes", true);
      if (S.companies.includes(s.name)) return toast(`There's already a company called ${s.name}`, true);
      s.step = 2; return render();
    }
    if (s.step === 2) {
      // The same check the core makes, early, so you fix it here and not after a failed create.
      const ids = s.people.filter(p => p.name.trim()).map(p => p.name.trim().split(/\s+/)[0].toLowerCase());
      const twice = ids.find((id, i) => ids.indexOf(id) !== i);
      if (twice) return toast(`Two people would both be ${twice}. Give one of them a different first name`, true);
      s.step = 3; return render();
    }
    const btn = f.querySelector(".btn.primary");
    btn.disabled = true; btn.textContent = "Creating…";
    try {
      const people = s.people.filter(p => p.name.trim());
      const name = await invoke("create_company", { name: s.name, repo: s.repo.path, mission: s.mission, people, brief: s.brief || null });
      S.setup = null; S.company = name; S.view = "company"; S.snap = null; S.last = "";
      toast(`${name} is open. Give someone a ticket with New ticket.`);
      await refresh(true);
    } catch (e) { toast(String(e), true); btn.disabled = false; btn.textContent = "Create the company"; }
    return;
  }
  if (f.id === "say") {
    const input = $("#say-text"), text = input.value.trim();
    if (!text) return input.focus();
    input.value = "";
    await call("say", { company: S.company, channel: S.channel, text }).catch(() => (input.value = text));
    return $("#say-text")?.focus();
  }
  if (f.id === "dept-form") {
    const name = $("#dept-name").value.trim();
    if (!name) return $("#dept-name").focus();
    const id = await call("add_department", { company: S.company, name }, `${name} is set up. Hire its first person.`).catch(() => null);
    if (id !== null) { S.addingDept = false; S.drawer = { type: "hire", dept: id, role: /eng/i.test(name) ? "Engineer" : name + " specialist" }; render(); }
  } else if (f.dataset.hbAdd) {
    const input = f.querySelector("input"), text = input.value.trim();
    if (!text) return input.focus();
    await call("add_to_handbook", { company: S.company, section: f.dataset.hbAdd, text }, `Added to ${f.dataset.hbAdd}. Everyone sees it from their next ticket.`).catch(() => {});
  } else if (f.id === "hb-section") {
    const name = $("#hb-new").value.trim();
    if (!name) return $("#hb-new").focus();
    const book = parseHandbook(S.handbook);
    if (book.sections.some(s => s.name.toLowerCase() === name.toLowerCase())) return toast(`There's already a ${name} section`, true);
    book.sections.push({ name, text: [], items: [] });
    await call("save_handbook", { company: S.company, text: writeHandbook(book) }, `Added the ${name} section`).catch(() => {});
  } else if (f.id === "hb-raw") {
    await call("save_handbook", { company: S.company, text: $("#hb-text").value }, "Handbook saved").then(() => { S.hbRaw = false; render(); }).catch(() => {});
  } else if (f.id === "role-form") {
    S.drawer.role = $("#role").value.trim() || S.drawer.role; renderDrawer();
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
document.addEventListener("input", e => {
  if (e.target.id === "pf-name" && S.profile && !S.founder?.photo) $(".welcome .pf-pic").innerHTML = face(e.target.value || "You", "neutral");
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
  if (e.key === "Escape" && S.pop) { S.pop = false; return renderPop(); }
  if (e.key === "Escape") { if (S.drawer) { S.drawer = null; renderDrawer(); } else if (S.addingDept) { S.addingDept = false; render(); } }
  if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k" && S.snap && !S.setup && !S.profile) { e.preventDefault(); S.drawer = { type: "new-ticket" }; renderDrawer(); $("#nt-title")?.focus(); }
});

// In a plain browser, #board, #inbox, #handbook, #ticket, #person, #new-ticket, #setup or #profile opens
// straight onto that screen, so each one can be looked at and screenshotted. Never used in the app.
async function previewScreen() {
  const screen = location.hash.slice(1);
  await refresh(true);
  if (["board", "inbox", "handbook", "chat"].includes(screen)) { S.view = screen; render(); }
  if (screen === "ticket") openTicket("SNO-11");
  if (screen === "person") openPerson("tomas");
  if (screen === "new-ticket") { S.view = "board"; S.drawer = { type: "new-ticket" }; render(); }
  if (screen === "setup") await startSetup();
  if (screen.startsWith("setup-")) { await startSetup(); await checkRepo("/Users/you/dev/snowztech"); S.setup.step = Number(screen.slice(6)); render(); }
  if (screen === "profile") { S.founder = null; S.profile = { name: "" }; render(); }
  if (screen === "popover") { S.pop = true; renderPop(); }
}
if (window.__TAURI__) refresh(true); else previewScreen();
setInterval(() => refresh(false), 1500);

/* Sample data for opening ui/index.html in a browser. Never used inside the app. */
function preview(cmd) {
  const team = [
    ["odile", "Odile Martin", "CTO", "claude", "opus", "working", "Reading the SNO-12 diff", 1], ["maya", "Maya Okafor", "Senior engineer", "codex", null, "blocked", "Need you on SNO-12", 1],
    ["tomas", "Tomás Reis", "Engineer", "codex", null, "working", "Bash cargo test billing", 1], ["rui", "Rui Alves", "Product lead", "claude", "sonnet", "working", "Created SNO-18", 2],
    ["lina", "Lina Park", "Product designer", "claude", "sonnet", "queued", "Up next: SNO-14", 2], ["sam", "Sam Duarte", "Writer", "claude", "haiku", "idle", "", 3],
  ].map(([id, name, role, runtime, model, status, bubble, department]) => ({ id, name, role, runtime, model, status, bubble, department, spend: "$0.40", ticket: null }));
  const data = {
    companies: { list: ["snowztech", "tvcask", "autoremind", "pingbuddy"], current: "snowztech" },
    snapshot: {
      company: "snowztech", serve: true, kickoff: { brief: "A notes app for writers.", planner: "rui", planned: true, planning: [] }, ship: { mode: "github", forge: "GitHub", pr_word: "pull request" }, employees: team, tickets: [
        { key: "SNO-9", title: "Rate limit the public API", assignee: "tomas", status: "failed", spend: "40k tok" },
        { key: "SNO-11", title: "Pricing page", assignee: "odile", status: "review", spend: "$0.31" },
        { key: "SNO-12", title: "Magic link login", assignee: "maya", status: "blocked", spend: "52k tok" },
        { key: "SNO-14", title: "Empty state for the notes list", assignee: "lina", status: "queued", spend: "$0.00" },
        { key: "SNO-15", title: "Retry failed Stripe webhooks", assignee: "tomas", status: "doing", spend: "75k tok" },
        { key: "SNO-8", title: "CI pipeline", assignee: "tomas", status: "done", spend: "$0.22" },
        { key: "SNO-16", title: "Export notes as Markdown", assignee: null, status: "backlog", spend: "$0.00", created_by: "Rui Alves" },
        { key: "SNO-17", title: "Notes API pagination", assignee: "maya", status: "waiting", spend: "$0.00", after: "SNO-12" }],
      departments: [{ id: 1, name: "Engineering", lead: "odile", members: ["odile", "maya", "tomas"] }, { id: 2, name: "Product", lead: "rui", members: ["rui", "lina"] }, { id: 3, name: "Growth", lead: "sam", members: ["sam"] }],
      with_leads: [{ id: 5, kind: "question", key: "SNO-15", employee: "tomas", name: "Tomás Reis", lead_name: "Odile Martin", question: "Retry webhooks 3 or 5 times?", section: "", options: ["3", "5"], default: "3", blocking: false }],
      inbox: [{ id: 2, kind: "question", key: "SNO-12", employee: "maya", name: "Maya Okafor", question: "Magic links: 15 or 60 minute expiry?", section: "", options: ["15 minutes", "60 minutes"], default: "15 minutes", blocking: true, via: "odile", via_name: "Odile Martin", note: "it changes what users see, so it's a product call" },
        { id: 3, kind: "handbook", key: "SNO-14", employee: "lina", name: "Lina Park", question: "Empty states are one sentence and one button", section: "Conventions", options: [], default: "", blocking: false }],
      failed: [{ key: "SNO-9", title: "Rate limit the public API", employee: "tomas", name: "Tomás Reis", why: "stopped before it finished" }],
      reviews: [{ key: "SNO-20", title: "Plan the first version", employee: "rui", name: "Rui Alves", kind: "plan", summary: "Setup first, then login and sync in parallel. Risk: magic links need an email provider.", spend: "$0.21",
          tickets: [{ key: "SNO-21", title: "Set up the project", description: "A skeleton that builds, with a README.", assignee: "maya", name: "Maya Okafor", after: null, status: "backlog" },
            { key: "SNO-22", title: "Magic link login", description: "Email a one-time link, 15 minute expiry.", assignee: "tomas", name: "Tomás Reis", after: "SNO-21", status: "backlog" }] },
        { key: "SNO-11", kind: "work", tickets: [], title: "Pricing page", employee: "odile", name: "Odile Martin", summary: "Added the pricing page with Free and Pro plans. Check the copy on mobile.", spend: "$0.31", pr_number: 41, pr_url: "https://github.com/snowztech/notes/pull/41" }],
      feed: [{ at: "2026-11-09 11:02:00", key: "SNO-11", employee: "odile", name: "Odile Martin", kind: "done", summary: "finished, $0.31" }, { at: "2026-11-09 10:52:00", key: "SNO-15", employee: "tomas", name: "Tomás Reis", kind: "tool", summary: "Bash cargo test billing" }],
      spend: { cost: 3.24, tokens: 180000, label: "$3.24" },
      channels: [{ name: "general", title: "General", department: null, last: 9 }, { name: "engineering", title: "Engineering", department: 1, last: 8 }, { name: "product", title: "Product", department: 2, last: 4 }, { name: "growth", title: "Growth", department: 3, last: null }],
    },
    handbook: "# snowztech handbook\n\n## Mission\n\nOpen source tools for building AI agents.\n\n## Decisions\n\n- Standard library first. No new dependencies without asking the founder.\n- Auth tokens are hashed at rest with sha2.\n\n## Conventions\n\n- All program output is lowercase, with no trailing punctuation.\n- CLI flags are long-form, kebab-case.\n",
    setup_info: { runtimes: ["claude", "fake"], templates: [
      { key: "solo", name: "Just me and one engineer", description: "One engineer to start. Hire the rest when you need them.", runtime: null, hires: [{ name: "Maya Okafor", role: "Engineer", dept: "Engineering" }] },
      { key: "small", name: "Small product team", description: "A CTO who leads Engineering and reviews, an engineer, and a designer in Product.", runtime: null, hires: [{ name: "Odile Martin", role: "CTO", dept: "Engineering" }, { name: "Maya Okafor", role: "Engineer", dept: "Engineering" }, { name: "Lina Park", role: "Product designer", dept: "Product" }] },
      { key: "demo", name: "Try it for free", description: "The small team on a stand-in with no AI.", runtime: "fake", hires: [{ name: "Odile Martin", role: "CTO", dept: "Engineering" }] },
      { key: "empty", name: "Start empty", description: "No one yet.", runtime: null, hires: [] }] },
    ticket_detail: { key: "SNO-11", title: "Pricing page", status: "review", description: "Free and Pro plans. Pro is $6 a month.", created_by: "Rui Alves", after: null, pr_number: 41, pr_url: "https://github.com/snowztech/notes/pull/41", employee: "odile", name: "Odile Martin", branch: "onecorp/snowztech/sno-11", base: "main", spend: "$0.31",
      log: [{ at: "2026-11-09 10:40:00", kind: "start", summary: "claude (opus), started" }, { at: "2026-11-09 10:58:00", kind: "commit", summary: "c3dbf20 SNO-11: Pricing page" }, { at: "2026-11-09 11:02:00", kind: "done", summary: "finished, $0.31" }] },
    new_repo_info: { parent: "/Users/you/dev", owners: ["you", "snowztech"] },
    check_repo: { ok: true, path: "/Users/you/dev/snowztech", branch: "main", suggested: "snowztech", used_by: [], forge: "GitHub", pr_word: "pull request", signed_in: true, account: "Logged in to github.com account you" },
    founder: { name: "Lucas", photo: null },
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
