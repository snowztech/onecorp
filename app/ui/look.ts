import { esc, hash, initials } from "./lib.ts";
import { render, typing } from "./render.ts";
import { S } from "./state.ts";
import type { Employee, Look, Model } from "./types.ts";
import { renderSheet } from "./views/settings.ts";

// How people, agents and companies look: faces, icons, agent marks and company badges.
/* Faces: drawn from the name, so everyone keeps the same face. The mood follows what they're doing.
   Each person gets one of eight hues, soft in light and deep in dark, so the team has colour that sits with the UI. */
export const HUES = [215, 12, 158, 262, 38, 330, 192, 95];

export const SKIN = ["#f3d5bd", "#e6b590", "#c88e64", "#9a6542", "#6b442b"];

export const HAIR = ["#1e1916", "#4b301e", "#8b5b2c", "#d0a660", "#b4442f", "#70747d"];

export function face(name: string, status: string) {
  const e = S.snap?.employees?.find(x => x.name === name);
  const url = e && photoUrl(e);
  if (url) return `<span class="av"><img src="${esc(url)}" alt=""></span>`;
  return drawFace(e?.face || name, status);
}

// A photo straight from its file, so it shows on the first render. The stamp changes with the file,
// so a new photo isn't served from the webview's cache.
const photoUrl = (e: Employee) => {
  const toUrl = window.__TAURI__?.core.convertFileSrc;
  return e.photo && e.photo_path && toUrl ? `${toUrl(e.photo_path)}?v=${e.photo}` : null;
};

// Other face sets, drawn once per person and kept. Ours is the default. Each person keeps their
// colour, light enough under the ink to read in both themes.
export const FACE_SETS = [["ours", "OneCorp", "Drawn here, with moods that follow their work."], ["lorelei", "Lorelei", "Fine-line portraits by Lisa Wischofsky."], ["notionists", "Notionists", "Ink sketches by Zoish."]];

export const faceSet = () => { try { const f = localStorage.getItem("onecorp.faces"); return f && FACE_SETS.some(([k]) => k === f) ? f :  "ours"; } catch { return "ours"; } };

export const drawn: Record<string, string> = {};

// The other sets are half a megabyte, so they load the first time one is shown.
export let facesLoading: Promise<void> | null = null;

export const loadFaces = () => (facesLoading ||= import("./vendor/faces.js").then(() => { if (!typing()) render(); renderSheet(); }));

export function drawFace(name: string, status: string, set = faceSet()) {
  if (set !== "ours") {
    const tile = `<span class="av set-${set}" style="--h:${HUES[(hash(name) >>> 12) % 8]}">`;
    if (!window.Faces) { loadFaces(); return tile + "</span>"; }
    const key = set + "|" + name;
    drawn[key] ||= window.Faces(set, String(name));
    return tile + drawn[key] + "</span>";
  }
  const h = hash(name), L = { skin: h % 5, hair: (h >>> 3) % 6, style: (h >>> 6) % 5, acc: (h >>> 9) % 4 }, hue = HUES[(h >>> 12) % 8];
  const mood = ({ working: "working", blocked: "blocked", idle: "sleep", queued: "neutral", done: "happy" } as Record<string, string>)[status] || "neutral";
  const sk = SKIN[L.skin], hr = HAIR[L.hair], ink = "#18181b";
  const cap = '<path d="M19 30q0-17 13-17t13 17q-3-9-13-9t-13 9z"/>';
  const back = L.style === 1 ? '<rect x="18" y="20" width="28" height="27" rx="9"/>' : "";
  const front = [cap, cap, '<circle cx="32" cy="12" r="5"/>' + cap,
    [[20, 22], [25, 17], [32, 15], [39, 17], [44, 22]].map(([x, y]) => `<circle cx="${x}" cy="${y}" r="5.5"/>`).join(""),
    '<path d="M20 25q12-12 24 0q-12-5-24 0z"/>'][L.style];
  const eyes = '<circle cx="27" cy="31" r="1.7"/><circle cx="37" cy="31" r="1.7"/>';
  const E: Record<string, string> = { happy: '<path d="M24.5 31q2.5-3 5 0M34.5 31q2.5-3 5 0" stroke-width="1.8" fill="none"/>', sleep: '<path d="M24.5 30.5h5M34.5 30.5h5" stroke-width="1.8"/>',
    blocked: eyes + '<path d="M24 26l5-2M40 26l-5-2" stroke-width="1.6"/>', working: eyes, neutral: eyes };
  const M: Record<string, string> = { happy: '<path d="M27 36q5 6 10 0z" fill="#fff" stroke-width="1.5"/>', sleep: '<path d="M30 37.5h4" stroke-width="1.6"/>',
    blocked: '<path d="M28 39q4-3.5 8 0" fill="none" stroke-width="1.6"/>', working: '<path d="M29 37.5h6" stroke-width="1.6"/>', neutral: '<path d="M28 36.5q4 3 8 0" fill="none" stroke-width="1.6"/>' };
  const A = ["", `<g fill="none" stroke="${ink}" stroke-width="1.5"><circle cx="27" cy="30.5" r="4.2"/><circle cx="37" cy="30.5" r="4.2"/><path d="M31.2 30.5h1.6"/></g>`,
    '<path d="M18 31a14 14 0 0 1 28 0" fill="none" stroke="#2f3440" stroke-width="3"/><rect x="15" y="27" width="5" height="10" rx="2" fill="#2f3440"/><rect x="44" y="27" width="5" height="10" rx="2" fill="#2f3440"/>',
    `<path d="M18.5 27q0-15 13.5-15t13.5 15z" fill="#3f3f46"/><rect x="18" y="24" width="28" height="5" rx="2.5" fill="#27272a"/>`][L.acc];
  return `<span class="av"><svg viewBox="0 0 64 64" aria-hidden="true"><rect width="64" height="64" style="fill:hsl(${hue} var(--av-tile))"/>
    <g fill="${hr}">${back}</g><path d="M11 64q2-17 21-17t21 17z" style="fill:hsl(${hue} var(--av-shirt))" stroke="${ink}" stroke-width="1.6"/><rect x="28.5" y="40" width="7" height="9" rx="2" fill="${sk}"/>
    <circle cx="32" cy="30" r="13" fill="${sk}" stroke="${ink}" stroke-width="1.6"/><g fill="${hr}">${front}</g><g fill="${ink}" stroke="${ink}" stroke-linecap="round">${E[mood]}${M[mood]}</g>${A}</svg></span>`;
}

