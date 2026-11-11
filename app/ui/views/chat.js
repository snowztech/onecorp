import { $, esc, first, fold, hhmm, sameMinutes } from "../lib.js";
import { agentBadge, face, icon, me } from "../look.js";
import { inline } from "../md.js";
import { S, byId } from "../state.js";

/* Chat: #general and a channel per department. Your team posts as they work; @mention someone to ask them. */
export function viewChat() {
  const s = S.snap, chans = s.channels || [], dms = s.dms || [];
  const dmRow = dms.find(d => d.channel === S.channel), person = dmRow && byId(dmRow.id);
  const ch = person ? { name: dmRow.channel, last: dmRow.last } : chans.find(c => c.name === S.channel) || chans[0];
  if (!ch) return "";
  S.seen[ch.name] = ch.last || 0;
  const dept = person ? null : s.departments.find(d => d.id === ch.department);
  const people = person ? [person] : (dept ? dept.members.map(byId) : s.employees).filter(Boolean);
  const lead = dept?.lead && byId(dept.lead);
  const unread = c => c.last && c.last > (S.seen[c.name || c.channel] || 0) ? '<i class="dot"></i>' : "";
  const list = chans.map(c => `<button class="chan" data-channel="${esc(c.name)}" aria-current="${c.name === ch.name}"><span class="hash">#</span>${esc(c.name)}${unread(c)}</button>`).join("");
  const STATE = { working: "Working", blocked: "Waiting on you", queued: "Up next", idle: "Free" };
  const dmList = dms.map(d => byId(d.id) && `<button class="chan dm" data-channel="${esc(d.channel)}" aria-current="${d.channel === ch.name}"><span class="with-agent">${face(byId(d.id).name, byId(d.id).status)}<i class="st st-${byId(d.id).status}" title="${STATE[byId(d.id).status] || ""}"></i></span>${esc(first(byId(d.id).name))}${unread(d)}</button>`).filter(Boolean).join("");
  const msgs = S.chat?.channel === ch.name ? S.chat.messages : null;
  let prev = null;
  const rows = (msgs || []).map(m => {
    const same = prev && prev.author === m.author && !prev.pending && !m.pending && sameMinutes(prev.at, m.at, 5);
    prev = m;
    const who = m.founder ? me() : face(m.name, m.pending ? "working" : "neutral");
    const body = m.pending ? `<p class="typing">writing<span><i></i><i></i><i></i></span></p>` : `<div class="msg-text">${chatText(m.text)}</div>`;
    const meta = `<header><b>${esc(m.founder ? m.name : first(m.name))}</b>${m.founder ? '<span class="you">you</span>' : ""}<time>${hhmm(m.at)}</time>${m.key ? `<a class="tkey" data-ticket="${esc(m.key)}">${esc(m.key)}</a>` : ""}</header>`;
    const del = m.pending ? "" : `<button class="msg-del ${S.msgDel === m.id ? "sure" : ""}" data-msg-del="${m.id}" title="Delete message" aria-label="Delete message">${S.msgDel === m.id ? "Delete?" : icon("trash")}</button>`;
    return `<li class="msg ${same ? "cont" : ""}">${same ? "<span></span>" : who}<div>${same ? "" : meta}${body}</div>${del}</li>`;
  }).join("");
  const empty = person
    ? `<li class="chat-empty"><b>Message ${esc(first(person.name))}</b>Just between you two. ${esc(first(person.name))} replies to every message and reads this before their next ticket.</li>`
    : `<li class="chat-empty"><b>Nothing in #${esc(ch.name)} yet</b>Your team posts here as they work. Mention someone with @ to ask them something.</li>`;
  const sub = person ? `${esc(person.role)} · ${STATE[person.status] || ""}` : dept ? `${lead ? "Led by " + esc(first(lead.name)) + " · " : ""}${people.length} ${people.length === 1 ? "person" : "people"}` : ch.name === "general" ? "Everyone at " + esc(s.company) : "";
  return `<div class="chat">
    <nav class="chans" aria-label="Channels"><h6>Channels</h6>${list}${dmList ? `<h6 class="dm-h">Direct messages</h6>${dmList}` : ""}</nav>
    <section class="room-chat">
      <header class="chat-h">${person ? `<span class="with-agent dm-face">${face(person.name, person.status)}${agentBadge(person.runtime)}</span>` : ""}<div><h3>${person ? esc(person.name) : "#" + esc(ch.name)}</h3><span class="muted">${sub}</span></div>
        ${person ? `<button class="btn ghost" data-person="${esc(person.id)}">Profile</button>` : members(ch.name, people, lead)}</header>
      <ul class="msgs" id="msgs">${msgs === null ? `<li class="chat-empty muted">Loading…</li>` : rows || empty}</ul>
      <form class="composer" id="say"><div class="compose-box"><div class="mpop" id="mpop" role="listbox" hidden></div>
        <input id="say-text" placeholder="${person ? `Message ${esc(first(person.name))}` : `Message #${esc(ch.name)}. Type @ to ask someone, @all for everyone`}" autocomplete="off" role="combobox" aria-controls="mpop">
        <p class="calls" id="calls"></p></div><button class="btn primary">Send</button></form>
    </section></div>`;
}

// Who's in a channel, like Slack: the face stack opens the list, with what each person is up to.
export function members(channel, people, lead) {
  const STATE = { working: "Working", blocked: "Waiting on you", queued: "Up next", idle: "Free" };
  const open = S.members === channel;
  const rows = people.map(e => `<li><button class="mem" data-mention="${esc(e.id)}" title="Mention ${esc(first(e.name))}">${face(e.name, e.status)}
      <span><b>${esc(e.name)}${lead?.id === e.id ? ' <small class="muted">lead</small>' : ""}</b><small class="muted">${esc(e.role)} · ${STATE[e.status] || ""}</small></span></button>
      <button class="btn ghost sm" data-dm="${esc(e.id)}">Message</button></li>`).join("");
  return `<button class="faces" data-members="${esc(channel)}" aria-expanded="${open}" title="${esc(people.map(e => first(e.name)).join(", "))}">
      ${people.slice(0, 5).map(e => `<span class="face-btn">${face(e.name, e.status)}</span>`).join("")}<span class="faces-n">${people.length}</span></button>
    ${open ? `<div class="members-pop" role="dialog" aria-label="People in #${esc(channel)}"><h6>${people.length} ${people.length === 1 ? "person" : "people"}</h6><ul>${rows}</ul></div>` : ""}`;
}

/* Mentions: type @ for a list of your team, and see who'll reply before you send. */
// Everyone in the channel you're in: the whole company in #general, a department's people in theirs.
export function channelPeople() {
  const dm = (S.snap.dms || []).find(d => d.channel === S.channel);
  if (dm) return [byId(dm.id)].filter(Boolean);
  const ch = (S.snap.channels || []).find(c => c.name === S.channel), dept = S.snap.departments.find(d => d.id === ch?.department);
  return (dept ? dept.members.map(byId) : S.snap.employees).filter(Boolean);
}

// Who a message calls, the same way the core reads it: @firstname, or @all for the channel.
export function calledBy(text) {
  const team = S.snap.employees, out = [];
  if (String(S.channel).startsWith("dm:")) out.push(...channelPeople());
  for (const [, w] of String(text).matchAll(/@([\p{L}\d]+)/gu)) {
    const id = fold(w);
    const add = ["all", "everyone", "channel"].includes(id) ? channelPeople() : team.filter(e => e.id === id);
    for (const e of add) if (!out.includes(e)) out.push(e);
  }
  return out;
}

export function mentionAt(input) {
  const m = input.value.slice(0, input.selectionStart).match(/(^|\s)@([\p{L}\d]*)$/u);
  return m ? fold(m[2]) : null;
}

export function updateMentions(input) {
  const pop = $("#mpop"), calls = $("#calls");
  if (!pop) return;
  const q = mentionAt(input), here = channelPeople();
  if (q === null) { pop.hidden = true; S.mention = null; }
  else {
    const people = [...here, ...S.snap.employees.filter(e => !here.includes(e))].filter(e => fold(first(e.name)).startsWith(q) || e.id.startsWith(q));
    const all = "all".startsWith(q) ? [{ all: true }] : [];
    const opts = [...people, ...all].slice(0, 7);
    S.mention = opts.length ? { opts, i: Math.min(S.mention?.i || 0, opts.length - 1) } : null;
    pop.hidden = !S.mention;
    if (S.mention) pop.innerHTML = opts.map((o, k) => o.all
      ? `<button type="button" class="mopt" data-mpick="${k}" aria-selected="${k === S.mention.i}"><span class="all-ic">@</span><span><b>all</b><small>Everyone in #${esc(S.channel)}, ${here.length} ${here.length === 1 ? "person" : "people"}</small></span></button>`
      : `<button type="button" class="mopt" data-mpick="${k}" aria-selected="${k === S.mention.i}"><span class="with-agent">${face(o.name, o.status)}${agentBadge(o.runtime)}</span><span><b>${esc(first(o.name))}</b><small>${esc(o.role)}${here.includes(o) ? "" : " · not in this channel"}</small></span></button>`).join("");
  }
  // Who'll reply, before you send. Each one is a paid AI reply, so the count is said out loud.
  const who = calledBy(input.value);
  const names = who.length > 3 ? `${who.length} people` : who.map(e => esc(first(e.name))).join(who.length === 2 ? " and " : ", ");
  const quiet = !who.length && input.value.trim() && !String(S.channel).startsWith("dm:");
  calls.innerHTML = who.length ? `<span class="calls-faces">${who.slice(0, 4).map(e => face(e.name, "neutral")).join("")}</span><span>${names} will reply here</span>`
    : quiet ? `<span class="muted">No one is mentioned, so no one will reply. Add @ to ask someone.</span>` : "";
}

