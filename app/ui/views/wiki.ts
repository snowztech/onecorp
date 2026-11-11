import { ago, esc, first } from "../lib.ts";
import { face } from "../look.ts";
import { md } from "../md.ts";
import { S } from "../state.ts";
import type { Question, WikiPage } from "../types.ts";

/* Wiki: the company's shared knowledge, in spaces and pages, each saying who wrote it.
   Every employee reads every page before each ticket. */

// What goes on a page nobody has written yet. Yours to fill in, in a line or two.
const HINTS: Record<string, string> = {
  "Company/Mission": "Why the company exists, in a sentence or two.",
  "Company/What we're building": "The product, who it's for, and what it does today. Your planner works from it.",
  "Company/Now": "What the team is focused on right now, and what not to touch. Everyone checks it before deciding how to do a ticket.",
  "Company/Decisions": "Calls that are settled, with why, so nobody reopens them. Rules your team proposes land here once you add them.",
  "Product/Customers": "Who uses the product, what they're trying to get done, and what they care about.",
  "Product/Rules": "Rules that are easy to break without knowing: limits, money, privacy, legal, promises made to users.",
  "Product/Voice": "How the product talks to people: tone, words to use and avoid, languages.",
  "Engineering/Codebase": "What isn't obvious from a quick look at the code: the stack, where the important pieces live, outside services.",
  "Engineering/Build and test": "The exact commands to set up, run and test, and what to run before calling work done.",
  "Engineering/Conventions": "How the code is written here: naming, structure, style, testing.",
};
const SPACES = ["Company", "Product", "Engineering"];

const proposals = (path?: string) => S.snap!.inbox.filter(q => q.kind === "wiki" && (!path || q.section === path));

// Pages, plus pages someone proposed that don't exist yet, so their proposals have somewhere to show.
function allPages(): WikiPage[] {
  const pages = [...S.wiki!.pages];
  for (const q of proposals()) {
    if (q.section.includes("/") && !pages.some(p => p.path === q.section)) {
      const [space, title] = q.section.split("/");
      pages.push({ path: q.section, space, title, text: "", edits: 0, by: null, at: null, note: null, first_by: null });
    }
  }
  return pages;
}

export const currentPage = () => {
  const pages = allPages();
  return pages.find(p => p.path === S.page) || pages.find(p => p.text) || pages[0] || null;
};

function tree(pages: WikiPage[], cur: WikiPage | null) {
  const spaces = [...new Set([...SPACES.filter(s => pages.some(p => p.space === s)), ...pages.map(p => p.space)])];
  return `<nav class="wk-tree" aria-label="Wiki pages">
    <div class="wk-tree-head"><span class="label">Spaces</span><button class="icon-btn" data-wk-new="" title="New page" aria-label="New page">+</button></div>
    ${spaces.map(s => `<div class="wk-space"><span class="wk-space-name">${esc(s)}</span>
      ${pages.filter(p => p.space === s).map(p => {
        const n = proposals(p.path).length;
        return `<button class="wk-link ${p.path === cur?.path && !S.wikiNew ? "on" : ""} ${p.text ? "" : "empty"}" data-wk-page="${esc(p.path)}">${esc(p.title)}${n ? `<span class="wk-badge" title="${n} proposed">${n}</span>` : ""}</button>`;
      }).join("")}
      <button class="wk-add" data-wk-new="${esc(s)}">+ Page</button></div>`).join("")}
    <p class="fine">Everyone reads every page before each ticket.</p>
    ${S.open.redraft ? `<form class="wk-redraft" id="wk-redraft"><span class="label">Redraft from the repo</span>
        <select id="wk-by" aria-label="Who drafts it"><option value="">Your team</option>${S.snap!.employees.map(e => `<option value="${esc(e.id)}">${esc(e.name)}</option>`).join("")}</select>
        <p class="fine">Drafted pages are rewritten from the repo. Pages only you write, like Now, aren't touched, and every earlier version stays in History.</p>
        <div class="acts"><button class="btn primary">Redraft</button><button class="btn ghost" type="button" data-close-open="redraft">Cancel</button></div></form>`
      : `<button class="link-btn wk-more" data-more="redraft">Redraft from the repo…</button>`}</nav>`;
}

function proposalCards(list: Question[]) {
  if (!list.length) return "";
  return `<section class="doc-props"><span class="label">Proposed by your team</span><div class="props">${list.map(q => `
    <div class="prop">${face(q.name, "neutral")}<div><b>${esc(first(q.name))}</b> <span class="muted">wants to add</span><p>${esc(q.question)}</p>
      <div class="acts"><button class="btn primary" data-answer="${q.id}" data-val="yes">Add</button><button class="btn" data-answer="${q.id}" data-val="no">Not for everyone</button></div></div></div>`).join("")}</div></section>`;
}

function meta(p: WikiPage) {
  if (!p.by) return `<p class="wk-meta muted">Nobody has written this page yet.</p>`;
  const wrote = p.first_by && p.first_by !== p.by ? `Written by <b>${esc(p.first_by)}</b> · edited by <b>${esc(p.by)}</b>` : `By <b>${esc(p.by)}</b>`;
  return `<p class="wk-meta">${face(p.by === "You" ? (S.founder?.name || "You") : p.by, "neutral")}<span>${wrote} ${p.at ? `· ${esc(ago(p.at))}` : ""}${p.note ? ` <span class="muted">· ${esc(p.note)}</span>` : ""}</span>
    <button class="link-btn" data-wk-history="${esc(p.path)}">History${p.edits > 1 ? ` (${p.edits})` : ""}</button></p>`;
}

