// OneCorp app UI. It polls the company snapshot and renders the Company page and the Inbox.
// Every action calls a Tauri command, which runs the same library code as the CLI.
// Opened in a plain browser (no Tauri), it runs on sample data so the design can be checked quickly.

const invoke = window.__TAURI__ ? window.__TAURI__.core.invoke : preview;
const $ = s => document.querySelector(s);
const esc = s => String(s ?? "").replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]));
const first = n => String(n || "").split(" ")[0];

const S = { companies: [], company: null, snap: null, last: "", view: "company", drawer: null, person: null, addingDept: false };

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
const byId = id => (S.snap?.employees || []).find(e => e.id === id);
const waiting = () => (S.snap ? S.snap.inbox.length + S.snap.reviews.length : 0);

/* Sidebar and top bar */
function renderSide() {
  const n = waiting(), pages = { company: "Company", inbox: "Inbox" };
  $("#side").innerHTML = `
    <div class="cos">${S.companies.map(c => `<button class="cobtn" data-co="${esc(c)}" aria-current="${c === S.company}"><span class="logo">${esc(c[0].toUpperCase())}</span><span class="coname">${esc(c)}</span></button>`).join("")}</div>
    ${S.company ? `<h6>${esc(S.company)}</h6>${Object.entries(pages).map(([k, v]) =>
      `<button class="navbtn" data-view="${k}" ${S.view === k ? 'aria-current="page"' : ""}>${v}${k === "inbox" && n ? `<span class="badge">${n}</span>` : ""}</button>`).join("")}` : ""}
    ${S.snap ? `<div class="serve ${S.snap.serve ? "" : "off"}"><i></i>${S.snap.serve ? "Your team is working" : "Nobody is running work"}</div>` : ""}
    <div class="side-foot"><span class="logo" style="background:var(--accent);color:#fff;border-radius:50%">Y</span><div><b>You</b><span>Founder</span></div></div>`;
}
function renderTop() {
  const s = S.snap;
  $("#title").textContent = S.view === "inbox" ? "Inbox" : S.company || "OneCorp";
  const depts = s ? s.departments.filter(d => d.id !== null).length : 0;
  $("#sub").textContent = s ? `${s.employees.length} ${s.employees.length === 1 ? "person" : "people"} · ${depts} ${depts === 1 ? "department" : "departments"}` : "";
  $("#spend").innerHTML = s ? `Spent <b>${esc(s.spend.label)}</b>` : "";
  const sel = $("#give-to"), keep = sel.value;
  sel.innerHTML = (s?.employees || []).map(e => `<option value="${esc(e.id)}">${esc(first(e.name))}${e.status === "idle" ? " (free)" : ""}</option>`).join("");
  if (keep && s?.employees.some(e => e.id === keep)) sel.value = keep;
  else { const free = s?.employees.find(e => e.status === "idle"); if (free) sel.value = free.id; }
  $("#give").hidden = !s || !s.employees.length;
}

