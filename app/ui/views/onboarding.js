import { logo } from "../brand.js";
import { $, esc, toast } from "../lib.js";
import { AGENTS, PALETTE, agentBadge, agentMark, badge, face, icon, me, monogram } from "../look.js";
import { render } from "../render.js";
import { S, invoke, version } from "../state.js";
import { fixRow } from "./settings.js";

/* Your profile: a name and a photo, kept on this Mac. No account. The first launch asks full screen,
   after that it's a small popover from your name in the sidebar. */
// Onboarding keeps a picked photo aside until you continue; afterwards a photo saves right away.
export const hasPhoto = () => !!(S.profile?.photoUrl || (!S.profile && S.founder?.photo) || (S.profile && S.founder?.photo && !S.profile.noPhoto));

export const photoActs = () => `${window.__TAURI__?.dialog ? `<button class="link-btn" type="button" data-pf="pick">${hasPhoto() ? "Change photo" : "Add a photo"}</button>` : ""}
  ${hasPhoto() ? `<button class="link-btn" type="button" data-pf="face">Remove photo</button>` : ""}`.trim();

export function pfPic(name) {
  if (S.profile?.photoUrl) return `<span class="av"><img src="${esc(S.profile.photoUrl)}" alt=""></span>`;
  if (S.founder?.photo && !S.profile?.noPhoto) return me();
  return monogram(name);
}

export function focusTop(cancel) {
  return `<div class="focus-top" data-tauri-drag-region><span class="brand">${logo()}OneCorp</span>${cancel ? `<button class="btn ghost" type="button" data-su="cancel">Cancel</button>` : ""}</div>`;
}

// The first thing a new founder sees: what OneCorp is, in a line, and one way forward.
export function viewWelcome() {
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

export function viewProfile() {
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

export async function saveProfile(photo, removePhoto) {
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
export async function startSetup() {
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
export const TIERS = [["strongest", "Strongest"], ["balanced", "Balanced"], ["cheapest", "Cheapest"]];

// A copied person keeps their model while it matches one of the tiers, so the picker shows it.
export const tierOf = (runtime, model) => TIERS.find(([t]) => S.models?.tiers?.[runtime]?.[t]?.id === model)?.[0];

export function setTemplate(key) {
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

export function readSetup() {
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

export async function checkRepo(path) {
  const s = S.setup;
  s.path = path; s.checked = path; s.repo = null; render();
  if (!path) return;
  try { s.repo = await invoke("check_repo", { path }); } catch (e) { s.repo = { ok: false, problem: String(e) }; }
  if (s.repo.ok && !s.name) s.name = s.repo.suggested;
  if (s.repo.ok && !s.folder) s.folder = s.name;
  render();
}

export async function makeRepo() {
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

export const RUNTIME_NAMES = { claude: "Claude Code", codex: "Codex", fake: "Stand-in, no AI" };

export function shipLine(r) {
  if (!r.forge) return "Approved work merges into the repo on your Mac.";
  if (r.signed_in) return `Finished work opens ${esc(r.pr_word)}s on ${esc(r.forge)}, and approving merges them.`;
  return `On ${esc(r.forge)}, but ${r.forge === "GitHub" ? "gh" : "glab"} isn't signed in, so work merges on your Mac for now.`;
}

export function shipNote(r) {
  if (!r.forge) return `<p class="su-warn">Approved work merges into this repo on your Mac. Add a GitHub or GitLab remote later to ship as pull requests.</p>`;
  if (r.signed_in) return `<p class="su-ok">Finished work opens ${esc(r.pr_word)}s on ${esc(r.forge)}, and approving merges them. ${esc(r.account || "")}</p>`;
  const cli = r.forge === "GitHub" ? "gh" : "glab";
  return `<p class="su-warn">This repo is on ${esc(r.forge)}, but ${cli} isn't signed in, so work merges locally for now. Run <code>${cli} auth login</code>, then <code>onecorp ship ${r.forge.toLowerCase()}</code>.</p>`;
}

// The whole journey, shown on every step. A first launch also counts you.
// Back to the You step from setup: your name and photo as they are, and setup waits where it was.
export function backToYou() {
  S.profile = { name: S.founder?.name || "", photoUrl: S.founder?.photo || null, photo: null, inSetup: true };
  render();
  $("#pf-name")?.focus();
}

// On the Connect step, tools are checked again when you come back from Terminal and every few
// seconds while one isn't connected, so a login shows up without a button. Only a changed
// result redraws, and the step has no fields to lose.
export let rechecking = false, toolTimer = null;

export const onConnect = () => S.setup && S.setup.step === 0 && !S.setup.launch && !S.profile;

export const toolMissing = r => (r || []).some(h => ["Claude Code", "Codex", "GitHub CLI"].includes(h.what) && h.level !== "ok");

export async function recheckTools() {
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

export function journey(at) {
  const steps = [...(S.companies.length && !S.onboarding && at !== "You" ? [] : ["You"]), "Connect", "Company", "Repo", "Team", "Launch"], i = steps.indexOf(at);
  // Steps you've done are links back, so you can change anything on the way.
  const item = (n, k) => `<span class="n">${k < i ? icon("check") : k + 1}</span>${n}`;
  return `<ol class="onb-steps">${steps.map((n, k) => `<li class="${k === i ? "on" : k < i ? "done" : ""}">${k < i && S.setup && !S.setup.launch ? `<button type="button" data-step-to="${n}">${item(n, k)}</button>` : item(n, k)}</li>`).join("")}</ol>`;
}

// Setup screens: the steps on the left, the step itself on the right, over a faint grid.
export const onboarding = (at, cancel, form) => `<div class="focus-wrap">${focusTop(cancel)}<div class="onb"><aside>${journey(at)}</aside>${form}</div></div>`;

export function viewLaunching() {
  const s = S.setup, l = s.launch;
  return `<div class="focus-wrap"><div class="launching">
      ${badge(s.name, { color: s.color, logo: s.logoUrl }, "xl")}
      <h1>Setting up ${esc(s.name)}</h1>
      <ol>${l.steps.map((t, i) => `<li class="${i < l.done ? "done" : i === l.done ? "now" : ""}"><span class="n">${i < l.done ? icon("check") : i === l.done ? '<i class="spin"></i>' : ""}</span>${esc(t)}</li>`).join("")}</ol>
    </div></div>`;
}

// Your GitHub repos, looked up once in the background when the Repo step opens, so the list is
// ready by the time you ask for it.
export function loadGithub() {
  const s = S.setup;
  if (!s || s.gh || s.ghLoading) return;
  s.ghLoading = true;
  invoke("github_repos").then(repos => ({ repos })).catch(e => ({ error: String(e) })).then(g => {
    s.ghLoading = false; s.gh = g;
    if (S.setup === s && s.mode === "github") render();
  });
}

// A path you change by clicking it. The app opens the folder picker, the browser preview types.
export const pathField = (id, value, action, label) => window.__TAURI__?.dialog
  ? `<button type="button" class="path-field" data-su="${action}" aria-label="${esc(label)}" title="Click to pick another folder"><span class="mono">${esc(value)}</span><span class="path-hint">Change</span></button>`
  : `<input id="${id}" class="mono path-field" value="${esc(value)}" autocomplete="off" spellcheck="false" aria-label="${esc(label)}">`;

// The first repo you add is the main one. Every one after joins it.
export async function addRepo(path) {
  path = String(path || "").trim();
  if (!path) return $("#su-add")?.focus();
  if (!S.setup.repo?.ok) return checkRepo(path);
  return addExtraRepo(path);
}

export async function addExtraRepo(path) {
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

export function viewSetup() {
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
