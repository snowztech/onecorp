import { esc, first } from "../lib.ts";
import { face, icon, prLink } from "../look.ts";
import { md } from "../md.ts";
import { S, mergeLabel } from "../state.ts";
import type { Failed, Question, Review } from "../types.ts";
import { STATUS } from "./board.ts";

/* Inbox: everything that needs you, answerable in one click */
export function viewInbox() {
  const s = S.snap!;
  const asks = s.inbox.filter(q => q.kind === "question"), runs = s.inbox.filter(q => q.kind === "command");
  const blocking = asks.filter(q => q.blocking), other = asks.filter(q => !q.blocking), books = s.inbox.filter(q => q.kind === "wiki");
  const plans = s.reviews.filter(r => r.kind === "plan"), work = s.reviews.filter(r => r.kind !== "plan");
  const cards = [...plans.map(planReview), ...(s.failed || []).map(stopped), ...runs.map(commandAsk), ...blocking.map(q => question(q)), ...work.map(review), ...other.map(q => question(q)), ...books.map(wiki)];
  const leads = (s.with_leads || []).map(q => question(q, true));
  const zero = `<div class="empty-state calm"><span class="empty-ic">${icon("check")}</span><b>You're all caught up</b><span>Questions, plans and reviews from your team land here when they need you.</span></div>`;
  return `<div class="inbox">${cards.length ? cards.join("") : zero}
    ${leads.length ? `<h2 class="section">With your leads <small class="muted">They decide what they can and pass up the rest. Step in anytime.</small></h2>${leads.join("")}` : ""}</div>`;
}

export function stopped(t: Failed) {
  return `<article class="ask stopped" id="in-${esc(t.key)}">${face(t.name, "blocked")}<div>
    <span class="kind">Stopped · ${esc(t.key)}</span><h4>${esc(t.title)}</h4>
    <p>${t.limit ? `${esc(first(t.name))}'s run ${esc(t.why)}. Keep going, or look at what they did first.` : `${esc(first(t.name))}'s run ${t.why && !/^stopped before/.test(t.why) ? `failed: ${esc(t.why)}` : "stopped before it finished."}`}</p>
    ${S.confirm === "ticket:" + t.key
      ? `<div class="confirm"><p>Delete ${esc(t.key)}? Its log and unfinished branch go with it.</p>
          <div class="acts"><button class="btn danger" data-remove="${esc(t.key)}">Delete</button><button class="btn ghost" data-confirm="">Cancel</button></div></div>`
      : `<div class="acts"><button class="btn primary" data-retry="${esc(t.key)}">${t.limit ? "Keep going" : "Retry"}</button>${t.limit ? `<button class="btn" data-ticket="${esc(t.key)}">See what they did</button>` : ""}${S.open[`rty-${t.key}`] ? "" : `<button class="btn ghost" type="button" data-more="rty-${esc(t.key)}">${t.limit ? "Keep going with a note…" : "Retry with a note…"}</button>`}<button class="btn ghost" data-confirm="ticket:${esc(t.key)}">Delete…</button></div>`}
    ${S.open[`rty-${t.key}`] ? replyForm(`rty-${t.key}`, `data-retry-note="${esc(t.key)}"`, "What should they do differently?", "Retry") : ""}</div></article>`;
}

export function question(q: Question, withLead?: boolean) {
  const opts = q.options.map(o => `<button class="btn primary" data-answer="${q.id}" data-val="${esc(o)}">${esc(o)}</button>`).join("");
  // A long default goes in the sentence above, so its button stays short.
  const def = q.default && !q.options.includes(q.default) ? `<button class="btn" data-answer="${q.id}" data-val="${esc(q.default)}">${q.default.length > 40 ? `Keep ${esc(first(q.name))}'s choice` : `Go with ${esc(q.default)}`}</button>` : "";
  const kind = withLead ? `With ${esc(first(q.lead_name || "their lead"))}` : q.blocking ? "Blocking" : "Question";
  const via = q.via_name ? `<p class="via"><b>${esc(first(q.via_name))}</b> passed this to you${q.note ? `: ${esc(q.note)}` : "."}</p>` : "";
  return `<article class="ask ${q.blocking ? "blocking" : ""} ${withLead ? "lead" : ""}" id="in-q${q.id}">${face(q.name, q.blocking ? "blocked" : "neutral")}<div>
    <span class="kind">${kind} · ${esc(q.key || "")}</span>${clamp(`q-${q.id}`, `<div class="q-text">${md(q.question).html}</div>`, q.question)}${via}
    <p>${esc(first(q.name))} ${q.blocking ? `stopped until ${withLead ? "someone" : "you"} answer${withLead ? "s" : ""}.` : q.default ? `went ahead with “${esc(q.default)}” for now.` : "is carrying on meanwhile."}</p>
    <div class="acts">${opts}${def}${q.blocking || withLead ? "" : `<button class="btn ghost" data-dismiss-q="${q.id}">Dismiss</button>`}${moreBtn(`ans-${q.id}`, "Write your own answer")}</div>
    ${S.open[`ans-${q.id}`] ? `${replyForm(`ans-${q.id}`, `data-reply="${q.id}"`, "Your answer", "Send", true, withLead ? "" : `<label class="keep" title="Adds the question and your answer to Company/Decisions"><input type="checkbox" data-keep> Save to the wiki</label>`)}` : ""}</div></article>`;
}

