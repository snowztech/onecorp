// UI smoke test: runs app/ui/app.js with a stand-in DOM and its built-in sample data, and renders
// every screen. Catches errors a syntax check misses. Run: node app/tests/ui-smoke.js
const el = () => new Proxy({ classList: { toggle() {}, add() {}, remove() {} }, dataset: {}, style: {}, setAttribute() {}, focus() {}, querySelector: () => el(), matches: () => false, closest: () => null }, {
  get: (t, k) => (k in t ? t[k] : ""), set: (t, k, v) => ((t[k] = v), true),
});
const els = {};
global.window = {};
global.document = { addEventListener() {}, querySelector: s => (els[s] ||= el()), getElementById: () => null, activeElement: null };
global.setInterval = () => {};
const src = require("fs").readFileSync(require("path").join(__dirname, "../ui/app.js"), "utf8");
eval(src + `
(async () => {
  await refresh(true);
  for (const v of ["company", "board", "inbox", "handbook"]) { S.view = v; await refresh(true); console.log(v, document.querySelector("#view").innerHTML.length, "chars"); }
  await openTicket("SNO-11"); console.log("ticket drawer", document.querySelector("#drawer").innerHTML.length, "chars");
  await openPerson("tomas"); console.log("person drawer", document.querySelector("#drawer").innerHTML.length, "chars");
  S.drawer = null;
  S.profile = { name: "Lucas" }; render(); console.log("profile", document.querySelector("#view").innerHTML.length, "chars"); S.profile = null;
  await startSetup();
  await checkRepo("/Users/you/dev/snowztech");
  for (let step = 0; step < 4; step++) { S.setup.step = step; render(); console.log("setup step", step, document.querySelector("#view").innerHTML.length, "chars"); }
  console.log("name suggested:", S.setup.name, "| template:", S.setup.template, "| runtime:", S.setup.runtime);
})().catch(e => { console.error("FAILED:", e); process.exit(1); });`);
