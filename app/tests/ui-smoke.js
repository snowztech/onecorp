// UI smoke test: runs app/ui/app.js with a stand-in DOM and its built-in sample data, and renders
// every screen. Catches errors a syntax check misses. Run: node app/tests/ui-smoke.js
import assert from "node:assert/strict";

const el = () => new Proxy({ classList: { toggle() {}, add() {}, remove() {} }, dataset: {}, style: {}, setAttribute() {}, focus() {}, querySelector: () => el(), matches: () => false, closest: () => null }, {
  get: (t, k) => (k in t ? t[k] : ""),
  set: (t, k, v) => { if (k === "innerHTML") t.writes = (t.writes || 0) + 1; t[k] = v; return true; },
});
const els = {};
const intervals = new Map();
let nextInterval = 1, requests = 0, command = (cmd, args) => preview(cmd, args);
globalThis.window = { __TAURI__: { core: { invoke: (cmd, args) => { requests++; return command(cmd, args); } } } };
globalThis.location = { hash: "" };
globalThis.document = { documentElement: el(), hasFocus: () => true, addEventListener() {}, querySelector: s => (els[s] ||= el()), getElementById: () => null, activeElement: null };
globalThis.setInterval = (callback, delay) => { const id = nextInterval++; intervals.set(id, { callback, delay }); return id; };
globalThis.clearInterval = id => intervals.delete(id);

// Imported after the stand-ins exist: the app starts loading as soon as it's evaluated.
const { S, preview, refresh, render, renderDrawer, renderPop, renderSheet, face, review, reviewActions, approvalCopy, drawerTicket,
  openTicket, openHire, openPerson, openSettings, startSetup, checkRepo } = await import("../ui/app.js");

try {
  await refresh(true);
  for (const n of ["Odile Martin", "Rui Alves", "Lina Park", "Sam Duarte", "Maya Okafor", "Tomás Reis", "Ada Lovelace"]) if (face(n, "neutral").includes("undefined")) throw new Error("broken face for " + n);
  assert.equal(intervals.size, 1, "company workspace starts one polling timer");
  const workspaceTimer = [...intervals.keys()][0];
  const wikiTicket = { key: "SNO-99", kind: "wiki", status: "review", title: "Write the wiki", name: "Odile Martin", log: [], document: "# Company wiki\n\n## Codebase\n\nA useful draft." };
  for (const theme of ["light", "dark"]) {
    document.documentElement.dataset.theme = theme;
    const card = review(wikiTicket);
    assert.match(card, /Save wiki/);
    assert.match(card, /A useful draft/);
    assert.doesNotMatch(card, /data-diff|Merge pull request|Approve and merge/);
    S.ticket = wikiTicket;
    const drawer = drawerTicket();
    assert.match(drawer, /Save wiki/);
    assert.doesNotMatch(drawer, /data-diff|Merge pull request|Approve and merge/);
    S.ticket = { ...wikiTicket, status: "done" };
    assert.match(drawerTicket(), /Completed/);
    assert.match(drawerTicket(), /Read wiki/);
    assert.doesNotMatch(drawerTicket(), /data-approve|Merged/);
  }
  assert.equal(approvalCopy({ kind: "plan" }).busy, "Starting…");
  assert.equal(approvalCopy(wikiTicket).busy, "Saving…");
  assert.match(reviewActions({ key: "SNO-11", kind: "work" }), /data-diff/);
  assert.doesNotMatch(reviewActions({ key: "SNO-20", kind: "plan" }), /data-diff/);
  S.ticket = null;

  for (const v of ["company", "board", "inbox", "chat", "wiki"]) { S.view = v; await refresh(true); console.log(v, document.querySelector("#view").innerHTML.length, "chars"); }
  assert.deepEqual([...intervals.keys()], [workspaceTimer], "renders reuse the workspace timer");
  await openTicket("SNO-11"); console.log("ticket drawer", document.querySelector("#drawer").innerHTML.length, "chars");
  S.drawer = { type: "new-ticket" }; renderDrawer(); console.log("new ticket drawer", document.querySelector("#drawer").innerHTML.length, "chars");
  openHire(1); console.log("hire drawer", document.querySelector("#drawer").innerHTML.length, "chars");
  await openPerson("tomas"); console.log("person drawer", document.querySelector("#drawer").innerHTML.length, "chars");
  S.drawer = null;
  S.profile = { name: "Lucas" }; render(); console.log("profile", document.querySelector("#view").innerHTML.length, "chars"); S.profile = null;
  assert.equal(intervals.size, 0, "welcome stops workspace polling");
  S.pop = "menu"; renderPop(); console.log("menu", document.querySelector("#pop").innerHTML.length, "chars"); S.pop = false;
  await openSettings(); console.log("settings sheet", document.querySelector("#sheet").innerHTML.length, "chars"); for (const t of ["spend", "agents", "you"]) { S.sheet = t; renderSheet(); console.log(t, document.querySelector("#sheet").innerHTML.length, "chars"); } S.sheet = null;
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
  assert.match(document.querySelector("#view").innerHTML, /id="profile"/, "first poll must render welcome");
  const preservesForm = async label => {
    assert.equal(intervals.size, 0, label + ": no polling timer during onboarding");
    const view = document.querySelector("#view"), writes = view.writes, before = requests;
    await refresh(false);
    assert.equal(requests, before, label + ": onboarding needs no background requests");
    assert.equal(view.writes, writes, label + ": polling must preserve input nodes and focus");
    await refresh(true);
    assert.ok(view.writes > writes, label + ": explicit refresh must still render");
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
} catch (e) {
  console.error("FAILED:", e);
  process.exit(1);
}
process.exit(0);
