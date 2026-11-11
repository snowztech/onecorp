// The drawn faces from the app (app/ui/app.js), so the landing page shows the same team.
const HUES = [215, 12, 158, 262, 38, 330, 192, 95];
const hash = s => [...String(s)].reduce((h, c) => (h * 31 + c.charCodeAt(0)) >>> 0, 7);
const SKIN = ["#f3d5bd", "#e6b590", "#c88e64", "#9a6542", "#6b442b"];
const HAIR = ["#1e1916", "#4b301e", "#8b5b2c", "#d0a660", "#b4442f", "#70747d"];
const drawn = {};
function drawFace(name, status, set = "ours") {
  if (set !== "ours" && window.Faces) {
    const key = set + "|" + name;
    drawn[key] ||= window.Faces(set, String(name));
    return `<span class="av set-${set}" style="--h:${HUES[(hash(name) >>> 12) % 8]}">${drawn[key]}</span>`;
  }
  const h = hash(name), L = { skin: h % 5, hair: (h >>> 3) % 6, style: (h >>> 6) % 5, acc: (h >>> 9) % 4 }, hue = HUES[(h >>> 12) % 8];
  const mood = { working: "working", blocked: "blocked", idle: "sleep", queued: "neutral", done: "happy" }[status] || "neutral";
  const sk = SKIN[L.skin], hr = HAIR[L.hair], ink = "#18181b";
  const cap = '<path d="M19 30q0-17 13-17t13 17q-3-9-13-9t-13 9z"/>';
  const back = L.style === 1 ? '<rect x="18" y="20" width="28" height="27" rx="9"/>' : "";
  const front = [cap, cap, '<circle cx="32" cy="12" r="5"/>' + cap,
    [[20, 22], [25, 17], [32, 15], [39, 17], [44, 22]].map(([x, y]) => `<circle cx="${x}" cy="${y}" r="5.5"/>`).join(""),
    '<path d="M20 25q12-12 24 0q-12-5-24 0z"/>'][L.style];
  const eyes = '<circle cx="27" cy="31" r="1.7"/><circle cx="37" cy="31" r="1.7"/>';
  const E = { happy: '<path d="M24.5 31q2.5-3 5 0M34.5 31q2.5-3 5 0" stroke-width="1.8" fill="none"/>', sleep: '<path d="M24.5 30.5h5M34.5 30.5h5" stroke-width="1.8"/>',
    blocked: eyes + '<path d="M24 26l5-2M40 26l-5-2" stroke-width="1.6"/>', working: eyes, neutral: eyes };
  const M = { happy: '<path d="M27 36q5 6 10 0z" fill="#fff" stroke-width="1.5"/>', sleep: '<path d="M30 37.5h4" stroke-width="1.6"/>',
    blocked: '<path d="M28 39q4-3.5 8 0" fill="none" stroke-width="1.6"/>', working: '<path d="M29 37.5h6" stroke-width="1.6"/>', neutral: '<path d="M28 36.5q4 3 8 0" fill="none" stroke-width="1.6"/>' };
  const A = ["", `<g fill="none" stroke="${ink}" stroke-width="1.5"><circle cx="27" cy="30.5" r="4.2"/><circle cx="37" cy="30.5" r="4.2"/><path d="M31.2 30.5h1.6"/></g>`,
    '<path d="M18 31a14 14 0 0 1 28 0" fill="none" stroke="#2f3440" stroke-width="3"/><rect x="15" y="27" width="5" height="10" rx="2" fill="#2f3440"/><rect x="44" y="27" width="5" height="10" rx="2" fill="#2f3440"/>',
    `<path d="M18.5 27q0-15 13.5-15t13.5 15z" fill="#3f3f46"/><rect x="18" y="24" width="28" height="5" rx="2.5" fill="#27272a"/>`][L.acc];
  return `<span class="av"><svg viewBox="0 0 64 64" aria-hidden="true"><rect width="64" height="64" style="fill:hsl(${hue} var(--av-tile))"/>
    <g fill="${hr}">${back}</g><path d="M11 64q2-17 21-17t21 17z" style="fill:hsl(${hue} var(--av-shirt))" stroke="${ink}" stroke-width="1.6"/><rect x="28.5" y="40" width="7" height="9" rx="2" fill="${sk}"/>
    <circle cx="32" cy="30" r="13" fill="${sk}" stroke="${ink}" stroke-width="1.6"/><g fill="${hr}">${front}</g><g fill="${ink}" stroke="${ink}" stroke-linecap="round">${E[mood]}${M[mood]}</g>${A}</svg></span>`;
}