/* Company page: you on top, departments beneath as rooms of desks */
function desk(e, lead) {
  const show = e.bubble ? "show" : "";
  return `<button class="desk ${e.status}" data-person="${esc(e.id)}">
    <span class="bubble ${show}">${esc(e.bubble)}</span>
    <span class="station"><span class="seat">${face(e.name, e.status)}</span><span class="monitor"><i></i><i></i><i></i><i></i></span></span>
    <span class="table"></span>${e.status === "idle" ? '<span class="zz">z</span>' : ""}
    <span class="plate"><b>${esc(first(e.name))}${lead ? '<em class="leadtag">Lead</em>' : ""}</b><span>${esc(e.role)}</span></span>
    <span class="mini">${chip(e)}</span></button>`;
}
function viewCompany() {
  const s = S.snap;
  const rooms = s.departments.map(d => {
    const ms = d.members.map(byId).filter(Boolean), lead = d.lead && byId(d.lead);
    const busy = ms.filter(e => e.status === "working").length, stuck = ms.filter(e => e.status === "blocked").length;
    const meta = [lead ? "Led by " + esc(first(lead.name)) : d.id === null ? "Move them into a department from their desk" : "No lead yet", ms.length + (ms.length === 1 ? " person" : " people")];
    if (busy) meta.push(`<span class="ok">${busy} working</span>`);
    if (stuck) meta.push(`<span class="warn">${stuck} waiting on you</span>`);
    const hire = d.id === null ? "" : `<button class="btn" data-hire="${d.id}">+ Hire</button>`;
    const empty = d.id === null ? "" : `<button class="desk empty" data-hire="${d.id}"><span class="station"><span class="seat ghost"></span></span><span class="table"></span><span class="plate"><b>Empty desk</b><span>Hire into ${esc(d.name)}</span></span></button>`;
    return `<li><section class="room"><header class="room-h"><div><h3>${esc(d.name)}</h3><p>${meta.join(" · ")}</p></div>${hire}</header>
      <div class="room-floor">${ms.map(e => desk(e, d.lead === e.id)).join("")}${empty}</div></section></li>`;
  }).join("");
  const add = S.addingDept
    ? `<li><form class="room-new" id="dept-form"><label for="dept-name"><b>Name the department</b></label><div class="row"><input id="dept-name" placeholder="Engineering, Design, Growth…" autocomplete="off"><button class="btn primary">Add</button></div></form></li>`
    : `<li><button class="room-new" data-add-dept="1"><b>+ New department</b><span>Engineering, Design, Growth. Hire into it right after.</span></button></li>`;
  const working = s.employees.filter(e => e.status === "working").length;
  const items = [...s.reviews.map(r => ({ who: r.employee, name: r.name, text: `${r.key} is ready for review` })), ...s.inbox.map(q => ({ who: q.employee, name: q.name, text: q.kind === "handbook" ? "Handbook proposal" : q.question }))];
  const needs = items.slice(0, 4).map(i => `<button class="need" data-view="inbox">${face(i.name, "neutral")}<span><b>${esc(first(i.name))}</b> ${esc(i.text)}</span></button>`).join("");
  return `<div class="hq"><div>
      <div class="founder"><span class="me">Y</span><div><b>You</b><span>Founder · ${s.employees.length} people · ${working} working now</span></div><button class="btn" data-add-dept="1">+ Department</button></div>
      <ul class="depts">${rooms}${add}</ul></div>
    <aside class="sidecol">
      ${items.length ? `<section class="panel"><h3>Needs you <span class="badge">${items.length}</span></h3><div class="needs">${needs}</div></section>` : ""}
      <section class="panel"><h3>Live <small class="muted">${s.serve ? "streaming" : "paused"}</small></h3><ul class="feed">${feed(s.feed)}</ul></section>
    </aside></div>`;
}
function feed(items) {
  if (!items.length) return `<li><span></span><span class="muted">Nothing yet. Give someone a ticket up top.</span></li>`;
  return items.map(f => `<li>${face(f.name, "neutral")}<div><b>${esc(first(f.name))}</b> <span class="key">${esc(f.key)}</span> ${esc(f.summary)}<time>${esc(String(f.at).slice(11, 16))}</time></div></li>`).join("");
}

/* Inbox: everything that needs you, answerable in one click */
function viewInbox() {
  const s = S.snap;
  const blocking = s.inbox.filter(q => q.kind !== "handbook" && q.blocking), other = s.inbox.filter(q => q.kind !== "handbook" && !q.blocking), books = s.inbox.filter(q => q.kind === "handbook");
  const cards = [...blocking.map(question), ...s.reviews.map(review), ...other.map(question), ...books.map(handbook)];
  if (!cards.length) return `<div class="empty-state"><b>Inbox zero</b>Your team puts questions, reviews and handbook ideas here when they need you.</div>`;
  return `<div class="inbox">${cards.join("")}</div>`;
}
function question(q) {
  const opts = q.options.map(o => `<button class="btn primary" data-answer="${q.id}" data-val="${esc(o)}">${esc(o)}</button>`).join("");
  const def = q.default && !q.options.includes(q.default) ? `<button class="btn" data-answer="${q.id}" data-val="${esc(q.default)}">Go with ${esc(q.default)}</button>` : "";
  return `<article class="ask ${q.blocking ? "blocking" : ""}">${face(q.name, q.blocking ? "blocked" : "neutral")}<div>
    <span class="kind">${q.blocking ? "Blocking" : "Question"} · ${esc(q.key || "")}</span><h4>${esc(q.question)}</h4>
    <p>${esc(first(q.name))} ${q.blocking ? "stopped until you answer." : q.default ? `went ahead with “${esc(q.default)}” for now.` : "is carrying on meanwhile."}</p>
    <div class="acts">${opts}${def}</div>
    <form class="reply" data-reply="${q.id}"><input id="ans-${q.id}" placeholder="Or write your own answer…"><button class="btn">Send</button></form></div></article>`;
}
function review(r) {
  return `<article class="ask review">${face(r.name, "done")}<div>
    <span class="kind">Review · ${esc(r.key)} · ${esc(r.spend)}</span><h4>${esc(r.title)}</h4><p>${esc(r.summary || `${first(r.name)} finished and is waiting for your review.`)}</p>
    <div class="acts"><button class="btn good" data-approve="${esc(r.key)}">Approve and merge</button></div>
    <form class="reply" data-changes="${esc(r.key)}"><input id="chg-${esc(r.key)}" placeholder="Or ask for changes…"><button class="btn">Send back</button></form></div></article>`;
}
function handbook(q) {
  return `<article class="ask">${face(q.name, "neutral")}<div>
    <span class="kind">Handbook · ${esc(q.section)}</span><h4>${esc(q.question)}</h4><p>${esc(first(q.name))} thinks everyone should follow this from now on.</p>
    <div class="acts"><button class="btn primary" data-answer="${q.id}" data-val="yes">Add to the handbook</button><button class="btn" data-answer="${q.id}" data-val="no">Not for everyone</button></div>
    <form class="reply" data-reply="${q.id}"><input id="ans-${q.id}" placeholder="Or add it in your own words…"><button class="btn">Add</button></form></div></article>`;
}