// Long text shows a few lines until opened, so a card never turns into a wall of text.
export function clamp(id: string, html: string, text: string) {
  const long = text.length > 320 || text.split("\n").length > 5;
  if (!long) return html;
  const open = S.open[id];
  return `<div class="foldable ${open ? "open" : ""}">${html}</div><button class="link-btn inline more-link" type="button" data-more="${esc(id)}">${open ? "Show less" : "Show all"}</button>`;
}

// Someone asks to run a command they aren't allowed yet. Allowing it adds it to the repo's list.
export function commandAsk(q: Question) {
  const cmds = q.question.split("\n").filter(Boolean), many = cmds.length > 1;
  return `<article class="ask blocking" id="in-q${q.id}">${face(q.name, "blocked")}<div>
    <span class="kind">${many ? "Commands" : "Command"} · ${esc(q.key || "")}</span><h4>${cmds.map(c => `<code class="cmd-ask">${esc(c)}</code>`).join(" ")}</h4>
    <p>${esc(first(q.name))} wants to run ${many ? "these" : "this"} in <b>${esc(q.repo)}</b>${q.note ? `: ${esc(q.note)}` : "."}</p>
    <p class="muted">Once allowed, everyone can run ${many ? "them" : "it"} in ${esc(q.repo)}. Or let employees run what they need, and they won't have to ask.</p>
    <div class="acts"><button class="btn primary" data-answer="${q.id}" data-val="allow" data-said="Allowed in ${esc(q.repo)}">${many ? "Allow all" : "Allow"} in ${esc(q.repo)}</button><button class="btn" data-run-any-q="${q.id}">Let them run anything</button><button class="btn ghost" data-answer="${q.id}" data-val="no" data-said="Not allowed">Not now</button></div></div></article>`;
}

// A reply under a card: a few lines to write, sent with ⌘↵, closed with Esc or Cancel.
export const replyForm = (id: string, attrs: string, placeholder: string, send: string, cancel = true, extra = "") =>
  `<form class="reply" ${attrs}><textarea id="${esc(id)}" rows="2" placeholder="${esc(placeholder)}"></textarea>
    <div class="reply-acts">${extra}<span class="fine">⌘↵ to send${cancel ? " · Esc to cancel" : ""}</span>${cancel ? `<button class="btn ghost" type="button" data-close-open="${esc(id)}">Cancel</button>` : ""}<button class="btn primary">${send}</button></div></form>`;

// The card's last action: opens a form under the row, and steps aside while it's open.
export const moreBtn = (id: string, label: string) => S.open[id] ? "" : `<button class="btn ghost" type="button" data-more="${esc(id)}">${label}</button>`;

// Free text stays folded until asked for, so each card leads with its one-click answers.
export function more(id: string, label: string, form: string) {
  return S.open[id] ? form : `<button class="link-btn" type="button" data-more="${esc(id)}">${label}</button>`;
}

function planRows(r: Review) {
  const people = S.snap!.employees;
  return r.tickets.map(t => `<li class="pt">
    <span class="k">${esc(t.key)}</span>
    <div class="pt-main"><b>${esc(t.title)}</b>${t.repo ? `<span class="repo-tag">${esc(t.repo)}</span>` : ""}${t.description ? (() => {
      // Short details show whole. Long ones fold to a few lines until you open them.
      const long = t.description.length > 220 || t.description.split("\n").length > 3, open = !long || S.open["pd-" + t.key];
      return `<div class="pt-desc ${open ? "open" : ""}">${md(t.description).html}</div>${open ? "" : `<button class="link-btn inline more-link" type="button" data-more="pd-${esc(t.key)}">Show details</button>`}`;
    })() : ""}${t.after ? `<span class="after">after ${esc(t.after)}</span>` : ""}</div>
    <select data-plan-assign="${esc(t.key)}" aria-label="Who does ${esc(t.key)}"><option value="">Nobody yet</option>${people.map(e => `<option value="${esc(e.id)}" ${e.id === t.assignee ? "selected" : ""}>${esc(first(e.name))} · ${esc(e.role)}</option>`).join("")}</select>
    <button class="x" data-plan-del="${esc(t.key)}" title="Remove it" aria-label="Remove ${esc(t.key)}">×</button></li>`).join("");
}

