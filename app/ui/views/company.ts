import { esc, first, hhmm } from "../lib.ts";
import { RUNTIME, agentBadge, face, icon, me, modelName } from "../look.ts";
import { needItems } from "../notify.ts";
import { S, byId, employeesOf, myName } from "../state.ts";
import type { Employee, FeedItem } from "../types.ts";

/* Company page: you on top, departments beneath as rooms of desks */
export function desk(e: Employee, lead: boolean) {
  const show = e.bubble ? "show" : "";
  const works = (RUNTIME[e.runtime]?.[0] || e.runtime) + (RUNTIME[e.runtime] ? ` · ${modelName(e.runtime, e.model)}` : "");
  return `<button class="desk ${e.status}" data-person="${esc(e.id)}" title="${esc(e.name)} works on ${esc(works)}">
    <span class="bubble ${show}">${esc(e.bubble)}</span>
    <span class="station"><span class="seat with-agent">${face(e.name, e.status)}${agentBadge(e.runtime)}</span><span class="monitor"><i></i><i></i><i></i><i></i></span></span>
    <span class="table"></span>${e.status === "idle" ? '<span class="zz">z</span>' : ""}
    <span class="plate"><b>${esc(first(e.name))}${lead ? '<em class="leadtag">Lead</em>' : ""}</b><span>${esc(e.role)}</span></span></button>`;
}

export function viewCompany() {
  const s = S.snap!;
  const rooms = s.departments.map(d => {
    const ms = employeesOf(d.members), lead = d.lead && byId(d.lead);
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
  const needs = items.map(i => `<button class="need" data-goto="${esc(i.to)}">${face(i.name, "neutral")}<span><b>${esc(first(i.name))}</b> ${esc(i.text)}</span></button>`).join("");
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

/* Kickoff: from what you're building to a plan of tickets */
export function kickoffCard() {
  const k = S.snap!.kickoff;
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
export function gettingStarted() {
  // Only real work counts: planning and the wiki your lead writes at launch are not "work" yet.
  const s = S.snap!, work = s.tickets.filter(t => !t.kind || t.kind === "work");
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

export function feed(items: FeedItem[]) {
  if (!items.length) return `<li class="none">Nothing yet. Start a ticket and it shows up here.</li>`;
  return items.map(f => `<li><button class="fl" data-ticket="${esc(f.key)}">${face(f.name, "neutral")}<div><p><b>${esc(first(f.name))}</b> ${esc(f.summary)}</p><p class="ft"><span class="key">${esc(f.key)}</span> ${esc(f.title || "")}</p></div><time>${hhmm(f.at)}</time></button></li>`).join("");
}