export const RUNTIME: Record<string, [string, string]> = { claude: ["Claude Code", "#d97757"], codex: ["Codex", "#3a9b7a"] };

// The model's own name, like Opus 5.5. With none set, the agent runs your own default, so say which.
// The models an agent offers, as the agent itself lists them.
export const modelsOf = (runtime: string): Model[] => (runtime === "claude" || runtime === "codex" ? S.models?.[runtime] : null) || [];
export function modelName(runtime: string, model: string | null) {
  if (!model) { const d = S.models?.defaults?.[runtime]; return d ? `${d} (default)` : "Default"; }
  return modelsOf(runtime).find(m => m.id === model || (!model.includes("-") && m.id.includes(model)))?.name || model;
}

export function chip(e: { runtime: string; model: string | null }) {
  const [label, color] = RUNTIME[e.runtime] || [e.runtime, "#888"];
  return `<span class="chip"><i style="background:${color}"></i>${esc(label)}${RUNTIME[e.runtime] ? " · " + esc(modelName(e.runtime, e.model)) : ""}</span>`;
}

// Links to a pull request open in your browser, not inside the app.
export function prLink(url: string | null | undefined, n: number | null | undefined) {
  if (!url) return "";
  const where = S.snap?.ship?.forge || "the forge";
  return `<a class="link" data-open-url="${esc(url)}">#${esc(n)} on ${esc(where)} ↗</a>`;
}

/* You, the founder: your photo, or a face generated from your name */
export function me() {
  const f = S.founder;
  return f?.photo ? `<span class="av"><img src="${esc(f.photo)}" alt=""></span>` : monogram(f?.name);
}

export const monogram = (name: string | null | undefined) => `<span class="av mono-av">${String(name || "").trim() ? esc(initials(name)) : icon("user")}</span>`;

