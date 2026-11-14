import { openHire, openPerson, openTicket, renderDrawer } from "./drawers.ts";
import { $ } from "./lib.ts";
import { refresh, render } from "./render.ts";
import { renderPop, renderSide, toggleRail } from "./shell.ts";
import { S, invoke } from "./state.ts";
import type { Founder } from "./types.ts";
import { checkRepo, setTemplate, startSetup } from "./views/onboarding.ts";
import { openSettings } from "./views/settings.ts";

const BACKEND = "- Read the handlers, models and tests around a change first, and follow how errors, logging and config are already done.\n- Keep APIs backward compatible. If a change would break a client, say so and ask before making it.\n- Schema changes go in a new migration that's safe on live data. Never edit an old one.\n- Validate input at the edges, and never log secrets, tokens or personal data.\n- Watch for queries in loops, missing indexes and lists with no limit.\n- Every endpoint or job you change gets a test, the failure path included.\n- In your summary, call out migrations, config or environment changes someone has to apply.";

// In a plain browser, #board, #inbox, #wiki, #ticket, #person, #new-ticket, #setup or #profile opens
// straight onto that screen, so each one can be looked at and screenshotted. Never used in the app.
export async function previewScreen() {
  const screen = location.hash.slice(1);
  // ?light or ?dark, for screenshots in either theme.
  if (/[?&](light|dark)\b/.test(location.search)) document.documentElement.dataset.theme = location.search.includes("light") ? "light" : "dark";
  await refresh(true);
  if (screen === "settings") await openSettings();
  if (screen === "switcher") { S.switching = true; renderSide(); }
  if (screen === "plan") {
    S.view = "board"; S.drawer = { type: "plan" }; render();
    $("#pj-name").value = "Stripe billing";
    $("#pj-goal").value = "A Pro plan at $6 a month next to the free tier. Checkout with Stripe, webhooks that survive retries, and a billing page to see invoices or cancel.";
  }
  if (screen === "rail") toggleRail(true);
  if (screen === "start") { S.snap!.tickets = []; S.snap!.projects = []; S.snap!.kickoff.brief = ""; render(); }
  if (screen === "wiki-edit") { S.view = "wiki"; S.hbEdit = true; await refresh(true); render(); }
  if (screen === "wiki-search") { S.view = "wiki"; S.wikiQ = "magic"; S.page = "Company/Decisions"; await refresh(true); }
  if (screen === "wiki-page") { S.view = "wiki"; S.page = "Product/Customers"; await refresh(true); }
  if (screen === "wiki-drafting") { S.view = "wiki"; S.page = "Engineering/Codebase"; await refresh(true); S.snap!.tickets.push({ key: "SNO-40", title: "Write the Engineering pages", assignee: "odile", status: "doing", spend: "$0.04", after: null, kind: "wiki", repo: null, created_by: null, plan: null } as never, { key: "SNO-41", title: "Write the Product pages", assignee: "rui", status: "queued", spend: "$0.00", after: null, kind: "wiki", repo: null, created_by: null, plan: null } as never); render(); }
  if (screen === "wiki-new") { S.view = "wiki"; S.wikiNew = true; S.wikiSpace = "Product"; await refresh(true); render(); }
  if (screen === "wiki-start") { S.view = "wiki"; S.page = "Company/Now"; S.snap!.tickets = S.snap!.tickets.filter(t => t.kind !== "wiki"); render(); }
  if (screen === "wiki-empty") { S.view = "wiki"; S.page = "Company/Now"; await refresh(true); }
  if (screen === "wiki-history") { S.view = "wiki"; S.page = "Company/Decisions"; await refresh(true); S.history = await invoke("page_history", { company: "snowztech", path: "Company/Decisions" }); S.historyOf = "Company/Decisions"; S.version = 3; render(); }
  if (screen === "inbox-zero") { S.snap!.inbox = []; S.snap!.reviews = []; S.snap!.failed = []; S.snap!.with_leads = []; S.view = "inbox"; render(); }
  if (screen === "launching") { await startSetup(); S.setup!.name = "snowztech"; await checkRepo("/Users/you/dev/snowztech"); S.setup!.launch = { done: 2, steps: ["Founding snowztech", "Hiring 3 people", "Connecting snowztech", "Odile starts writing the wiki"] }; render(); }
  if (screen === "connect") await startSetup();
  if (screen === "team") { await startSetup(); setTemplate("startup"); S.setup!.step = 3; render(); }
  if (screen === "team-playbook") { await startSetup(); setTemplate("startup"); S.roles = await invoke("roles"); S.setup!.people[0].pb = "reviewer"; S.setup!.step = 3; render(); }
  if (screen === "dm") { S.view = "chat"; S.channel = "dm:tomas"; await refresh(true); }
  if (screen === "edit-avatar") { await openPerson("tomas"); S.drawer!.edit = true; renderDrawer(); }
  if (screen === "spend") await openSettings("spend");
  if (screen === "paused") { S.snap!.paused = true; render(); }
  if (screen === "hire") openHire(1);
  if (screen === "person-edit") { await openPerson("tomas"); S.drawer!.edit = true; renderDrawer(); }
  if (screen === "person-rewrite") { await openPerson("tomas"); S.drawer!.edit = true; renderDrawer(); S.drawer!.form!.rewriting = true; S.drawer!.form!.rewrite = "- Ship small.\n- Go, with table tests."; renderDrawer(); }
  if (screen === "idle") { S.snap!.employees.forEach(e => (e.status = "idle")); S.snap!.inbox = []; S.snap!.reviews = []; S.snap!.failed = []; render(); }
  if (screen === "project") { S.view = "board"; S.project = "SNO-20"; render(); }
  if (["board", "inbox", "wiki", "chat"].includes(screen)) { S.view = screen; render(); }
  if (screen === "ticket") openTicket("SNO-11");
  if (screen === "diff") { S.diffs["SNO-11"] = await invoke("review", { company: S.company, key: "SNO-11" }); await openTicket("SNO-11"); }
  if (screen === "person") openPerson("tomas");
  if (screen === "edit-person") { await openPerson("tomas"); S.drawer!.edit = true; S.person!.runtime = "codex"; renderDrawer(); }
  if (screen === "new-ticket") { S.view = "board"; S.drawer = { type: "new-ticket" }; render(); }
  if (screen === "setup") await startSetup();
  if (screen === "repos") { await startSetup(); S.setup!.name = "snowztech"; await checkRepo("/Users/you/dev/notes"); S.setup!.extra = [{ path: "/Users/you/dev/notes-api", name: "notes-api", branch: "feat/search" }]; S.setup!.step = 2; render(); }
  if (screen === "github-pick") { await startSetup(); S.setup!.name = "snowztech"; S.setup!.step = 2; S.setup!.mode = "github"; S.setup!.gh = { repos: await invoke("github_repos") }; S.setup!.pick = { name: "snowztech/notes-api", path: "/Users/you/dev/notes-api" }; render(); }
  if (screen === "github") { await startSetup(); S.setup!.name = "snowztech"; S.setup!.step = 2; S.setup!.mode = "github"; S.setup!.gh = { repos: await invoke("github_repos") }; render(); }
  if (screen.startsWith("setup-")) { await startSetup(); S.setup!.name = "snowztech"; S.setup!.mission = "Open source tools for building AI agents"; await checkRepo("/Users/you/dev/snowztech"); S.setup!.step = Number(screen.slice(6)); render(); }
  if (screen === "profile") { S.founder = null; S.companies = []; S.profile = { name: "" }; render(); }
  if (screen === "welcome") { S.founder = null; S.companies = []; S.welcome = true; S.profile = { name: "" }; render(); }
  if (screen === "welcome-back") { S.founder = null; S.profile = { name: "" }; render(); }
  if (["popover", "you", "agents", "appearance", "about"].includes(screen)) await openSettings(screen === "popover" ? "you" : screen);
  if (screen === "menu") { S.pop = "menu"; renderPop(); }
  if (screen.startsWith("faces-")) { try { localStorage.setItem("onecorp.faces", screen.slice(6)); } catch {} render(); }
  if (screen === "limit") { S.snap!.limits = [{ runtime: "claude", back: "15:00" }]; render(); }
  if (screen === "health") await openSettings("health");
}

