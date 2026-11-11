import { esc, first } from "../lib.js";
import { face, icon } from "../look.js";
import { S, byId } from "../state.js";

/* Board: every ticket by where it stands. Click one for its log and actions. */
export const COLUMNS = [["Backlog", ["backlog"]], ["Up next", ["todo", "queued", "waiting"]], ["Working", ["doing"]], ["Needs you", ["blocked", "review", "failed"]], ["Done", ["done"]]];

export const PILL = { blocked: ["Question", "warn"], review: ["Review", "info"], failed: ["Stopped", ""], waiting: ["Waiting", ""] };

export const spent = t => t.spend && !/^(\$0\.00|0 tok)$/.test(t.spend);

export function viewBoard() {
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

export const STATUS = { backlog: "In the backlog", waiting: "Waiting for another ticket", todo: "Up next", queued: "Up next", doing: "Working", blocked: "Waiting on you", review: "Ready for review", failed: "Stopped", done: "Merged" };
