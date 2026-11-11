// Small helpers with no app state: the DOM, escaping, money and time.
export const $ = s => document.querySelector(s);

export const esc = s => String(s ?? "").replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]));

export const first = n => String(n || "").split(" ")[0];

export function toast(msg, isError) {
  const t = $("#toast");
  t.textContent = msg;
  t.className = "toast show" + (isError ? " err" : "");
  clearTimeout(toast.h);
  toast.h = setTimeout(() => (t.className = "toast"), isError ? 5000 : 2600);
}

export const hash = s => [...String(s)].reduce((h, c) => (h * 31 + c.charCodeAt(0)) >>> 0, 7);

// You, without a photo: your initials. Drawn faces are for your AI team, so you never look like one of them.
export const initials = name => (String(name || "").trim().split(/\s+/).filter(Boolean).slice(0, 2).map(w => w[0]).join("") || "?").toUpperCase();

// Paths read shorter from your home: ~/dev/holt.
export const tilde = p => String(p).replace(/^\/Users\/[^/]+/, "~");

// Dollars the way people read them: cents under $100, whole dollars above.
export const tokensText = n => !n ? "no tokens" : n >= 1e6 ? `${(n / 1e6).toFixed(1)}M tokens` : n >= 1e3 ? `${Math.round(n / 1e3)}k tokens` : `${n} tokens`;

export const money = x => `$${x >= 100 ? Math.round(x).toLocaleString() : Number(x || 0).toFixed(2)}`;

// Agents park background output in long temp folders. Only the file name says anything.
export const shortPaths = t => String(t).replace(/(?:\/private)?\/tmp\/claude-[^\s"']*\/tasks\//g, "…/");

// The database keeps UTC. Shown in your own time.
export const utc = at => new Date(String(at).replace(" ", "T") + "Z");

export const hhmm = at => { const d = utc(at); return isNaN(d) ? "" : d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }); };

export const fold = t => String(t).normalize("NFD").replace(/[\u0300-\u036f]/g, "").toLowerCase();

export const sameMinutes = (a, b, n) => Math.abs(utc(b) - utc(a)) <= n * 60000;