function history(p: WikiPage) {
  const list = S.history || [];
  const v = list.find(e => e.id === S.version);
  return `<section class="wk-hist"><div class="wk-hist-head"><span class="label">History of ${esc(p.title)}</span><button class="btn ghost" data-wk-history="">Close</button></div>
    <ol>${list.map((e, i) => `<li><button class="wk-ver ${e.id === S.version ? "on" : ""}" data-wk-version="${e.id}">
      <b>${esc(e.by)}</b> <span class="muted">${esc(e.note || (i === list.length - 1 ? "created it" : "edited it"))}</span><span class="muted at">${esc(ago(e.at))}</span></button></li>`).join("") || `<li class="muted">No saved versions yet.</li>`}</ol>
    ${v ? `<div class="wk-version"><div class="wk-hist-head"><span class="muted">The page as ${v.by === "You" ? "you" : esc(first(v.by))} left it, ${esc(ago(v.at))}</span>${v.id !== list[0]?.id ? `<button class="btn" data-wk-restore="${v.id}">Restore this version</button>` : `<span class="muted">Current</span>`}</div>
      <div class="wiki">${v.text ? md(v.text).html : `<p class="muted">Empty: the page was deleted here.</p>`}</div></div>` : ""}</section>`;
}

function editor(p: WikiPage | null) {
  const space = p?.space || S.wikiSpace || "Company", title = p?.title || "", text = p?.text ?? "";
  const spaces = [...new Set([...SPACES, ...S.wiki!.pages.map(x => x.space)])];
  const del = p && S.wiki!.pages.some(x => x.path === p.path) ? (S.confirm === "page:" + p.path
    ? `<span class="confirm-inline">Delete ${esc(p.title)}? Its history stays. <button class="btn danger" type="button" data-wk-delete="${esc(p.path)}">Delete</button><button class="btn ghost" type="button" data-confirm="">Keep</button></span>`
    : `<button class="link-btn danger-link" type="button" data-confirm="page:${esc(p.path)}">Delete page…</button>`) : "";
  return `<form class="wiki-edit" id="hb-raw" data-from="${esc(p && S.wiki!.pages.some(x => x.path === p.path) ? p.path : "")}">
    <header><div class="wk-name"><input id="wk-space" list="wk-spaces" value="${esc(space)}" aria-label="Space" placeholder="Space"><span>/</span><input id="wk-title" value="${esc(title)}" aria-label="Title" placeholder="Page title"></div>
      <datalist id="wk-spaces">${spaces.map(s => `<option value="${esc(s)}">`).join("")}</datalist>
      <div class="acts">${del}<button class="btn ghost" type="button" data-hb-edit="0">Cancel</button><button class="btn primary">Save</button></div></header>
    <span class="muted fine">Markdown. ## starts a section. Everyone reads it from their next ticket.</span>
    <div class="wiki-split"><textarea id="hb-text" spellcheck="false" placeholder="${esc(HINTS[p?.path || ""] || "Write it the way you'd brief a new teammate.")}">${esc(text)}</textarea><article class="doc wiki" id="hb-preview">${md(text).html}</article></div></form>`;
}

export function viewWiki() {
  if (S.wiki === null) return `<p class="muted">Opening the wiki…</p>`;
  const pages = allPages(), p = currentPage();
  if (S.wikiNew || S.hbEdit) return `<div class="wk">${tree(pages, p)}<main class="wk-main">${editor(S.wikiNew ? null : p)}</main></div>`;
  if (!p) return `<div class="wk">${tree(pages, p)}<main class="wk-main"><div class="wk-empty"><p>No pages yet.</p><button class="btn primary" data-wk-new="">New page</button></div></main></div>`;
  const page = md(p.text);
  const hint = HINTS[p.path];
  const body = p.text ? `<div class="wiki">${page.html}</div>`
    : `<div class="wk-empty"><p>${esc(hint || "This page is empty.")}</p><button class="btn primary" data-hb-edit="1">Write it</button></div>`;
  const learn = p.path === "Engineering/Codebase" ? `<footer class="doc-foot"><button class="link-btn" data-learn="1">Update from the repo</button></footer>` : "";
  return `<div class="wk">${tree(pages, p)}<main class="wk-main"><article class="doc">
      <div class="wk-crumbs">${esc(p.space)} <span>›</span></div>
      <div class="wiki-top"><h1>${esc(p.title)}</h1>${p.text ? `<button class="btn" data-hb-edit="1">Edit</button>` : ""}</div>
      ${meta(p)}
      ${S.history && S.historyOf === p.path ? history(p) : ""}
      ${proposalCards(proposals(p.path))}
      ${body}${learn}
    </article></main>
    ${page.toc.length > 2 ? `<nav class="toc" aria-label="On this page"><span class="label">On this page</span>${page.toc.map(([id, n]) => `<button data-toc="${id}">${esc(n.replace(/[`*]/g, ""))}</button>`).join("")}</nav>` : ""}</div>`;
}