/* Icons: one small stroke set, so every page speaks the same visual language */
export const ICON: Record<string, string> = {
  company: '<path d="M4 21V5a1 1 0 0 1 1-1h9a1 1 0 0 1 1 1v16M15 9h4a1 1 0 0 1 1 1v11M8 8h3M8 12h3M8 16h3M3 21h18"/>',
  board: '<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M9 4v16M15 4v16"/>',
  inbox: '<path d="M3 13h5l1.5 3h5l1.5-3h5"/><path d="M5.5 5h13l2.5 8v5a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-5z"/>',
  chat: '<path d="M20 12a8 8 0 0 1-11.6 7.1L4 20l1-4.4A8 8 0 1 1 20 12z"/>',
  wiki: '<path d="M5 4.5A1.5 1.5 0 0 1 6.5 3H19v15H6.5A1.5 1.5 0 0 0 5 19.5z"/><path d="M5 19.5A1.5 1.5 0 0 0 6.5 21H19M9 7h6"/>',
  settings: '<circle cx="12" cy="12" r="3"/><path d="M19 12a7 7 0 0 0-.1-1.2l2-1.6-2-3.4-2.4 1a7 7 0 0 0-2-1.2L14 3h-4l-.5 2.6a7 7 0 0 0-2 1.2l-2.4-1-2 3.4 2 1.6a7 7 0 0 0 0 2.4l-2 1.6 2 3.4 2.4-1a7 7 0 0 0 2 1.2L10 21h4l.5-2.6a7 7 0 0 0 2-1.2l2.4 1 2-3.4-2-1.6c.1-.4.1-.8.1-1.2z"/>',
  coin: '<circle cx="12" cy="12" r="9"/><path d="M15 9.5c-.5-1-1.6-1.5-3-1.5-1.7 0-3 .8-3 2s1.3 1.7 3 2 3 .8 3 2-1.3 2-3 2c-1.4 0-2.5-.5-3-1.5M12 6v2M12 16v2"/>',
  spark: '<path d="M12 3v4M12 17v4M3 12h4M17 12h4M6 6l2.5 2.5M15.5 15.5 18 18M6 18l2.5-2.5M15.5 8.5 18 6"/>',
  eye: '<path d="M2.5 12S6 5.5 12 5.5 21.5 12 21.5 12 18 18.5 12 18.5 2.5 12 2.5 12z"/><circle cx="12" cy="12" r="3"/>',
  info: '<circle cx="12" cy="12" r="9"/><path d="M12 11v6M12 7.5v.5"/>',
  copy: '<rect x="8" y="8" width="12" height="12" rx="2"/><path d="M16 8V5a1 1 0 0 0-1-1H5a1 1 0 0 0-1 1v10a1 1 0 0 0 1 1h3"/>',
  user: '<circle cx="12" cy="8" r="4"/><path d="M4 21a8 8 0 0 1 16 0"/>',
  more: '<circle cx="5" cy="12" r="1"/><circle cx="12" cy="12" r="1"/><circle cx="19" cy="12" r="1"/>',
  check: '<path d="M5 12.5l4.5 4.5L19 7.5"/>',
  chevrons: '<path d="M8 9.5l4-4 4 4M8 14.5l4 4 4-4"/>',
  pause: '<path d="M9 5v14M15 5v14"/>',
  plus: '<path d="M12 5v14M5 12h14"/>',
  trash: '<path d="M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3"/>',
  bug: '<rect x="7" y="7" width="10" height="13" rx="5"/><path d="M12 7V4M9.5 4.5 11 7M14.5 4.5 13 7M3 12h4M17 12h4M4 18l3-2M20 18l-3-2M4 6l3 2M20 6l-3 2"/>',
  logout: '<path d="M15 4h3a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2h-3M10 16l-4-4 4-4M6 12h10"/>',
};

export const icon = (k: string) => `<svg class="ic" viewBox="0 0 24 24" aria-hidden="true">${ICON[k]}</svg>`;

// The agent someone runs on, as a small badge on their avatar.
export function agentBadge(runtime: string) {
  const a = AGENTS[runtime];
  return a ? `<span class="agent-badge" style="--c:${a.color}" title="Works on ${esc(a.name)}">${agentMark(runtime)}</span>` : "";
}

// The agents someone can work on, and the models each offers. Default means the agent's own choice.
export const AGENTS: Record<string, { name: string; by: string; color: string; models: string[][] }> = {
  claude: { name: "Claude Code", by: "Anthropic", color: "#d97757", models: [["", "Default"], ["opus", "Opus"], ["sonnet", "Sonnet"], ["haiku", "Haiku"]] },
  codex: { name: "Codex", by: "OpenAI", color: "#18181b", models: [["", "Default"], ["gpt-5-codex", "GPT-5 Codex"], ["gpt-5", "GPT-5"]] },
  fake: { name: "Stand-in", by: "Free, no AI", color: "#8a90a0", models: [["", "No model"]] },
};

