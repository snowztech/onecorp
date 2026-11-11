// Small helpers with no app state: the DOM, escaping, money and time.
// The elements asked for by id are known to be on the page; callers use ?. where one may be missing.
export const $ = <T extends HTMLElement = HTMLInputElement>(s: string) => document.querySelector(s) as T;

const ENTITIES: Record<string, string> = { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" };
export const esc = (s: unknown) => String(s ?? "").replace(/[&<>"]/g, c => ENTITIES[c]);

export const first = (n: string | null | undefined) => String(n || "").split(" ")[0];

let toastTimer: ReturnType<typeof setTimeout> | undefined;
export function toast(msg: string, isError?: boolean) {
  const t = $("#toast");
  t.textContent = msg;
  t.className = "toast show" + (isError ? " err" : "");
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => (t.className = "toast"), isError ? 5000 : 2600);
}

export const hash = (s: string) => [...String(s)].reduce((h, c) => (h * 31 + c.charCodeAt(0)) >>> 0, 7);

// You, without a photo: your initials. Drawn faces are for your AI team, so you never look like one of them.
export const initials = (name: string | null | undefined) => (String(name || "").trim().split(/\s+/).filter(Boolean).slice(0, 2).map(w => w[0]).join("") || "?").toUpperCase();

// Paths read shorter from your home: ~/dev/holt.
export const tilde = (p: string) => String(p).replace(/^\/Users\/[^/]+/, "~");

// Dollars the way people read them: cents under $100, whole dollars above.
export const tokensText = (n: number) => !n ? "no tokens" : n >= 1e6 ? `${(n / 1e6).toFixed(1)}M tokens` : n >= 1e3 ? `${Math.round(n / 1e3)}k tokens` : `${n} tokens`;

export const money = (x: number) => `$${x >= 100 ? Math.round(x).toLocaleString() : Number(x || 0).toFixed(2)}`;

// Agents park background output in long temp folders. Only the file name says anything.
export const shortPaths = (t: string) => String(t).replace(/(?:\/private)?\/tmp\/claude-[^\s"']*\/tasks\//g, "…/");

// The database keeps UTC. Shown in your own time.
export const utc = (at: string) => new Date(String(at).replace(" ", "T") + "Z");

export const hhmm = (at: string) => { const d = utc(at); return isNaN(d.getTime()) ? "" : d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }); };

export const fold = (t: string) => String(t).normalize("NFD").replace(/[\u0300-\u036f]/g, "").toLowerCase();

export const sameMinutes = (a: string, b: string, n: number) => Math.abs(utc(b).getTime() - utc(a).getTime()) <= n * 60000;
