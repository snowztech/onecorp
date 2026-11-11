import { $, esc, first, hash, shortPaths, toast, utc } from "./lib.ts";
import { AGENTS, agentBadge, agentMark, chip, drawFace, face, modelsOf, prLink } from "./look.ts";
import { inline } from "./md.ts";
import { S, byId, call, invoke } from "./state.ts";
import type { LogLine, PersonForm } from "./types.ts";
import { changes, replyForm, discardBtn, discardConfirm, more, reviewActions, ticketStatus, wikiDraft } from "./views/inbox.ts";

// The panel that slides in from the right: a ticket, a person, hiring, a new ticket or a project.
/* Plan a project: like an epic, except your planner breaks it down and you approve it */
export function drawerPlan() {
  const s = S.snap!, team = s.employees, isFirst = !(s.projects || []).length, k = s.kickoff;
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

// One run log line: time, what kind, and the whole text, wrapped under itself. A tool line shows
// its tool dimmed, so the command stands out.
export function logRow(a: LogLine, from: number) {
  const [tool, ...rest] = a.kind === "tool" ? shortPaths(a.summary).split(" ") : [];
  const text = a.kind === "tool" ? `<span class="tname">${esc(tool)}</span> ${esc(rest.join(" "))}`
    : a.kind === "say" ? String(a.summary).split("\n").map(inline).join("\n") : esc(shortPaths(a.summary));
  return `<div class="lr k-${esc(a.kind)}"><span class="at">${esc(String(a.at).slice(from, 16))}</span><span class="kd">${esc(a.kind)}</span><span class="tx">${text}</span></div>`;
}

// A command still going says so, with how long, so a long build never looks like a stuck run.
export function stillRunning(rows: LogLine[], doing: boolean) {
  const last = rows.at(-1);
  if (!doing || last?.kind !== "tool") return "";
  const min = Math.floor((Date.now() - utc(last.at).getTime()) / 60000);
  return `<div class="lr running"><span class="at"></span><span class="kd"></span><span class="tx"><i class="spin"></i>${min >= 60 ? `still running, ${Math.floor(min / 60)} h ${min % 60} min` : min >= 1 ? `still running, ${min} min` : "running…"}</span></div>`;
}

export function drawerTicket() {
  const t = S.ticket;
  if (!t) return `<div class="dhead"><h2>Loading</h2><button class="close" data-close="1" aria-label="Close">×</button></div>`;
  const people = (S.snap?.employees || []).map(e => `<option value="${esc(e.id)}" ${e.id === t.employee ? "selected" : ""}>${esc(e.name)}</option>`).join("");
  const acts = ({
    backlog: `<select id="t-who" data-assign="${esc(t.key)}" aria-label="Who does it"><option value="">Nobody yet</option>${people}</select><button class="btn primary" data-start="${esc(t.key)}">Start</button>`,
    waiting: `<button class="btn" data-stop="${esc(t.key)}">Back to backlog</button>`,
    doing: `<button class="btn" data-stop="${esc(t.key)}">Stop</button>`,
    queued: `<button class="btn" data-stop="${esc(t.key)}">Cancel</button>`,
    failed: `<button class="btn primary" data-retry="${esc(t.key)}">Retry</button>`,
    blocked: `<button class="btn primary" data-view="inbox" data-close="1">Answer it in the Inbox</button>`,
    review: reviewActions(t) + discardBtn(t),
    done: t.kind === "wiki" ? `<button class="btn" data-view="wiki" data-close="1">Read wiki</button>` : "",
  } as Record<string, string>)[t.status] || "";
  const back = t.status === "review" ? replyForm(`chg-${t.key}`, `data-changes="${esc(t.key)}"`, "Ask for changes…", "Send back", false) : "";
  const log = t.log.map(a => logRow(a, 5)).join("") + stillRunning(t.log, t.status === "doing");
  return `<div class="dhead">${face(t.name, t.status === "doing" ? "working" : "neutral")}<div><span class="mono muted">${esc(t.key)} · ${esc(ticketStatus(t))}</span><h2>${esc(t.title)}</h2></div><button class="close" data-close="1" aria-label="Close">×</button></div>
    <div class="dbody">
      ${t.description ? `<p class="details">${esc(t.description)}</p>` : ""}
      <dl class="kv"><dt>Who</dt><dd>${t.employee ? `<a class="link" data-person="${esc(t.employee)}">${esc(t.name)}</a>` : "Nobody yet"}</dd><dt>Spent</dt><dd>${esc(t.spend)}</dd>
        ${t.created_by ? `<dt>Created by</dt><dd>${esc(t.created_by)}</dd>` : ""}
        ${t.after ? `<dt>After</dt><dd><a class="link" data-ticket="${esc(t.after)}">${esc(t.after)}</a></dd>` : ""}
        ${t.pr_url ? `<dt>Pull request</dt><dd>${prLink(t.pr_url, t.pr_number)}</dd>` : ""}
        ${t.repo ? `<dt>Repo</dt><dd class="mono">${esc(t.repo)}</dd>` : ""}
        ${t.branch ? `<dt>Branch</dt><dd class="mono">${esc(t.branch)}${t.base ? ` <span class="muted">from ${esc(t.base)}</span>` : ""}</dd>` : ""}</dl>
      ${wikiDraft(t)}${acts ? `<div class="acts">${acts}</div>` : ""}${t.status === "review" ? discardConfirm(t) : ""}${back}${changes(t.key)}
      ${more(`log-${t.key}`, "Everything that happened", log ? `<div class="term tall" id="term">${log}</div>` : `<p class="muted" style="margin:0">Nothing yet.</p>`)}
      ${["backlog", "todo", "waiting", "done", "failed"].includes(t.status) ? (S.confirm === "ticket:" + t.key
        ? `<div class="confirm"><p>Delete ${esc(t.key)}? Its log goes with it.${t.status === "done" ? (t.kind === "wiki" ? " The wiki stays saved." : t.kind === "plan" ? " Its work tickets stay." : " The merged work stays in your repo.") : t.status === "failed" ? " Its unfinished branch is thrown away." : ""}</p>
            <div class="acts"><button class="btn danger" data-remove="${esc(t.key)}">Delete</button><button class="btn ghost" data-confirm="">Cancel</button></div></div>`
        : `<div class="dfoot"><button class="link-btn danger-link" data-confirm="ticket:${esc(t.key)}">Delete ticket…</button></div>`) : ""}
    </div>`;
}

/* New ticket: details, who, what it waits for, and start now or later */
export function drawerNewTicket() {
  const open = (S.snap?.tickets || []).filter(t => t.status !== "done"), team = S.snap?.employees || [];
  const pick = S.drawer?.who || (team.find(e => e.status === "idle") || team[0])?.id;
  return `<div class="dhead"><div><h2>New ticket</h2><span class="muted">Plan it now, start it now or later.</span></div><button class="close" data-close="1" aria-label="Close">×</button></div>
    <form class="dbody" id="nt">
      <div class="field"><label for="nt-title">What needs doing</label><input id="nt-title" placeholder="Add magic link login" autocomplete="off"></div>
      <div class="field"><label for="nt-who">Who</label><select id="nt-who"><option value="">Nobody yet</option>${team.map(e => `<option value="${esc(e.id)}" ${e.id === pick ? "selected" : ""}>${esc(e.name)}, ${esc(e.role)}${e.status === "idle" ? " (free)" : ""}</option>`).join("")}</select></div>
      ${(S.snap?.repos || []).length > 1 ? `<div class="field"><label for="nt-repo">Repo</label><select id="nt-repo">${S.snap!.repos.map(r => `<option value="${esc(r.name)}">${esc(r.name)}${r.main ? " (primary)" : ""}</option>`).join("")}</select></div>` : ""}
      <div class="field"><label for="nt-desc">Details <span class="muted">(optional)</span></label><textarea id="nt-desc" rows="5" placeholder="What done looks like, links, constraints"></textarea></div>
      ${(S.snap?.projects || []).length ? `<div class="field"><label for="nt-project">Project <span class="muted">(optional)</span></label><select id="nt-project"><option value="">None</option>${S.snap!.projects.map(p => `<option value="${esc(p.key)}" ${p.key === S.project ? "selected" : ""}>${esc(p.title)}</option>`).join("")}</select></div>` : ""}
      <div class="field"><label for="nt-after">After</label><select id="nt-after"><option value="">Nothing, it can start right away</option>${open.map(t => `<option value="${esc(t.key)}">${esc(t.key)} ${esc(t.title)}</option>`).join("")}</select></div>
      <div class="acts"><button class="btn primary" data-nt="now">Start now</button><button class="btn" data-nt="later">Save for later</button></div>
    </form>`;
}

export async function saveNewTicket(later: boolean) {
  const title = $("#nt-title").value.trim();
  if (!title) return $("#nt-title").focus();
  const args = { company: S.company, title, employee: $("#nt-who").value || null, description: $("#nt-desc").value, after: $("#nt-after").value || null, project: $("#nt-project")?.value || null, repo: $("#nt-repo")?.value || null, later };
  const key = await call("new_ticket", args, later ? "Saved to the backlog" : "Started").catch(() => null);
  if (key) { S.view = "board"; openTicket(key); }
}

export async function openTicket(key: string) {
  S.drawer = { type: "ticket", key }; S.ticket = null; S.confirm = null; renderDrawer();
  try { S.ticket = await invoke("ticket_detail", { company: S.company, key }); } catch (e) { toast(String(e), true); }
  renderDrawer();
}

/* Drawers: a person, or hiring into a department */
export function renderDrawer() {
  const d = $("#drawer"), open = !!S.drawer;
  d.classList.toggle("open", open); $("#scrim").classList.toggle("open", open); d.setAttribute("aria-hidden", String(!open));
  if (!open || !S.drawer) return;
  if (S.drawer.type === "hire") return (d.innerHTML = drawerHire());
  if (S.drawer.type === "new-ticket") return (d.innerHTML = drawerNewTicket());
  if (S.drawer.type === "plan") return (d.innerHTML = drawerPlan());
  if (S.drawer.type === "ticket") { d.innerHTML = drawerTicket(); const term = $("#term"); if (term) term.scrollTop = term.scrollHeight; return; }
  const p = S.person;
  if (!p) return (d.innerHTML = `<div class="dhead"><h2>Loading</h2><button class="close" data-close="1" aria-label="Close">×</button></div>`);
  const e = byId(p.id) || { status: "idle" };
  const depts = S.snap!.departments.filter(x => x.id !== null), dept = depts.find(x => x.id === p.department);
  const head = `<div class="dhead"><span class="with-agent">${face(p.name, e.status)}${agentBadge(p.runtime)}</span><div><h2>${esc(p.name)}</h2><span class="muted">${esc(p.role)}${dept ? ` · ${esc(dept.name)}` : ""}</span></div><button class="close" data-close="1" aria-label="Close">×</button></div>`;
  if (S.drawer.edit && S.drawer.form?.photoUrl) {
    d.innerHTML = head.replace(/<span class="with-agent">[\s\S]*?<\/span>(?=<div>)/, `<span class="with-agent"><span class="av"><img src="${esc(S.drawer.form.photoUrl)}" alt=""></span>${agentBadge(p.runtime)}</span>`);
  }
  if (S.drawer.edit) {
    d.innerHTML = `${head}<form class="dbody" id="emp-form">
      ${avatarPicker(S.drawer.form ||= { name: p.name, role: p.role, runtime: p.runtime, model: p.model, bio: p.bio, voice: p.voice, face: byId(p.id)?.face || p.name, photoUrl: null, photoPath: null, removePhoto: false, hasPhoto: !!byId(p.id)?.photo })}
      ${personFields(S.drawer.form!)}
      <div class="field"><label for="pf-cap">Daily cap <span class="muted">(optional)</span></label><div class="money-in"><span>$</span><input id="pf-cap" type="number" min="0" step="0.5" value="${(byId(p.id)?.cap ?? 0) > 0 ? byId(p.id)!.cap : ""}" placeholder="No cap"><span class="unit">a day. They stop picking up work when they reach it</span></div></div>
      <div class="acts"><button class="btn primary">Save</button><button class="btn ghost" type="button" data-edit-person="">Cancel</button></div></form>`;
    return;
  }
  const leads = dept?.lead === p.id;
  const activity = p.activity.map(a => logRow(a, 11)).join("") + stillRunning(p.activity, p.current?.status === "doing");
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
      <div><h5>Tickets</h5>${p.tickets.length ? `<ul class="tlist">${p.tickets.map(t => `<li><button class="tl" data-ticket="${esc(t.key)}"><span class="k">${esc(t.key)}</span>${esc(t.title)}<span class="s chip">${esc(ticketStatus(t))}</span></button></li>`).join("")}</ul>` : `<p class="muted" style="margin:0">None yet.</p>`}</div>
      ${letGo}
    </div>`;
  const term = $("#term"); if (term) term.scrollTop = term.scrollHeight;
}

// Candidates differ in how they work, not just in name: cheap and fast, senior, or one that can build and test.
export const NAMES = ["Noor Haddad", "Jonas Berg", "Inês Costa", "Kwame Mensah", "Priya Raman", "Léa Moreau", "Mateo Silva", "Hana Sato", "Oskar Lind", "Amara Obi", "Rafael Duarte", "Mei Lin"];

export function candidates(role: string) {
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

export function agentPicker(f: PersonForm) {
  const have = S.runtimes || Object.keys(AGENTS), a = AGENTS[f.runtime] || AGENTS.claude;
  // The exact models each agent offers, from the agent itself where it says. An older short name
  // like "opus" still lights up the model it stands for.
  const live = modelsOf(f.runtime);
  const def = S.models?.defaults?.[f.runtime];
  const list = live?.length ? [["", def ? `Default · ${def}` : "Default", "Your own setting for this agent"], ...live.map(m => [m.id, m.name, m.about])] : a.models;
  const same = (v: string) => v === (f.model || "") || (f.model && v && !f.model.includes("-") && v.includes(f.model));
  const models = list.some(([v]) => same(v)) ? list : [...list, [f.model || "", f.model || ""]];
  return `<div class="field"><span class="flabel">Works on</span><div class="agents">${Object.entries(AGENTS).filter(([k]) => k !== "fake" || f.runtime === "fake").map(([k, x]) => {
      const ok = have.includes(k) || k === f.runtime;
      return `<button type="button" class="agent" data-agent="${k}" aria-pressed="${k === f.runtime}" ${ok ? "" : 'disabled title="Not installed on this Mac. Settings shows how to install it"'}>
        <span class="glyph" style="--c:${x.color}">${agentMark(k)}</span><span><b>${x.name}</b><small>${ok ? x.by : "Not installed"}</small></span></button>`;
    }).join("")}</div><input type="hidden" id="pf-rt" value="${esc(f.runtime)}"></div>
    <div class="field"><span class="flabel">Model</span><div class="models">${models.map(([v, n, about]) => `<button type="button" class="mchip" data-model="${esc(v)}" aria-pressed="${same(v)}" ${about ? `title="${esc(about)}"` : ""}>${esc(n)}</button>`).join("")}</div><input type="hidden" id="pf-m" value="${esc(f.model || "")}"></div>`;
}

// The fields for a person, shared by hiring and editing, so both look and behave the same.
export function avatarPicker(f: PersonForm) {
  // Variations lead with a number: a change at the start reshapes the whole face, at the end it barely moves.
  const base = f.name || "New hire", seeds = [base, ...[1, 2, 3, 4, 5, 6, 7].map(n => `${n} ${base}`)];
  const photo = f.photoUrl || (f.hasPhoto && !f.removePhoto);
  return `<div class="field"><span class="flabel">Avatar</span><div class="faces-pick">${seeds.map(sd => `<button type="button" class="fpick" data-face="${esc(sd)}" aria-pressed="${!photo && sd === (f.face || base)}" aria-label="Face ${esc(sd)}">${drawFace(sd, "neutral")}</button>`).join("")}</div>
    <div class="photo-acts">${window.__TAURI__?.dialog ? `<button class="link-btn" type="button" data-avatar-photo="1">${photo ? "Change photo" : "Use a photo instead"}</button>` : ""}${photo ? `<button class="link-btn" type="button" data-avatar-clear="1">Remove photo</button>` : ""}</div></div>`;
}

export function personFields(f: PersonForm) {
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
export function voicePicker(f: PersonForm) {
  const list = S.voices || [], auto = S.drawer?.type !== "person", cur = f.voice || (auto ? "" : list[0]?.key);
  const opts = [...(auto ? [{ key: "", label: "Fits role", sample: "Picked to fit their role. Change it any time." }] : []), ...list];
  const sample = opts.find(v => v.key === cur)?.sample || "";
  return `<div class="field"><span class="flabel">Personality</span><div class="models">${opts.map(v => `<button type="button" class="mchip vchip" data-vpick="${v.key}" aria-pressed="${v.key === cur}">${esc(v.label)}</button>`).join("")}</div>
    <div class="vsample"><span class="vs-who">${face(f.face || f.name || "New hire", "neutral")}<b>${esc(first(f.name) || "They")}</b><span>in chat</span></span><p id="pf-vs">${esc(sample)}</p></div>
    <input type="hidden" id="pf-v" value="${esc(cur || "")}"></div>`;
}

export const readPerson = () => ({ name: $("#pf-n").value.trim(), role: $("#pf-r").value.trim() || "Engineer", runtime: $("#pf-rt").value, model: $("#pf-m").value.trim() || null, bio: $("#pf-b").value.trim(), voice: $("#pf-v")?.value || null });

export function drawerHire() {
  const dept = S.snap!.departments.find(x => x.id === S.drawer!.dept), f = S.drawer!.form!;
  const picks = candidates(f.role || "Engineer");
  return `<div class="dhead"><span class="with-agent"><span class="hire-face">${face(f.name || "New hire", "neutral")}</span>${agentBadge(f.runtime)}</span><div><h2>Hire into ${esc(dept?.name || "the company")}</h2><span class="muted">Start from a suggestion or make your own. You can change anything later.</span></div><button class="close" data-close="1" aria-label="Close">×</button></div>
    <form class="dbody" id="hire-form">
      <div class="picks">${picks.map((c, i) => `<button type="button" class="pick" data-pick-fill="${i}">${face(c.name, "neutral")}<div><b>${esc(first(c.name))}</b><span>${esc(c.tier)}</span></div></button>`).join("")}</div>
      ${personFields(f)}
      <div class="acts"><button class="btn primary">Hire</button></div>
    </form>`;
}

export function openHire(deptId: number) {
  const dept = S.snap!.departments.find(x => x.id === deptId);
  const role = !dept || /eng/i.test(dept.name) ? "Engineer" : /design/i.test(dept.name) ? "Product designer" : /product/i.test(dept.name) ? "Product manager" : dept.name + " specialist";
  const c = candidates(role)[1] || candidates(role)[0];
  S.drawer = { type: "hire", dept: deptId, form: { name: c.name, role, runtime: c.runtime, model: c.model, bio: "" } };
  renderDrawer();
}

export async function openPerson(id: string) {
  S.drawer = { type: "person", id }; S.person = null; renderDrawer();
  try { S.person = await invoke("person", { company: S.company, id }); } catch (e) { toast(String(e), true); }
  renderDrawer();
}