export function pickMention(k) {
  const input = $("#say-text"), o = S.mention?.opts[k];
  if (!o) return;
  const at = input.selectionStart, before = input.value.slice(0, at).replace(/@[\p{L}\d]*$/u, "@" + (o.all ? "all" : first(o.name)) + " ");
  input.value = before + input.value.slice(at);
  input.setSelectionRange(before.length, before.length);
  S.mention = null; input.focus(); updateMentions(input);
}

// Mentions and ticket keys stand out, and a ticket key opens the ticket.
// Agents write Markdown, so chat reads it like Slack does: code, bold, italics, links and code
// blocks. Mentions and keys are only picked out of plain text, never inside code.
export function chatText(text) {
  const asking = String(text).match(/\n\(Asking (.+)\.\)$/);
  if (asking) {
    const toYou = /founder/.test(asking[1]);
    const note = toYou ? `<span class="msg-meta">Asked you · <button class="link-btn inline" data-view="inbox">Answer in Inbox</button></span>` : `<span class="msg-meta">Asked ${chatText(asking[1])}</span>`;
    return chatText(String(text).slice(0, asking.index)) + note;
  }
  const blocks = String(text).split(/```(?:\w+)?\n?/);
  return blocks.map((part, i) => {
    if (i % 2) return `<pre class="chat-code"><code>${esc(part.replace(/\n$/, ""))}</code></pre>`;
    return inline(part).split(/(<code>[\s\S]*?<\/code>|<a [^>]*>[\s\S]*?<\/a>)/).map((bit, j) => j % 2 ? bit
      : bit.replace(/@([\p{L}\d]+)/gu, '<span class="mention">@$1</span>').replace(/\b([A-Z]{2,5}-\d+)\b/g, '<a class="tkey" data-ticket="$1">$1</a>')).join("");
  }).join("");
}
