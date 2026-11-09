// UI smoke test: runs app/ui/app.js with a stand-in DOM and its built-in sample data, and renders
// every screen. Catches errors a syntax check misses. Run: node app/tests/ui-smoke.js
const el = () => new Proxy({ classList: { toggle() {}, add() {}, remove() {} }, dataset: {}, style: {}, setAttribute() {}, focus() {}, querySelector: () => el(), matches: () => false, closest: () => null }, {
  get: (t, k) => (k in t ? t[k] : ""),
  set: (t, k, v) => { if (k === "innerHTML") t.writes = (t.writes || 0) + 1; t[k] = v; return true; },
});
const els = {};
global.window = {};
global.location = { hash: "" };
global.document = { documentElement: el(), hasFocus: () => true, addEventListener() {}, querySelector: s => (els[s] ||= el()), getElementById: () => null, activeElement: null };
const intervals = new Map();
let nextInterval = 1;
global.setInterval = (callback, delay) => { const id = nextInterval++; intervals.set(id, { callback, delay }); return id; };
global.clearInterval = id => intervals.delete(id);
const read = f => require("fs").readFileSync(require("path").join(__dirname, "../ui", f), "utf8");
const src = read("brand.js") + "\n" + read("app.js");
eval(`let requests = 0, command = (cmd, args) => preview(cmd, args);
window.__TAURI__ = { core: { invoke: (cmd, args) => { requests++; return command(cmd, args); } } };
` + src + `
(async () => {
  await refresh(true);
  for (const n of ["Odile Martin", "Rui Alves", "Lina Park", "Sam Duarte", "Maya Okafor", "Tomás Reis", "Ada Lovelace"]) if (face(n, "neutral").includes("undefined")) throw new Error("broken face for " + n);
  const assert = require("node:assert/strict");
  assert.equal(intervals.size, 1, "company workspace starts one polling timer");
  const workspaceTimer = [...intervals.keys()][0];
  for (const v of ["company", "board", "inbox", "chat", "wiki"]) { S.view = v; await refresh(true); console.log(v, document.querySelector("#view").innerHTML.length, "chars"); }
  assert.deepEqual([...intervals.keys()], [workspaceTimer], "renders reuse the workspace timer");
  await openTicket("SNO-11"); console.log("ticket drawer", document.querySelector("#drawer").innerHTML.length, "chars");
  S.drawer = { type: "new-ticket" }; renderDrawer(); console.log("new ticket drawer", document.querySelector("#drawer").innerHTML.length, "chars");
  openHire(1); console.log("hire drawer", document.querySelector("#drawer").innerHTML.length, "chars");
  await openPerson("tomas"); console.log("person drawer", document.querySelector("#drawer").innerHTML.length, "chars");
  S.drawer = null;
  S.profile = { name: "Lucas" }; render(); console.log("profile", document.querySelector("#view").innerHTML.length, "chars"); S.profile = null;
  assert.equal(intervals.size, 0, "welcome stops workspace polling");
  S.pop = "menu"; renderPop(); console.log("menu", document.querySelector("#pop").innerHTML.length, "chars"); S.pop = "profile"; renderPop(); console.log("profile popover", document.querySelector("#pop").innerHTML.length, "chars"); S.pop = false;
  await openSettings(); console.log("settings sheet", document.querySelector("#sheet").innerHTML.length, "chars"); S.sheet = "health"; renderSheet(); console.log("setup check", document.querySelector("#sheet").innerHTML.length, "chars"); S.sheet = null;
  await startSetup();
  await checkRepo("/Users/you/dev/snowztech");
  for (let step = 0; step < 5; step++) { S.setup.step = step; render(); console.log("setup step", step, document.querySelector("#view").innerHTML.length, "chars"); }
  console.log("name suggested:", S.setup.name, "| template:", S.setup.template, "| runtime:", S.setup.runtime);
  // A fresh install has no company, unlike the default sample data. Polling must not
  // replace its onboarding DOM: replacing an input drops focus even if its value survives.
  const sample = preview;
  command = (cmd, args) => cmd === "companies" ? Promise.resolve({ list: [], current: null }) : sample(cmd, args);
  S.company = null; S.founder = null; S.profile = null; S.setup = null; S.shown = "";
  await refresh(false);
  require("node:assert/strict").match(document.querySelector("#view").innerHTML, /id="profile"/, "first poll must render welcome");
  const preservesForm = async label => {
    assert.equal(intervals.size, 0, label + ": no polling timer during onboarding");
    const view = document.querySelector("#view"), writes = view.writes, before = requests;
    await refresh(false);
    require("node:assert/strict").equal(requests, before, label + ": onboarding needs no background requests");
    require("node:assert/strict").equal(view.writes, writes, label + ": polling must preserve input nodes and focus");
    await refresh(true);
    require("node:assert/strict").ok(view.writes > writes, label + ": explicit refresh must still render");
  };
  for (const value of ["", "L", "Lucas "]) {
    document.querySelector("#pf-name").value = value;
    await preservesForm("welcome " + JSON.stringify(value));
  }
  S.profile = null;
  await startSetup();
  await checkRepo("/Users/you/dev/snowztech");
  for (let step = 0; step < 5; step++) {
    S.setup.step = step; render();
    await preservesForm("first company step " + step);
  }
  // Leaving setup (launch or cancel) resumes live workspace updates, once only.
  command = sample; S.setup = null; S.founder = { name: "Lucas" }; S.view = "company";
  await refresh(true);
  assert.equal(intervals.size, 1, "leaving setup restarts polling");
  const timer = [...intervals.values()][0], before = requests;
  assert.equal(timer.delay, 1500);
  await timer.callback();
  assert.ok(requests > before, "workspace polling fetches updates");
  await startSetup();
  assert.equal(intervals.size, 0, "adding another company stops polling too");
  S.setup = null; await refresh(true);
  assert.equal(intervals.size, 1, "cancelling setup resumes one timer");
  console.log("onboarding has no polling timer; workspace polling resumes on exit");
})().catch(e => { console.error("FAILED:", e); process.exit(1); });`);