/* Drawers: a person, or hiring into a department */
function renderDrawer() {
  const d = $("#drawer"), open = !!S.drawer;
  d.classList.toggle("open", open); $("#scrim").classList.toggle("open", open); d.setAttribute("aria-hidden", String(!open));
  if (!open) return;
  if (S.drawer.type === "hire") return (d.innerHTML = drawerHire());
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
      <div><h5>${p.current ? `Now · ${esc(p.current.key)} · ${esc(p.current.title)}` : "Now"}</h5>${activity ? `<div class="term" id="term">${activity}</div>` : `<p class="muted" style="margin:0">Free. Give ${esc(first(p.name))} a ticket from the top bar.</p>`}</div>
      <div><h5>What ${esc(first(p.name))} has learned</h5>${p.memory.length ? `<ul class="notes">${p.memory.map(m => `<li>${esc(m)}</li>`).join("")}</ul>` : `<p class="muted" style="margin:0">Nothing yet. Notes build up as they work and as you review them.</p>`}</div>
      <div><h5>Tickets</h5>${p.tickets.length ? `<ul class="tlist">${p.tickets.map(t => `<li><span class="k">${esc(t.key)}</span>${esc(t.title)}<span class="s chip">${esc(t.status)}</span></li>`).join("")}</ul>` : `<p class="muted" style="margin:0">None yet.</p>`}</div>
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

/* Rendering and polling */
function render() {
  renderSide(); renderTop();
  const v = $("#view");
  if (!S.company) {
    v.innerHTML = `<div class="empty-state"><b>No company yet</b>Create one from a terminal for now:<br><br><code>onecorp new snowztech --repo ~/dev/your-repo</code><br><br>It shows up here right away.</div>`;
  } else if (!S.snap) {
    v.innerHTML = `<p class="muted">Opening ${esc(S.company)}…</p>`;
  } else {
    v.innerHTML = S.view === "inbox" ? viewInbox() : viewCompany();
  }
  renderDrawer();
}
function typing() {
  const a = document.activeElement;
  return a && a.matches("input, textarea") && a.closest("#view, #drawer") && a.value.trim() !== "";
}
async function refresh(force) {
  try {
    const c = await invoke("companies");
    S.companies = c.list;
    if (!S.company || !c.list.includes(S.company)) S.company = c.current;
    if (!S.company) { S.snap = null; return render(); }
    const snap = await invoke("snapshot", { company: S.company });
    const json = JSON.stringify(snap);
    if (!force && (json === S.last || typing())) return;
    S.snap = snap; S.last = json;
    if (S.drawer?.type === "person") S.person = await invoke("person", { company: S.company, id: S.drawer.id }).catch(() => S.person);
    render();
  } catch (e) {
    toast(String(e), true);
  }
}

/* Actions */
document.addEventListener("click", async e => {
  if (e.target.id === "scrim") { S.drawer = null; return renderDrawer(); }
  const el = e.target.closest("[data-co],[data-view],[data-person],[data-hire],[data-add-dept],[data-close],[data-answer],[data-approve],[data-pick]");
  if (!el) return;
  const d = el.dataset;
  if (d.co) { S.company = d.co; S.snap = null; S.last = ""; render(); await call("use_company", { name: d.co }); }
  else if (d.view) { S.view = d.view; render(); }
  else if (d.person) openPerson(d.person);
  else if (d.hire) { const dept = S.snap.departments.find(x => String(x.id) === d.hire); S.drawer = { type: "hire", dept: dept.id, role: /eng/i.test(dept.name) ? "Engineer" : dept.name + " specialist" }; renderDrawer(); }
  else if (d.addDept) { S.addingDept = true; S.view = "company"; render(); $("#dept-name")?.focus(); }
  else if (d.close) { S.drawer = null; renderDrawer(); }
  else if (d.answer) { el.disabled = true; await call("answer", { company: S.company, id: Number(d.answer), text: d.val }, d.val === "no" ? "Left out of the handbook" : "Answered").catch(() => (el.disabled = false)); }
  else if (d.approve) { el.disabled = true; el.textContent = "Merging…"; await call("approve", { company: S.company, key: d.approve }, `${d.approve} merged. New tickets start from it.`).catch(() => { el.disabled = false; el.textContent = "Approve and merge"; }); }
  else if (d.pick) {
    const role = $("#role").value.trim() || S.drawer.role, c = candidates(role)[Number(d.pick)];
    const bio = BIOS.find(([re]) => re.test(role))[1];
    el.disabled = true;
    try {
      await call("hire", { company: S.company, name: c.name, role, bio, runtime: c.runtime, model: c.model || null, department: S.drawer.dept }, `${c.name} joined. Give them a ticket up top.`);
      S.drawer = null; renderDrawer();
    } catch { el.disabled = false; }
  }
});
document.addEventListener("submit", async e => {
  e.preventDefault();
  const f = e.target;
  if (f.id === "give") {
    const title = $("#give-task").value.trim(), employee = $("#give-to").value;
    if (!title) return $("#give-task").focus();
    await call("new_ticket", { company: S.company, title, employee }, `Given to ${first(byId(employee)?.name)}`).then(() => ($("#give-task").value = "")).catch(() => {});
  } else if (f.id === "dept-form") {
    const name = $("#dept-name").value.trim();
    if (!name) return $("#dept-name").focus();
    const id = await call("add_department", { company: S.company, name }, `${name} is set up. Hire its first person.`).catch(() => null);
    if (id !== null) { S.addingDept = false; S.drawer = { type: "hire", dept: id, role: /eng/i.test(name) ? "Engineer" : name + " specialist" }; render(); }
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
document.addEventListener("change", async e => {
  if (e.target.id === "move" && e.target.value) {
    await call("move_to_department", { company: S.company, employee: e.target.dataset.move, department: Number(e.target.value) }, "Moved").catch(() => {});
    if (S.drawer?.type === "person") openPerson(S.drawer.id);
  }
});
document.addEventListener("keydown", e => {
  if (e.key === "Escape") { if (S.drawer) { S.drawer = null; renderDrawer(); } else if (S.addingDept) { S.addingDept = false; render(); } }
  if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") { e.preventDefault(); $("#give-task").focus(); }
});

refresh(true);
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
      company: "snowztech", serve: true, employees: team, tickets: [],
      departments: [{ id: 1, name: "Engineering", lead: "odile", members: ["odile", "maya", "tomas"] }, { id: 2, name: "Product", lead: "rui", members: ["rui", "lina"] }, { id: 3, name: "Growth", lead: "sam", members: ["sam"] }],
      inbox: [{ id: 2, kind: "question", key: "SNO-12", employee: "maya", name: "Maya Okafor", question: "Magic links: 15 or 60 minute expiry?", section: "", options: ["15 minutes", "60 minutes"], default: "15 minutes", blocking: true },
        { id: 3, kind: "handbook", key: "SNO-14", employee: "lina", name: "Lina Park", question: "Empty states are one sentence and one button", section: "Conventions", options: [], default: "", blocking: false }],
      reviews: [{ key: "SNO-11", title: "Pricing page", employee: "odile", name: "Odile Martin", summary: "Added the pricing page with Free and Pro plans. Check the copy on mobile.", spend: "$0.31" }],
      feed: [{ at: "2026-11-09 11:02:00", key: "SNO-11", employee: "odile", name: "Odile Martin", kind: "done", summary: "finished, $0.31" }, { at: "2026-11-09 10:52:00", key: "SNO-15", employee: "tomas", name: "Tomás Reis", kind: "tool", summary: "Bash cargo test billing" }],
      spend: { cost: 3.24, tokens: 180000, label: "$3.24" },
    },
    person: { id: "tomas", name: "Tomás Reis", role: "Engineer", runtime: "codex", model: null, bio: "Fast, likes refactors a bit too much.", department: 1, current: { key: "SNO-15", title: "Retry failed Stripe webhooks", status: "doing" },
      activity: [{ at: "2026-11-09 10:50:00", kind: "tool", summary: "Edit src/billing/webhook.rs" }, { at: "2026-11-09 10:52:00", kind: "tool", summary: "Bash cargo test billing" }], tickets: [{ key: "SNO-15", title: "Retry failed Stripe webhooks", status: "doing", spend: "75k tok" }],
      memory: ["2026-11-09 Stripe retries for 3 days, make handlers idempotent instead of queueing"] },
  };
  if (data[cmd]) return Promise.resolve(data[cmd]);
  return Promise.reject("This is a preview. Open the OneCorp app to make changes.");
}
