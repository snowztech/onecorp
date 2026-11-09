// Walks the whole onboarding in a real browser, like a first user: types, clicks Continue, and
// checks every step lands where it should. Catches what unit tests miss, like a Continue that
// stays greyed out or a form that forgets what you typed.
//
// Needs Chrome and Node 22. Run: node app/tests/onboarding.mjs
// Skips with a note when Chrome isn't installed, so contributors without it aren't blocked.

import { spawn } from "node:child_process";
import { createServer } from "node:http";
import { existsSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { extname, join, normalize } from "node:path";
import assert from "node:assert/strict";

const CHROME = [
  process.env.CHROME,
  "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
  "/usr/bin/google-chrome",
  "/usr/bin/google-chrome-stable",
  "/usr/bin/chromium",
  "/usr/bin/chromium-browser",
].find(p => p && existsSync(p));
if (!CHROME) {
  console.log("onboarding walk-through skipped: no Chrome found (set CHROME to its path)");
  process.exit(0);
}

// The UI over HTTP, as a browser would load it.
const root = new URL("../ui/", import.meta.url).pathname;
const types = { ".html": "text/html", ".js": "text/javascript", ".css": "text/css" };
const server = createServer((req, res) => {
  const path = normalize(join(root, decodeURIComponent(new URL(req.url, "http://x").pathname)));
  if (!path.startsWith(root) || !existsSync(path)) return res.writeHead(404).end();
  res.writeHead(200, { "content-type": types[extname(path)] || "application/octet-stream" }).end(readFileSync(path));
});
await new Promise(r => server.listen(0, "127.0.0.1", r));
const url = `http://127.0.0.1:${server.address().port}/index.html#profile`;

const profile = mkdtempSync(join(tmpdir(), "onecorp-walk-"));
const chrome = spawn(CHROME, ["--headless=new", "--disable-gpu", "--no-first-run", "--no-default-browser-check", "--remote-debugging-port=0",
  `--user-data-dir=${profile}`, "--window-size=1280,800", url], { stdio: ["ignore", "ignore", "pipe"] });
const port = await new Promise((resolve, reject) => {
  let err = "";
  chrome.stderr.on("data", d => { err += d; const m = err.match(/DevTools listening on ws:\/\/[^:]+:(\d+)/); if (m) resolve(m[1]); });
  chrome.on("exit", code => reject(new Error(`Chrome exited (${code}): ${err.slice(-400)}`)));
});

let failed = false;
try {
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  let page;
  for (let i = 0; i < 50 && !page; i++) {
    page = (await (await fetch(`http://127.0.0.1:${port}/json`)).json()).find(t => t.type === "page" && t.url.startsWith(url.split("#")[0]));
    if (!page) await sleep(100);
  }
  const ws = new WebSocket(page.webSocketDebuggerUrl);
  let id = 0;
  const pending = {}, errors = [];
  ws.onmessage = m => {
    const d = JSON.parse(m.data);
    if (d.id) pending[d.id]?.(d);
    if (d.method === "Runtime.exceptionThrown") errors.push(d.params.exceptionDetails.exception?.description || d.params.exceptionDetails.text);
  };
  const send = (method, params = {}) => new Promise(r => { const i = ++id; pending[i] = r; ws.send(JSON.stringify({ id: i, method, params })); });
  await new Promise(r => (ws.onopen = r));
  await send("Runtime.enable");
  const js = async e => (await send("Runtime.evaluate", { expression: e, awaitPromise: true, returnByValue: true })).result.result?.value;
  // Waits for something to become true, so the test follows the app's pace instead of guessing it.
  const until = async (e, what) => {
    for (let i = 0; i < 60; i++) { if (await js(e)) return; await sleep(100); }
    throw new Error(`timed out waiting for ${what}`);
  };
  const type = async (sel, text) => { await js(`document.querySelector(${JSON.stringify(sel)}).focus()`); for (const ch of text) await send("Input.insertText", { text: ch }); };
  const click = sel => js(`document.querySelector(${JSON.stringify(sel)}).click()`);
  const step = () => js(`document.querySelector(".onb-steps li.on")?.textContent.replace(/^\\d+/, "").trim()`);
  const at = async name => { await until(`document.querySelector(".onb-steps li.on")?.textContent.includes(${JSON.stringify(name)})`, `the ${name} step`); };
  const cont = () => click("#view .su-nav .btn.primary");

  // You: typing survives the wait, and Continue goes on to Connect AI.
  await until(`document.querySelector("#pf-name")`, "the welcome");
  await type("#pf-name", "Ada Lovelace");
  await sleep(2000);
  assert.equal(await js(`document.querySelector("#pf-name").value`), "Ada Lovelace", "what you type stays put");
  assert.equal(await js(`document.activeElement?.id`), "pf-name", "the name field keeps focus");
  assert.equal(await js(`document.querySelector(".welcome .mono-av")?.textContent`), "AL", "your initials, not a drawn face");
  await cont();

  // Connect AI: each tool says where it stands.
  await at("Connect AI");
  const tools = await js(`[...document.querySelectorAll(".tool")].map(t => t.querySelector("b").textContent + "=" + t.querySelector(".pill-state").textContent).join(", ")`);
  assert.match(tools, /Claude Code=Connected/, "a connected agent shows as connected");
  await cont();

  // Company: the badge follows the name.
  await at("Company");
  await type("#su-name", "acme");
  assert.equal(await js(`document.querySelector(".co-card .logo").textContent`), "A");
  await cont();

  // Repo: Continue checks a typed path by itself.
  await at("Repo");
  await type("#su-path", "/Users/you/dev/acme");
  await cont();

  // Team: templates, removing someone removes them, and the team survives redraws.
  await at("Team");
  const templates = await js(`[...document.querySelectorAll(".tpl-card b")].map(b => b.textContent).join(", ")`);
  assert.ok(!/free|stand-in/i.test(templates), `no team without AI is offered: ${templates}`);
  await click('[data-tpl="startup"]');
  const names = () => js(`[...document.querySelectorAll(".person-row input[id$='-name']")].map(i => i.value).join(", ")`);
  assert.equal(await names(), "Odile Martin, Maya Okafor, Tomás Reis, Rui Alves, Lina Park, Sam Duarte");
  assert.equal(await js(`document.querySelectorAll(".person-row .agent-badge").length`), 6, "everyone shows their agent");
  await click('[data-su="del-1"]');
  assert.equal(await names(), "Odile Martin, Tomás Reis, Rui Alves, Lina Park, Sam Duarte", "the row you remove is the one that goes");
  await cont();

  // Launch: the summary, the setup sequence, then the company.
  await at("Launch");
  assert.equal(await js(`document.querySelector(".launch header b")?.textContent`), "acme");
  await cont();
  await until(`document.querySelector(".launching")`, "the launch sequence");
  await until(`document.querySelector("#title")?.textContent === "Company" && !document.querySelector(".onb")`, "the company page after launch");

  assert.deepEqual(errors, [], "no JavaScript errors along the way");
  ws.close();
  console.log("onboarding walk-through: You, Connect AI, Company, Repo, Team, Launch, Company page");
} catch (e) {
  failed = true;
  console.error("onboarding walk-through FAILED:", e.message);
} finally {
  // Chrome keeps writing its profile until it has really exited, so wait for that before cleaning up.
  const gone = new Promise(r => chrome.once("exit", r));
  chrome.kill();
  await Promise.race([gone, new Promise(r => setTimeout(r, 3000))]);
  server.close();
  try { rmSync(profile, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 }); } catch {}
}
process.exit(failed ? 1 : 0);
