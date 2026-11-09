// The OneCorp badge: an employee ID badge on its clip, with a face cut out of it. One color
// (currentColor), so it matches whatever it sits on. Moods: happy, sleep, wink.
function mascot(mood = "happy") {
  const id = "oc-" + mood;
  const face = {
    happy: '<rect x="11.4" y="13.6" width="2.6" height="4.2" rx="1.3"/><rect x="18" y="13.6" width="2.6" height="4.2" rx="1.3"/><path d="M12.6 21q3.4 2.8 6.8 0" stroke="#000" stroke-width="1.9" stroke-linecap="round" fill="none"/>',
    sleep: '<rect x="10.8" y="15.4" width="4" height="1.8" rx=".9"/><rect x="17.2" y="15.4" width="4" height="1.8" rx=".9"/><rect x="14.6" y="21" width="2.8" height="1.8" rx=".9"/>',
    wink: '<rect x="11.4" y="13.6" width="2.6" height="4.2" rx="1.3"/><path d="M17.6 16.4q1.7-2 3.4 0" stroke="#000" stroke-width="1.9" stroke-linecap="round" fill="none"/><path d="M12.6 21q3.4 2.8 6.8 0" stroke="#000" stroke-width="1.9" stroke-linecap="round" fill="none"/>',
  }[mood];
  return `<svg class="mascot" viewBox="0 0 32 32" aria-hidden="true"><defs><mask id="${id}"><rect width="32" height="32" fill="#fff"/>
      <rect x="14.4" y="2.9" width="3.2" height="1.6" rx=".8" fill="#000"/><rect x="13" y="8.2" width="6" height="1.7" rx=".85" fill="#000"/><g fill="#000">${face}</g><rect x="10.5" y="25.2" width="11" height="1.7" rx=".85" fill="#000"/></mask></defs>
    <g fill="currentColor" mask="url(#${id})"><rect x="12.6" y="1.6" width="6.8" height="5" rx="1.7"/><rect x="6" y="5" width="20" height="25.4" rx="6"/></g></svg>`;
}
