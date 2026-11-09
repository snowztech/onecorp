// The OneCorp badge: an employee ID badge with a face. Moods: happy, sleep, wink.
function mascot(mood = "happy") {
  const ink = "#1d2230";
  const eyes = {
    happy: `<circle cx="26" cy="31" r="2.5" fill="${ink}"/><circle cx="38" cy="31" r="2.5" fill="${ink}"/>`,
    sleep: `<path d="M23.5 31.5h5M35.5 31.5h5" stroke="${ink}" stroke-width="2.2" stroke-linecap="round"/>`,
    wink: `<circle cx="26" cy="31" r="2.5" fill="${ink}"/><path d="M35.5 31.5q2.5-2.6 5 0" stroke="${ink}" stroke-width="2.2" stroke-linecap="round" fill="none"/>`,
  }[mood];
  const mouth = mood === "sleep" ? `<path d="M30 37.5h4" stroke="${ink}" stroke-width="2.2" stroke-linecap="round"/>` : `<path d="M27 36q5 5 10 0" stroke="${ink}" stroke-width="2.2" stroke-linecap="round" fill="none"/>`;
  return `<svg class="mascot" viewBox="0 0 64 64" aria-hidden="true">
    <defs><linearGradient id="oc-badge" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#4d76ea"/><stop offset="1" stop-color="#2447b8"/></linearGradient></defs>
    <rect x="25" y="1.5" width="14" height="11" rx="3.5" fill="#cfd5e1"/><rect x="28.5" y="4.5" width="7" height="3" rx="1.5" fill="#7d8496"/>
    <rect x="9" y="9" width="46" height="53" rx="13" fill="url(#oc-badge)"/>
    <rect x="26" y="13.5" width="12" height="3" rx="1.5" fill="#152d78" opacity=".6"/>
    <rect x="16.5" y="20" width="31" height="24" rx="9.5" fill="#fff"/>
    ${eyes}${mouth}
    <circle cx="21.5" cy="36" r="2.2" fill="#ff9d8a" opacity=".75"/><circle cx="42.5" cy="36" r="2.2" fill="#ff9d8a" opacity=".75"/>
    <rect x="19" y="49" width="26" height="3.6" rx="1.8" fill="#fff" opacity=".9"/><rect x="23.5" y="55" width="17" height="2.6" rx="1.3" fill="#fff" opacity=".45"/>
  </svg>`;
}