/* Sample data for the UI in a plain browser (make ui). Never used inside the app. */
export let previewFounder: Founder | null = null;

export function preview(cmd: string, args: Record<string, string> = {}): Promise<unknown> {
  const team = [
    ["odile", "Odile Martin", "CTO", "claude", "opus", "working", "Reading the SNO-12 diff", 1], ["maya", "Maya Okafor", "Senior engineer", "codex", null, "blocked", "Need you on SNO-12", 1],
    ["tomas", "Tomás Reis", "Engineer", "codex", null, "working", "Bash cargo test billing", 1], ["rui", "Rui Alves", "Product lead", "claude", "sonnet", "working", "Created SNO-18", 2],
    ["lina", "Lina Park", "Product designer", "claude", "sonnet", "queued", "Up next: SNO-14", 2], ["sam", "Sam Duarte", "Writer", "claude", "haiku", "idle", "", 3],
  ].map(([id, name, role, runtime, model, status, bubble, department]) => ({ id, name, role, runtime, model, status, bubble, department, spend: "$0.40", ticket: null }));
  const VOICES = [["terse","Terse","Done. Login works, tests pass."],["warm","Warm","Login's in! Thanks for the clear ticket. Tests are green."],["thorough","Thorough","Login is in. I went with magic links over passwords: less to store. Tests cover both paths."],["dry","Dry","Login works. The tests agree, which is a first this week."],["blunt","Blunt","Login works. The old session code was broken, so I replaced it."],["curious","Curious","Login works. Should links expire after 15 minutes or an hour?"],["upbeat","Upbeat","Login shipped! On to signup next."],["calm","Calm","Login is done. 1. Link sent. 2. Token checked. 3. Session set."]].map(([key, label, sample]) => ({ key, label, sample }));
  const data: Record<string, unknown> = {
    voices: VOICES,
    models: { claude: [["claude-opus-5-5", "Opus 5.5"], ["claude-sonnet-5-5", "Sonnet 5.5"], ["claude-haiku-5-5", "Haiku 5.5"], ["claude-fable-5-1", "Fable 5.1"]].map(([id, name]) => ({ id, name, about: "" })), codex: [["gpt-6.1-sol", "GPT-6.1-Sol"], ["gpt-6-astra", "GPT-6-Astra"], ["gpt-6-luna", "GPT-6-Luna"]].map(([id, name]) => ({ id, name, about: "" })),
      tiers: { claude: { strongest: { id: "claude-opus-5-5", name: "Opus 5.5" }, balanced: { id: "claude-sonnet-5-5", name: "Sonnet 5.5" }, cheapest: { id: "claude-haiku-5-5", name: "Haiku 5.5" } },
        codex: { strongest: { id: "gpt-6-astra", name: "GPT-6-Astra" }, balanced: { id: "gpt-6.1-sol", name: "GPT-6.1-Sol" }, cheapest: { id: "gpt-6-luna", name: "GPT-6-Luna" } } },
      defaults: { claude: "Opus 5.5", codex: "GPT-6-Astra" } },
    companies: { list: ["snowztech", "tvcask", "autoremind", "pingbuddy"], current: "snowztech", looks: { snowztech: { color: "#4772ae" }, tvcask: { color: "#ae5c47" }, autoremind: { color: "#47ae88" }, pingbuddy: { color: "#27272a" } } },
    snapshot: {
      repos: [{ name: "notes", path: "/Users/you/dev/notes", main: true }, { name: "notes-web", path: "/Users/you/dev/notes-web", main: false }],
      company: "snowztech", serve: true, kickoff: { brief: "A notes app for writers.", planner: "rui", planned: true, planning: [] }, ship: { mode: "github", forge: "GitHub", pr_word: "pull request" }, employees: team, tickets: [
        { key: "SNO-9", plan: "SNO-20", title: "Rate limit the public API", assignee: "tomas", status: "failed", spend: "40k tok" },
        { key: "SNO-11", plan: "SNO-20", title: "Pricing page", assignee: "odile", status: "review", spend: "$0.31" },
        { key: "SNO-12", plan: "SNO-20", title: "Magic link login", assignee: "maya", status: "blocked", spend: "52k tok" },
        { key: "SNO-14", repo: "notes-web", plan: "SNO-20", title: "Empty state for the notes list", assignee: "lina", status: "queued", spend: "$0.00" },
        { key: "SNO-15", plan: "SNO-20", title: "Retry failed Stripe webhooks", assignee: "tomas", status: "doing", spend: "75k tok" },
        { key: "SNO-8", plan: "SNO-20", title: "CI pipeline", assignee: "tomas", status: "done", spend: "$0.22" },
        { key: "SNO-16", title: "Export notes as Markdown", assignee: null, status: "backlog", spend: "$0.00", created_by: "Rui Alves" },
        { key: "SNO-17", title: "Notes API pagination", assignee: "maya", status: "waiting", spend: "$0.00", after: "SNO-12" }],
      projects: [{ key: "SNO-20", title: "First version", goal: "", status: "done", planner: "rui", planner_name: "Rui Alves", total: 8, done: 5 }, { key: "SNO-30", title: "Stripe billing", goal: "Pro plan at $6 a month with a free tier.", status: "review", planner: "rui", planner_name: "Rui Alves", total: 4, done: 0 }],
      departments: [{ id: 1, name: "Engineering", lead: "odile", members: ["odile", "maya", "tomas"] }, { id: 2, name: "Product", lead: "rui", members: ["rui", "lina"] }, { id: 3, name: "Growth", lead: "sam", members: ["sam"] }],
      with_leads: [{ id: 5, kind: "question", key: "SNO-15", employee: "tomas", name: "Tomás Reis", lead_name: "Odile Martin", question: "Retry webhooks 3 or 5 times?", section: "", options: ["3", "5"], default: "3", blocking: false }],
      // A question with a long default and no options, like most real ones.
      inbox_extra: null,
      inbox: [{ id: 9, kind: "command", key: "SNO-21", employee: "tomas", name: "Tomás Reis", question: "xcodegen generate", repo: "notes", note: "the Xcode project is generated from project.yml, and I need it to build the iOS target", section: "", options: [], default: "", blocking: true },
        { id: 10, kind: "wiki", key: "SNO-11", employee: "odile", name: "Odile Martin", section: "Engineering/Conventions", question: "Every API handler validates its input with the shared schema in api/schema.ts before touching the database. Errors go back as { error: { code, message } } with a 4xx status, never a 500 for bad input. Handlers stay thin: parsing, one call into the service layer, and the response. Anything with business rules lives in services/, with its tests next to it. When a handler needs data from two services, it calls a third one that composes them rather than calling both itself.", options: [], default: "", blocking: false },
        { id: 7, kind: "question", key: "SNO-20", employee: "rui", name: "Rui Alves", question: "Which K-drama and region did you see Viki missing on? The API already pulls TMDB's subscription tiers, so I suspect a data gap, not a filter on our side.", section: "", options: [], default: "I'll plan with an investigation ticket first, using popular K-dramas in US, KR and FR.", blocking: false }, { id: 2, kind: "question", key: "SNO-12", employee: "maya", name: "Maya Okafor", question: "Magic links: 15 or 60 minute expiry?", section: "", options: ["15 minutes", "60 minutes"], default: "15 minutes", blocking: true, via: "odile", via_name: "Odile Martin", note: "it changes what users see, so it's a product call" },
        { id: 3, kind: "wiki", key: "SNO-14", employee: "lina", name: "Lina Park", question: "Empty states are one sentence and one button", section: "Product/Voice", options: [], default: "", blocking: false }],
      failed: [{ key: "SNO-9", title: "Rate limit the public API", employee: "tomas", name: "Tomás Reis", why: "stopped before it finished" }],
      reviews: [{ key: "SNO-20", title: "First version", employee: "rui", name: "Rui Alves", kind: "plan", summary: "Setup first, then login and sync in parallel. Risk: magic links need an email provider.", spend: "$0.21",
          tickets: [{ key: "SNO-21", title: "Set up the project", description: "A skeleton that builds, with a README.\n\nDone looks like:\n- `cargo build` and `cargo test` pass\n- A README that says how to run it\n- CI runs the tests on every push", assignee: "maya", name: "Maya Okafor", after: null, status: "backlog" },
            { key: "SNO-22", title: "Magic link login", description: "Email a one-time link, 15 minute expiry.", assignee: "tomas", name: "Tomás Reis", after: "SNO-21", status: "backlog" }] },
        { key: "SNO-30", kind: "report", title: "Audit our SEO", employee: "lina", name: "Lina Park", summary: "", spend: "$0.18",
          document: "## What I found\n\nThe landing page ranks for our name and little else. Three things hold it back:\n\n1. **No meta descriptions.** Google writes its own from the nav.\n2. **One page for every audience.** Writers searching for *distraction-free notes* land on a page about sync.\n3. **Slow first load** on mobile: 3.8s, mostly the hero video.\n\n## What I'd do\n\nThe three tickets below, in that order. The first is an hour of work.",
          tickets: [{ key: "SNO-31", title: "Add meta descriptions to every page", description: "One line per page, under 155 characters.", assignee: "lina", name: "Lina Park", after: null, status: "backlog" },
            { key: "SNO-32", title: "A landing page for writers", description: "", assignee: "rui", name: "Rui Alves", after: null, status: "backlog" },
            { key: "SNO-33", title: "Lazy-load the hero video", description: "", assignee: "maya", name: "Maya Okafor", after: null, status: "backlog" }] },
        { key: "SNO-11", kind: "work", tickets: [], title: "Pricing page", employee: "odile", name: "Odile Martin", summary: "Added the pricing page with Free and Pro plans, reusing `PlanCard` from settings. `cargo test` and `pnpm lint` pass. Check the copy on mobile.", spend: "$0.31", pr_number: 41, pr_url: "https://github.com/snowztech/notes/pull/41" }],
      feed: [{ at: "2026-11-09 11:02:00", key: "SNO-11", employee: "odile", name: "Odile Martin", kind: "done", summary: "finished, $0.31", title: "Pricing page" }, { at: "2026-11-09 10:52:00", key: "SNO-15", employee: "tomas", name: "Tomás Reis", kind: "start", summary: "started", title: "Retry failed Stripe webhooks" }, { at: "2026-11-09 10:31:00", key: "SNO-12", employee: "maya", name: "Maya Okafor", kind: "block", summary: "asked you about magic link expiry", title: "Magic link login" }],
      spend: { cost: 3.24, tokens: 180000, label: "$3.24", today: 3.24, today_label: "$3.24", budget: 5, warn: false, paused_by_budget: false },
      dms: [["odile", 12], ["maya", null], ["tomas", 14], ["rui", null], ["lina", null], ["sam", null]].map(([id, last]) => ({ id, channel: "dm:" + id, last })),
      channels: [{ name: "general", title: "General", department: null, last: 9 }, { name: "engineering", title: "Engineering", department: 1, last: 8 }, { name: "product", title: "Product", department: 2, last: 4 }, { name: "growth", title: "Growth", department: 3, last: null }],
    },
    wiki: { recent: [], rules_tokens: 4200, pages: [
      { path: "Company/Overview", space: "Company", title: "Overview", text: "## Why we exist\n\nWriters lose work to tools built for everyone else. We make the one built for them.\n\n## What we're building\n\nA notes app for writers: fast, offline first, synced across devices.\n\n## Where we are\n\nIn beta with 40 writers. Login and notes are live, sync ships next.", edits: 2, by: "You", first_by: "Rui Alves", at: "2026-11-08 16:20:00", note: "" },
      { path: "Company/Now", space: "Company", title: "Now", text: "", edits: 0, by: null, first_by: null, at: null, note: null },
      { path: "Company/Decisions", space: "Company", title: "Decisions", text: "- Standard library first. A new dependency needs a reason in the PR.\n- Magic links expire after 15 minutes.\n- No email reminders: SMS only.", edits: 4, by: "Odile Martin", first_by: "You", at: "2026-11-10 10:05:00", note: "proposed it, and you added it" },
      { path: "Product/Customers", space: "Product", title: "Customers", text: "## Who\n\nNovelists and journalists who write every day and switch between a laptop and a phone.\n\n## What they care about\n\n- Never losing a word, even offline\n- A screen with nothing but the text\n- Export to Markdown and Word", edits: 1, by: "Rui Alves", first_by: "Rui Alves", at: "2026-11-09 11:30:00", note: "wrote the first draft" },
      { path: "Product/Rules", space: "Product", title: "Rules", text: "- Notes are private. Nothing leaves the device unencrypted.\n- Free plan: 3 devices. Pro: unlimited.", edits: 1, by: "Rui Alves", first_by: "Rui Alves", at: "2026-11-09 11:30:00", note: "wrote the first draft" },
      { path: "Product/Voice", space: "Product", title: "Voice", text: "", edits: 0, by: null, first_by: null, at: null, note: null },
      { path: "Product/Glossary", space: "Product", title: "Glossary", text: "- **Note**: one document. A writer has many.\n- **Notebook**: a folder of notes, shared or private.\n- **Device**: a signed-in app. Free allows 3.", edits: 1, by: "Rui Alves", first_by: "Rui Alves", at: "2026-11-09 11:30:00", note: "wrote the first draft" },
      { path: "Engineering/Codebase", space: "Engineering", title: "Codebase", text: "| Repo | What it is | Stack |\n| --- | --- | --- |\n| `notes` (primary) | The app | Rust, Axum, SQLite |\n| `notes-web` | Marketing site | Next.js, Tailwind |\n\n- Sync runs in `sync/`, a CRDT per note.\n- Email goes through Resend.", edits: 3, by: "OneCorp", first_by: "Odile Martin", at: "2026-11-10 08:00:00", note: "read it from the repo" },
      { path: "Engineering/Build and test", space: "Engineering", title: "Build and test", text: "- `cargo test` for the app, `pnpm test` in `notes-web`.\n- Before calling work done: `cargo clippy -- -D warnings` and `cargo test`.", edits: 1, by: "Odile Martin", first_by: "Odile Martin", at: "2026-11-09 11:28:00", note: "wrote the first draft" },
      { path: "Engineering/Conventions", space: "Engineering", title: "Conventions", text: "- Errors with `anyhow`, `?` everywhere, no `unwrap` outside tests.\n- One module per feature.", edits: 1, by: "Odile Martin", first_by: "Odile Martin", at: "2026-11-09 11:28:00", note: "wrote the first draft" },
    ] },
    page_history: [
      { id: 4, by: "Odile Martin", note: "proposed it, and you added it", at: "2026-11-10 10:05:00", text: "- Standard library first. A new dependency needs a reason in the PR.\n- Magic links expire after 15 minutes.\n- No email reminders: SMS only." },
      { id: 3, by: "You", note: "", at: "2026-11-09 18:00:00", text: "- Standard library first. A new dependency needs a reason in the PR.\n- Magic links expire after 15 minutes." },
      { id: 1, by: "You", note: "", at: "2026-11-02 09:00:00", text: "- Standard library first." }],
    save_page: "Company/Decisions", restore_page: "Company/Decisions",
    github_repos: [["snowztech/vikusha", "A Go framework for building AI agents", false], ["snowztech/onecorp", "Run a company of AI employees", false], ["lucas/notes", "Notes app for writers", true], ["snowztech/website", "", false]].map(([name, description, priv]) => ({ name, description, private: priv })),
    setup_info: { runtimes: ["claude", "fake"], voices: VOICES,
      teams_from: [{ company: "tvcask", hires: [{ name: "Inès Costa", role: "CTO", dept: "Engineering", runtime: "claude", voice: "calm" }, { name: "Kenji Sato", role: "Engineer", dept: "Engineering", runtime: "codex", voice: "terse" }] }],
      templates: [
      { key: "solo", name: "Solo builder", description: "One engineer who ships. Hire more when you need them.", runtime: null, hires: [{ name: "Maya Okafor", role: "Engineer", dept: "Engineering" }] },
      { key: "startup", name: "Startup", description: "A CTO, frontend and backend engineers, a PM, a designer, and a writer for the launch.", runtime: null, hires: [{ name: "Odile Martin", voice: "calm", role: "CTO", dept: "Engineering", tier: "strongest" }, { name: "Maya Okafor", voice: "upbeat", role: "Frontend engineer", dept: "Engineering" }, { name: "Tomás Reis", voice: "terse", role: "Backend engineer", dept: "Engineering" }, { name: "Rui Alves", voice: "curious", role: "Product manager", dept: "Product" }, { name: "Lina Park", voice: "warm", role: "Product designer", dept: "Product" }, { name: "Sam Duarte", voice: "dry", role: "Growth writer", dept: "Growth", tier: "cheapest" }] },
      { key: "indie", name: "Indie maker", description: "Build it and launch it yourself: an engineer, a designer, and a growth marketer.", runtime: null, hires: [{ name: "Maya Okafor", role: "Engineer", dept: "Product" }, { name: "Lina Park", role: "Product designer", dept: "Product" }, { name: "Sam Duarte", role: "Growth marketer", dept: "Growth" }] },
      { key: "agency", name: "Agency", description: "Client work: a PM who plans, two engineers, a designer, and a QA reviewer who checks every change.", runtime: null, hires: [{ name: "Rui Alves", role: "Project manager", dept: "Delivery" }, { name: "Maya Okafor", role: "Engineer", dept: "Delivery" }, { name: "Tomás Reis", role: "Engineer", dept: "Delivery" }, { name: "Lina Park", role: "Product designer", dept: "Delivery" }, { name: "Odile Martin", role: "QA reviewer", dept: "Delivery" }] },
      { key: "oss", name: "Open source", description: "A maintainer who reviews every change, an engineer, and a writer for docs and changelogs.", runtime: null, hires: [{ name: "Odile Martin", role: "Maintainer", dept: "Engineering" }, { name: "Maya Okafor", role: "Engineer", dept: "Engineering" }, { name: "Sam Duarte", role: "Docs writer", dept: "Docs" }] },
      { key: "empty", name: "Start empty", description: "No one yet. Add departments and hire them yourself.", runtime: null, hires: [] }] },
    ticket_detail: { kind: "work", key: "SNO-11", title: "Pricing page", status: "review", description: "Free and Pro plans. Pro is $6 a month.", created_by: "Rui Alves", after: null, pr_number: 41, pr_url: "https://github.com/snowztech/notes/pull/41", employee: "odile", name: "Odile Martin", branch: "onecorp/snowztech/sno-11", base: "main", spend: "$0.31",
      log: [["10:40", "start", "claude (opus), started"], ["10:40", "tool", "Read src/routes/mod.rs"], ["10:41", "say", "Pricing lives next to the landing page, reusing the plan cards from settings."],
        ["10:44", "tool", "Write src/routes/pricing.rs"], ["10:47", "tool", "Edit templates/pricing.html"], ["10:49", "tool", "Bash cargo test pricing"], ["10:50", "error", "test pricing::pro_monthly ... FAILED"],
        ["10:52", "tool", "Edit src/routes/pricing.rs"], ["10:54", "tool", "Bash cargo test"], ["10:55", "say", "All 48 tests pass. Pro shows $6 a month, yearly saves two months."],
        ["10:56", "tool", "post_message Pricing page is up for review"], ["10:58", "commit", "c3dbf20 SNO-11: Pricing page"], ["11:02", "done", "finished, $0.31"]].map(([t, kind, summary]) => ({ at: `2026-11-09 ${t}:00`, kind, summary })) },
    new_repo_info: { parent: "/Users/you/dev", owners: ["you", "snowztech"] },
    readiness: [{ level: "ok", what: "git", detail: "git version 2.47.1", fix: null }, { level: "ok", what: "Claude Code", detail: "2.3.4", fix: null },
      { level: "warn", what: "Codex", detail: "not installed. Employees can't run on it", fix: "npm install -g @openai/codex" }, { level: "warn", what: "GitHub CLI", detail: "installed but not signed in", fix: "gh auth login" }],
    settings: { talk: "balanced", talk_models: { cheapest: ["Haiku 5.5"], balanced: ["Sonnet 5.5"] }, commits: "team", git_you: { name: "Lucas", email: "lucas@example.com" }, repos: [{ name: "notes", path: "/Users/you/dev/notes", main: true, commands: ["cargo build", "cargo test", "cargo check", "cargo clippy", "cargo fmt"], custom: false }, { name: "notes-web", path: "/Users/you/dev/notes-web", main: false, commands: ["pnpm install", "pnpm test", "pnpm run"], custom: false }], name: "snowztech", repo: "/Users/you/dev/snowztech", data: "/Users/you/.onecorp/companies/snowztech", home: "/Users/you/.onecorp", ship: "github", forge: "github", forge_name: "GitHub", origin: "git@github.com:snowztech/notes.git", tickets: 22, people: 6, version: "0.1.0" },
    health: [{ level: "ok", what: "git", detail: "git version 2.47.1", fix: null }, { level: "ok", what: "Claude Code", detail: "2.3.4 (Claude Code)", fix: null },
      { level: "warn", what: "Codex", detail: "not installed. Employees can't run on it", fix: "npm install -g @openai/codex, then run codex once to sign in" },
      { level: "ok", what: "company snowztech", detail: "opens pull requests on git@github.com:snowztech/notes.git (Logged in to github.com account you)", fix: null }],
    check_repo: { ok: true, path: "/Users/you/dev/snowztech", branch: "main", suggested: "snowztech", used_by: [], forge: "GitHub", pr_word: "pull request", signed_in: true, account: "Logged in to github.com account you" },
    founder: { name: "Lucas", photo: null },
    save_founder: null,
    create_company: "acme",
    spend_report: { budget: 5, run_limit: 5, estimated: true,
      totals: { today: { cost: 3.24, tokens: 180000, label: "$3.24" }, week: { cost: 14.8, tokens: 900000, label: "$14.80" }, all: { cost: 31.5, tokens: 2100000, label: "$31.50" } },
      days: [0.4, 1.2, 0, 2.1, 3.8, 1.1, 0, 0.6, 2.9, 4.6, 1.8, 2.2, 3.1, 3.24].map((cost, i) => ({ day: new Date(Date.UTC(2026, 9, 27 + i)).toISOString().slice(0, 10), cost, kinds: { ticket: cost * 0.78, chat: cost * 0.14, lead: cost * 0.05, help: cost * 0.03 } })),
      people: [["odile", "Odile Martin", 1.31, 6.2, 12.4, 0], ["tomas", "Tomás Reis", 1.12, 4.1, 9.8, 3], ["maya", "Maya Okafor", 0.52, 3.0, 6.1, 0], ["rui", "Rui Alves", 0.29, 1.5, 3.2, 0]].map(([id, name, t, w, a, cap]) => ({ id, name, cap, today: { cost: t }, week: { cost: w }, all: { cost: a } })),
      projects: [{ key: "SNO-20", title: "First version", today: { cost: 2.1 }, week: { cost: 11.2 }, all: { cost: 24.9 } }],
      agents: [{ runtime: "claude", estimated: false, today: { cost: 2.12 }, week: { cost: 10.7 }, all: { cost: 21.7 } }, { runtime: "codex", estimated: true, today: { cost: 1.12 }, week: { cost: 4.1 }, all: { cost: 9.8 } }],
      prices: [{ runtime: "codex", price: 5 }, { runtime: "claude", price: 9 }],
      limits: [{ runtime: "claude", times: 3, elsewhere: [] }],
      kinds: [{ kind: "ticket", label: "Tickets", today: { cost: 2.61, tokens: 0, label: "" }, week: { cost: 21.4, tokens: 0, label: "" }, all: { cost: 58.2, tokens: 0, label: "" } }, { kind: "chat", label: "Chat replies", today: { cost: 0.42, tokens: 0, label: "" }, week: { cost: 3.1, tokens: 0, label: "" }, all: { cost: 7.9, tokens: 0, label: "" } }, { kind: "lead", label: "Lead decisions", today: { cost: 0.12, tokens: 0, label: "" }, week: { cost: 0.9, tokens: 0, label: "" }, all: { cost: 2.2, tokens: 0, label: "" } }, { kind: "help", label: "Ask OneCorp", today: { cost: 0.09, tokens: 0, label: "" }, week: { cost: 0.3, tokens: 0, label: "" }, all: { cost: 0.3, tokens: 0, label: "" } }] },
    chat: { channel: "general", messages: [
      { id: 1, at: "2026-11-09 09:12:00", author: "founder", name: "Lucas", founder: true, text: "Morning team. Goal this week: magic link login and the pricing page live.", key: null, pending: false },
      { id: 2, at: "2026-11-09 09:40:00", author: "rui", name: "Rui Alves", text: "Plan is in your inbox as SNO-20. Setup first, then login and sync in parallel.", key: "SNO-20", pending: false },
      { id: 3, at: "2026-11-09 10:05:00", author: "maya", name: "Maya Okafor", text: "Heads up: magic links need an **email provider**. I'll stub it behind the `Mailer` trait for now, see [the Resend docs](https://resend.com/docs).", key: "SNO-12", pending: false },
      { id: 4, at: "2026-11-09 10:06:00", author: "maya", name: "Maya Okafor", text: "@odile can you look at the token hashing when you review?", key: "SNO-12", pending: false },
      { id: 5, at: "2026-11-09 11:02:00", author: "odile", name: "Odile Martin", text: "Pricing page is up for review in SNO-11. Copy needs a look on mobile.", key: "SNO-11", pending: false },
      { id: 6, at: "2026-11-09 11:20:00", author: "founder", name: "Lucas", founder: true, text: "@tomas how far are the Stripe webhook retries?", key: null, pending: false },
      { id: 7, at: "2026-11-09 11:20:00", author: "tomas", name: "Tomás Reis", text: "", key: null, pending: true }] },
    role_for: "backend",
    roles: [{ key: "backend", label: "Backend engineer", playbook: BACKEND }, { key: "frontend", label: "Frontend engineer", playbook: "- Handle every state." }, { key: "engineer", label: "Engineer", playbook: "- Read the code first." }, { key: "reviewer", label: "Tech lead", playbook: "- Read a diff for what it breaks." }],
    person: { id: "tomas", name: "Tomás Reis", role: "Backend engineer", runtime: "codex", model: null, bio: "We write Go with table tests. Logs go through slog.", kind: "backend", picked: false, playbook: null, lead: false, department: 1, voice: "dry", voices: VOICES, current: { key: "SNO-15", title: "Retry failed Stripe webhooks", status: "doing" },
      activity: [{ at: "2026-11-09 10:49:00", kind: "say", summary: "Retries happen in the webhook handler today, inline, so one slow Stripe call holds the request open. I'll move them to the jobs table with a backoff of 1, 5 and 30 minutes, and keep the handler idempotent with the event id.\n\nThen the tests: one for a duplicate event, one for a retry that succeeds on the second try." }, { at: "2026-11-09 10:50:00", kind: "tool", summary: "Edit src/billing/webhook.rs" }, { at: "2026-11-09 10:51:00", kind: "tool", summary: "Bash cargo test billing -- --nocapture 2>&1 | tail -40; echo \"exit ${PIPESTATUS[0]}\"" }, { at: "2026-11-09 10:51:00", kind: "error", summary: "A variable in braces in this command can't be checked before it runs" }, { at: "2026-11-09 10:52:00", kind: "tool", summary: "Bash cargo test billing" }], tickets: [{ key: "SNO-15", title: "Retry failed Stripe webhooks", status: "doing", spend: "75k tok" }, { key: "SNO-9", title: "Rate limit the public API", status: "failed", spend: "40k tok" }, { key: "SNO-8", title: "CI pipeline", status: "merged", spend: "$0.22" }, { key: "SNO-4", title: "Notes API: create and list", status: "merged", spend: "$0.48" }],
      memory: ["2026-11-09 Stripe retries for 3 days, make handlers idempotent instead of queueing", "2026-11-08 Lucas wants errors logged with the ticket key, never swallowed", "2026-11-07 Billing code lives in src/billing, one file per provider", "2026-11-06 Run cargo test billing before the full suite, it's 10x faster"] },
  };
  data.review = { key: "SNO-11", title: "Pricing page", base: "main", branch: "onecorp/snowztech/sno-11", added: 5, removed: 1,
    files: [{ path: "web/src/pages/pricing.tsx", added: 4, removed: 1 }, { path: "README.md", added: 1, removed: 0 }],
    commits: [{ hash: "c3dbf20", author: "Odile Martin", subject: "SNO-11: pricing page" }],
    patch: "diff --git a/web/src/pages/pricing.tsx b/web/src/pages/pricing.tsx\nindex 1..2 100644\n--- a/web/src/pages/pricing.tsx\n+++ b/web/src/pages/pricing.tsx\n@@ -1,3 +1,6 @@\n export function Pricing() {\n-  return null\n+  return <>\n+    <Plan name=\"Free\" price={0} />\n+    <Plan name=\"Pro\" price={6} features={[\"Unlimited notes\", \"Sync across every device\", \"Version history for a year\", \"Priority support\"]} />\n+  </>\n }\ndiff --git a/README.md b/README.md\nindex 3..4 100644\n--- a/README.md\n+++ b/README.md\n@@ -1 +1,2 @@\n # Notes\n+Pricing: free, or $6 a month.\n", truncated: false };
  if (cmd === "chat" && String(args?.channel).startsWith("dm:")) return Promise.resolve({ channel: args.channel, messages: args.channel === "dm:tomas"
    ? [{ id: 13, at: "2026-11-09 11:40:00", author: "founder", name: "Lucas", founder: true, text: "How are the webhook retries going?", key: null, pending: false },
       { id: 14, at: "2026-11-09 11:41:00", author: "tomas", name: "Tomás Reis", text: "Close. Retries back off over 3 days now, and handlers are idempotent. Tests next, then it's in your Inbox as SNO-15.", key: "SNO-15", pending: false }] : [] });
  // Saving your name sticks for the rest of the preview, like it does in the app.
  if (cmd === "clone_target") return Promise.resolve(`/Users/you/dev/${String(args.name).split("/").pop()}`);
  if (cmd === "save_founder") { previewFounder = { name: args.name, photo: null }; return Promise.resolve(null); }
  if (cmd === "founder" && previewFounder) return Promise.resolve(previewFounder);
  if (cmd === "check_repo" && args?.path) return Promise.resolve({ ...(data.check_repo as object), path: args.path, suggested: args.path.split("/").pop() });
  if (cmd in data) return Promise.resolve(data[cmd]);
  return Promise.reject("This is a preview. Open the OneCorp app to make changes.");
}
