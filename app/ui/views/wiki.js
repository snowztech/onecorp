import { esc, first } from "../lib.js";
import { face } from "../look.js";
import { inline, md } from "../md.js";
import { S } from "../state.js";

/* Wiki: the company's shared knowledge, the file every employee reads before each ticket */
export function viewWiki() {
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
