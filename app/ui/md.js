import { esc } from "./lib.js";

// A small Markdown renderer for the wiki: headings, lists, tables, quotes, code, bold, italics, links.
// Enough for a wiki page, and nothing that needs a library.
export function inline(t) {
  return esc(t)
    .replace(/`([^`]+)`/g, "<code>$1</code>")
    .replace(/\*\*([^*]+)\*\*/g, "<b>$1</b>")
    .replace(/(^|[^*])\*([^*\s][^*]*)\*/g, "$1<em>$2</em>")
    .replace(/\[([^\]]+)\]\((https?:[^)\s]+)\)/g, '<a class="link" data-open-url="$2">$1</a>');
}

// A table row's cells. A | inside `code` stays part of the cell.
export function cells(line) {
  const out = [];
  let cur = "", tick = false;
  for (const ch of line.trim().replace(/^\|/, "").replace(/\|$/, "")) {
    if (ch === "`") tick = !tick;
    if (ch === "|" && !tick) { out.push(cur.trim()); cur = ""; } else cur += ch;
  }
  return [...out, cur.trim()];
}

// What OneCorp writes when you skip the mission. Shown as a hint, not as if it were yours.
export const MISSION_HINT = "What the company is for, in a sentence or two.";

export function md(text) {
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