export function planReview(r: Review) {
  const rows = planRows(r);
  return `<article class="ask plan" id="in-${esc(r.key)}">${face(r.name, "done")}<div>
    <span class="kind">Project plan · ${esc(r.key)} · by ${esc(first(r.name))}</span><h4>${esc(r.title)}, in ${r.tickets.length} ${r.tickets.length === 1 ? "ticket" : "tickets"}</h4>
    ${r.summary ? `<p>${esc(r.summary)}</p>` : ""}
    <ol class="plan-list">${rows || `<li class="muted">No tickets left. Plan again, or dismiss it.</li>`}</ol>
    <div class="acts"><button class="btn good" data-approve="${esc(r.key)}" ${r.tickets.length ? "" : "disabled"}>Start the plan</button><button class="btn" data-dismiss="${esc(r.key)}">Dismiss</button>${moreBtn(`chg-${r.key}`, "Plan it again…")}</div>
    ${S.open[`chg-${r.key}`] ? `${replyForm(`chg-${r.key}`, `data-changes="${esc(r.key)}"`, "What should change?", "Plan again")}` : ""}</div></article>`;
}

/* A report: the ticket changed no files, so its reply is the work, plus the tickets it proposes. */
export function reportReview(r: Review) {
  const ready = r.tickets.filter(t => t.assignee).length, n = r.tickets.length;
  const doc = r.document || r.summary || "";
  return `<article class="ask review report" id="in-${esc(r.key)}">${face(r.name, "done")}<div>
    <span class="kind">Report · ${esc(r.key)} · by ${esc(first(r.name))}</span><h4>${esc(r.title)}</h4>
    <div class="doc report-doc">${clamp(`rp-${r.key}`, `<div class="wiki">${md(doc).html}</div>`, doc)}</div>
    ${n ? `<span class="label">Proposed next · ${n}</span><ol class="plan-list">${planRows(r)}</ol>` : ""}
    <div class="acts">${ready ? `<button class="btn good" data-approve="${esc(r.key)}">Start ${ready} ${ready === 1 ? "ticket" : "tickets"}</button><button class="btn" data-close-report="${esc(r.key)}">Close, keep for later</button>` : `<button class="btn good" data-close-report="${esc(r.key)}">Done</button>`}${moreBtn(`chg-${r.key}`, "Ask a follow-up")}</div>
    ${S.open[`chg-${r.key}`] ? `${replyForm(`chg-${r.key}`, `data-changes="${esc(r.key)}"`, "What else should they look into?", "Send")}` : ""}</div></article>`;
}

export function approvalCopy(t?: { kind?: string; key?: string; empty?: boolean } | null) {
  const diff = t?.key ? S.diffs[t.key] : undefined;
  if (t?.kind === "work" && (t.empty || (diff && diff !== "loading" && diff.files.length === 0))) return { label: "Close as done", busy: "Closing…", after: "Closed", done: "Closed. Everything it had was already on main." };
  if (t?.kind === "report") return { label: "Close and start what it proposes", busy: "Starting…", after: "Started", done: "Report closed." };
  if (t?.kind === "wiki") return { label: "Save wiki", busy: "Saving…", after: "Saved", done: "Wiki saved. Your team can read it now." };
  if (t?.kind === "plan") return { label: "Start the plan", busy: "Starting…", after: "Started", done: "Plan started." };
  return { label: mergeLabel(), busy: "Merging…", after: "Merged", done: "Work merged. New tickets start from it." };
}

export function reviewActions(t: { key: string; kind?: string; empty?: boolean }) {
  return `<button class="btn good" data-approve="${esc(t.key)}">${approvalCopy(t).label}</button>${t.kind === "work" && !S.diffs[t.key] ? `<button class="btn" data-diff="${esc(t.key)}">Show changes</button>` : ""}`;
}

// Last in the row: the one action that throws work away.
export const discardBtn = (t: { key: string; kind?: string }) => t.kind === "work" && S.confirm !== "discard:" + t.key ? `<button class="btn quiet-danger" data-confirm="discard:${esc(t.key)}">Discard…</button>` : "";

// Discarding asks once, right where you clicked: the work is gone after, the ticket isn't.
export function discardConfirm(t: { key: string; pr_url?: string | null }) {
  if (S.confirm !== "discard:" + t.key) return "";
  return `<div class="confirm"><p>Throw away ${esc(t.key)}'s work?${t.pr_url ? " Its pull request is closed and its branch deleted." : " Its branch is deleted."} The ticket goes back to the backlog, to start again or delete.</p>
    <div class="acts"><button class="btn danger" data-discard="${esc(t.key)}">Discard the work</button><button class="btn ghost" data-confirm="">Keep it</button></div></div>`;
}