export const GLYPHS: Record<string, string> = {
  claude: '<path d="M12 3.5v17M3.5 12h17M6 6l12 12M18 6 6 18" stroke="#fff" stroke-width="2.4" stroke-linecap="round"/>',
  // OpenAI's mark, from simple-icons (CC0), so Codex reads as OpenAI at a glance.
  codex: '<g transform="translate(2.5 2.5) scale(.79)"><path d="M22.2819 9.8211a5.9847 5.9847 0 0 0-.5157-4.9108 6.0462 6.0462 0 0 0-6.5098-2.9A6.0651 6.0651 0 0 0 4.9807 4.1818a5.9847 5.9847 0 0 0-3.9977 2.9 6.0462 6.0462 0 0 0 .7427 7.0966 5.98 5.98 0 0 0 .511 4.9107 6.051 6.051 0 0 0 6.5146 2.9001A5.9847 5.9847 0 0 0 13.2599 24a6.0557 6.0557 0 0 0 5.7718-4.2058 5.9894 5.9894 0 0 0 3.9977-2.9001 6.0557 6.0557 0 0 0-.7475-7.0729zm-9.022 12.6081a4.4755 4.4755 0 0 1-2.8764-1.0408l.1419-.0804 4.7783-2.7582a.7948.7948 0 0 0 .3927-.6813v-6.7369l2.02 1.1686a.071.071 0 0 1 .038.052v5.5826a4.504 4.504 0 0 1-4.4945 4.4944zm-9.6607-4.1254a4.4708 4.4708 0 0 1-.5346-3.0137l.142.0852 4.783 2.7582a.7712.7712 0 0 0 .7806 0l5.8428-3.3685v2.3324a.0804.0804 0 0 1-.0332.0615L9.74 19.9502a4.4992 4.4992 0 0 1-6.1408-1.6464zM2.3408 7.8956a4.485 4.485 0 0 1 2.3655-1.9728V11.6a.7664.7664 0 0 0 .3879.6765l5.8144 3.3543-2.0201 1.1685a.0757.0757 0 0 1-.071 0l-4.8303-2.7865A4.504 4.504 0 0 1 2.3408 7.872zm16.5963 3.8558L13.1038 8.364 15.1192 7.2a.0757.0757 0 0 1 .071 0l4.8303 2.7913a4.4944 4.4944 0 0 1-.6765 8.1042v-5.6772a.79.79 0 0 0-.407-.667zm2.0107-3.0231l-.142-.0852-4.7735-2.7818a.7759.7759 0 0 0-.7854 0L9.409 9.2297V6.8974a.0662.0662 0 0 1 .0284-.0615l4.8303-2.7866a4.4992 4.4992 0 0 1 6.6802 4.66zM8.3065 12.863l-2.02-1.1638a.0804.0804 0 0 1-.038-.0567V6.0742a4.4992 4.4992 0 0 1 7.3757-3.4537l-.142.0805L8.704 5.459a.7948.7948 0 0 0-.3927.6813zm1.0976-2.3654l2.602-1.4998 2.6069 1.4998v2.9994l-2.5974 1.4997-2.6067-1.4997Z" fill="#fff"/></g>',
  gh: '<path d="M12 3.5a8.5 8.5 0 0 0-2.7 16.6c.4.1.6-.2.6-.4v-1.5c-2.4.5-2.9-1-2.9-1-.4-1-1-1.3-1-1.3-.8-.5.1-.5.1-.5.9.1 1.3.9 1.3.9.8 1.3 2 1 2.5.7.1-.6.3-1 .6-1.2-1.9-.2-3.9-.9-3.9-4.2 0-.9.3-1.7.9-2.3-.1-.2-.4-1.1.1-2.3 0 0 .7-.2 2.3.9a8 8 0 0 1 4.2 0c1.6-1.1 2.3-.9 2.3-.9.5 1.2.2 2.1.1 2.3.6.6.9 1.4.9 2.3 0 3.3-2 4-3.9 4.2.3.3.6.8.6 1.6v2.4c0 .2.2.5.6.4A8.5 8.5 0 0 0 12 3.5z" fill="#fff"/>',
};

// An agent's mark: its glyph, white on its color.
export const agentMark = (k: string) => `<svg viewBox="0 0 24 24" aria-hidden="true">${GLYPHS[k] || ""}</svg>`;

// A company's badge: its logo if it has one, otherwise its first letter on its color.
// Graphite first, like the OneCorp icon, then muted tones that sit with the zinc theme.
export const PALETTE = ["#27272a", "#4772ae", "#ae5c47", "#47ae88", "#6d47ae", "#ae8847", "#ae477a", "#4799ae"];

export function badge(name: string, look?: Look | null, cls = "") {
  if (look?.logo) return `<span class="logo ${cls} img"><img src="${esc(look.logo)}" alt=""></span>`;
  // No name yet: a quiet building, not a question mark.
  const mark = name ? esc(name[0].toUpperCase()) : icon("company");
  return `<span class="logo ${cls}" style="background:${esc(look?.color || PALETTE[0])};color:#fff">${mark}</span>`;
}