export function wikiDraft(t: { kind?: string; status?: string; document?: string | null }) {
  if (t.kind === "report" && t.document) return `<article class="doc report-doc"><div class="wiki">${md(t.document).html}</div></article>`;
  return t.kind === "wiki" && (!t.status || t.status === "review") ? `<article class="doc wiki">${md(t.document || "No wiki draft was saved. Ask for changes to write it again.").html}</article>` : "";
}

export function ticketStatus(t: { status: string; kind?: string }) {
  return t.status === "done" && ["wiki", "plan", "report"].includes(t.kind ?? "") ? (t.kind === "plan" ? "Plan started" : "Completed") : (STATUS[t.status] || t.status);
}

export function review(r: Review) {
  if (r.kind === "report") return reportReview(r);
  return `<article class="ask review" id="in-${esc(r.key)}">${face(r.name, "done")}<div>
    <span class="kind">Review · ${esc(r.key)}</span><h4>${esc(r.title)}</h4>${r.pr_url ? `<p class="prline">${prLink(r.pr_url, r.pr_number)}</p>` : ""}${clamp(`r-${r.key}`, `<div class="q-text summary">${md(r.summary || `${first(r.name)} finished and is waiting for your review.`).html}</div>`, r.summary || "")}
    ${r.empty ? `<p class="nothing-left">Nothing left to merge: everything on this branch is already on main. Close it as done${r.pr_url ? ", and its pull request is closed" : ""}.</p>` : ""}${r.kind === "wiki" ? `<p>Save these pages to the wiki. Only the pages in the draft change, and what you wrote yourself is kept.</p>` : ""}${wikiDraft(r)}<div class="acts">${reviewActions(r)}${S.open[`chg-${r.key}`] ? "" : `<button class="btn ghost" type="button" data-more="chg-${esc(r.key)}">Ask for changes</button>`}${discardBtn(r)}</div>
    ${S.open[`chg-${r.key}`] ? `${replyForm(`chg-${r.key}`, `data-changes="${esc(r.key)}"`, "What should change?", "Send back")}` : ""}
    ${discardConfirm(r)}${changes(r.key)}</div></article>`;
}

export function wiki(q: Question) {
  return `<article class="ask" id="in-q${q.id}">${face(q.name, "neutral")}<div>
    <span class="kind">Wiki · ${esc(q.section)}</span>${clamp(`w-${q.id}`, `<div class="q-text">${md(q.question).html}</div>`, q.question)}<p>${esc(first(q.name))} thinks everyone should follow this from now on.</p>
    <div class="acts"><button class="btn primary" data-answer="${q.id}" data-val="yes">Add to the wiki</button><button class="btn" data-answer="${q.id}" data-val="no">Not for everyone</button>${moreBtn(`ans-${q.id}`, "Add it in your own words")}</div>
    ${S.open[`ans-${q.id}`] ? `${replyForm(`ans-${q.id}`, `data-reply="${q.id}"`, "The rule, as you'd put it", "Add")}` : ""}</div></article>`;
}

/* The changes behind a review: commits, files, then the diff file by file. Loaded on click. */
export function changes(key: string) {
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
    // Old and new line numbers in the gutter, counted from each hunk's header.
    let o = 0, n = 0;
    const ln = (a: number | string, b: number | string) => `${String(a).padStart(4)} ${String(b).padStart(4)}`;
    const html = lines.map(l => {
      const h = l.match(/^@@ -(\d+)(?:,\d+)? \+(\d+)/);
      if (h) { o = +h[1]; n = +h[2]; return `<span class="hunk">${esc(l)}</span>`; }
      const [cls, gut] = l.startsWith("+") ? ["add", ln("", n++)] : l.startsWith("-") ? ["del", ln(o++, "")] : l.startsWith("\\") ? ["", ln("", "")] : ["", ln(o++, n++)];
      return `<span class="${cls}" data-ln="${gut}">${esc(l) || " "}</span>`;
    }).join("\n");
    return `<details class="file" ${open ? "open" : ""}><summary><span class="mono">${esc(path)}</span><span class="n"><span class="ok">+${f.added}</span> <span class="del">−${f.removed}</span></span></summary><pre class="patch">${html}</pre></details>`;
  }).join("");
  return `<div class="changes">
    <div class="diffsum"><b>${files} ${files === 1 ? "file" : "files"}</b> <span class="ok">+${d.added}</span> <span class="del">−${d.removed}</span> <span class="muted">· ${esc(d.branch)} into ${esc(d.base)}</span>
      <button class="btn" data-hide-diff="${esc(key)}">Hide</button></div>
    ${commits ? `<ul class="commits">${commits}</ul>` : ""}${body || `<p class="muted">No file changes on this branch: everything it had is already on main. Close it as done.</p>`}
    ${d.truncated ? `<p class="muted">This change is too big to show in full. See the rest with <code>onecorp diff ${esc(key)}</code>.</p>` : ""}</div>`;
}
