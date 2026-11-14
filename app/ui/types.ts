// What the Tauri commands return, as the UI reads it. The Rust side builds these with json!, in
// crates/core/src/view.rs and app/src/main.rs, so a field renamed there must be renamed here too.

export type Status = "backlog" | "todo" | "queued" | "waiting" | "doing" | "blocked" | "review" | "failed" | "done";
export type EmployeeStatus = "working" | "blocked" | "idle" | "queued" | "done";
export type Level = "ok" | "warn" | "fail";
type Cost = { cost: number; tokens?: number; label?: string };
type Windows = { today: Cost; week: Cost; all: Cost };

export interface Employee {
  id: string; name: string; role: string; runtime: string; model: string | null;
  face: string; photo: string | null; photo_path: string | null; today: number; cap: number;
  status: EmployeeStatus; bubble: string; spend: string; department: number | null;
  ticket: { key: string; title: string } | null;
}

export interface Question {
  id: number; kind: "question" | "command" | "wiki"; key: string | null;
  employee: string; name: string; question: string; section: string; options: string[]; repo?: string;
  default: string | null; blocking: boolean; via?: string | null; via_name?: string | null; note?: string | null;
  lead_name?: string | null;
}

export interface PlanTicket { key: string; title: string; description: string; assignee: string | null; name: string; after: string | null; status: Status; repo?: string | null }

export interface Review {
  key: string; title: string; employee: string; name: string; kind: string; tickets: PlanTicket[];
  document?: string | null; summary: string; spend: string; pr_number?: number | null; pr_url?: string | null; empty?: boolean;
}

export interface Ticket {
  key: string; title: string; assignee: string | null; status: Status; spend: string;
  after?: string | null; kind?: string; repo?: string | null; created_by?: string | null; plan?: string | null;
}

export interface Department { id: number | null; name: string; lead: string | null; members: string[] }
export interface Channel { name: string; title: string; department: number | null; last: number | null }
export interface Dm { id: string; channel: string; last: number | null }
export interface FeedItem { at: string; key: string; employee: string; name: string; kind: string; summary: string; title: string }
export interface Failed { key: string; title: string; employee: string; name: string; why: string }
export interface Project { key: string; title: string; goal: string; status: Status; planner: string | null; planner_name: string; total: number; done: number }
export interface Spend { cost: number; tokens: number; label: string; today: number; today_label: string; budget: number; warn: boolean; paused_by_budget: boolean }

export interface Snapshot {
  kickoff: { brief: string; planner: string | null; planned: boolean; planning: { key: string; name: string; title: string }[] };
  projects: Project[];
  paused: boolean;
  repos: { name: string; path: string; main: boolean }[];
  limits: { runtime: string; back: string }[];
  ship: { mode: string; forge: string | null; pr_word: string | null };
  departments: Department[];
  company: string;
  serve: boolean;
  employees: Employee[];
  tickets: Ticket[];
  inbox: Question[];
  with_leads: Question[];
  reviews: Review[];
  failed: Failed[];
  feed: FeedItem[];
  channels: Channel[];
  dms: Dm[];
  spend: Spend;
}

export interface LogLine { at: string; kind: string; summary: string }

export interface TicketDetail {
  key: string; title: string; status: Status; kind: string; document: string | null; employee: string | null; name: string;
  branch: string | null; base: string | null; spend: string; log: LogLine[]; pr_number: number | null; pr_url: string | null;
  repo: string | null; description: string; after: string | null; created_by: string | null;
}

export interface Voice { key: string; label: string; sample: string }

export interface RoleInfo { key: string; label: string; playbook: string }

export interface Person {
  id: string; name: string; role: string; runtime: string; model: string | null; bio: string; kind: string; picked: boolean; playbook: string | null; lead: boolean; department: number | null;
  voice: string; voices: Voice[];
  current: { key: string; title: string; status: Status } | null;
  activity: LogLine[];
  tickets: { key: string; title: string; status: Status; kind: string; spend: string }[];
  memory: string[];
}

export interface Diff {
  key: string; title: string; base: string; branch: string; added: number; removed: number;
  files: { path: string; added: number; removed: number }[];
  commits: { hash: string; author: string; subject: string }[];
  patch: string; truncated: boolean;
}

export interface Proposed { key: string; title: string; employee: string | null; name: string | null; status: string }
export interface Issue { kind: "bug" | "idea"; title: string; body: string }
export interface Message { id: number; at: string; author: string; name: string; text: string; key: string | null; pending: boolean; founder: boolean; proposed?: Proposed[]; issue?: Issue | null }
export interface Chat { channel: string; messages: Message[] }

export interface Look { color?: string; logo?: string | null }
export interface Companies { list: string[]; current: string | null; looks: Record<string, Look>; dev: string | null }
export interface Founder { name: string; photo: string | null }

export interface SettingsRepo { name: string; path: string; main: boolean; commands: string[]; custom: boolean; setup?: string | null }
export interface Settings {
  name: string; repo: string; data: string; home: string; ship: string; forge: string | null; forge_name: string | null;
  origin: string | null; tickets: number; people: number; version: string; repos: SettingsRepo[];
  commits: string; pr_sign: boolean; talk: string; talk_models: Record<string, string[]>; run_any?: boolean;
  git_you: { name: string; email: string } | null;
}

export interface SpendReport {
  totals: Windows; budget: number; estimated: boolean;
  days: { day: string; cost: number; kinds?: Record<string, number> }[];
  people: (Windows & { id: string; name: string; runtime?: string; cap: number })[];
  projects: (Windows & { key: string; title: string })[];
  agents: (Windows & { runtime: string; estimated: boolean; price?: number })[];
  prices: { runtime: string; price: number }[];
  limits?: { runtime: string; times: number; elsewhere: string[] }[];
  kinds?: (Windows & { kind: string; label: string })[];
}

export interface Check { level: Level; what: string; detail: string; fix: string | null }
export interface Attention { company: string; inbox: Question[]; reviews: Review[]; failed: Failed[]; spend: Spend }

export interface Model { id: string; name: string; about: string }
type Tier = { id: string; name: string } | null;
export interface Models {
  claude: Model[]; codex: Model[];
  tiers: Record<string, Record<string, Tier>>;
  defaults: Record<string, string | null>;

}

export interface Hire { name: string; role: string; dept: string | null; tier?: string | null; voice?: string | null; runtime?: string; model?: string | null }
export interface Template { key: string; name: string; description: string; runtime: string | null; hires: Hire[] }
export interface SetupInfo { templates: Template[]; runtimes: string[]; voices: Voice[]; teams_from: { company: string; hires: Hire[] }[] }

export interface RepoCheck {
  ok: boolean; problem?: string; path: string; branch: string; suggested: string; used_by: string[];
  forge: string | null; pr_word: string | null; signed_in: boolean | null; account: string | null;
}
export interface NewRepoInfo { parent: string; owners: string[] }
export interface GithubRepo { name: string; description: string; private: boolean }

// What each command returns. Commands not listed return nothing the UI reads.
export interface Results {
  add_department: number;
  add_repo: SettingsRepo[];
  attention: Attention[];
  chat: Chat;
  check_repo: RepoCheck;
  clone_repo: string;
  clone_target: string;
  companies: Companies;
  create_company: string;
  create_repo: string;
  fire: string[];
  founder: Founder | null;
  github_repos: GithubRepo[];
  health: Check[];
  image_preview: string;
  models: Models;
  new_repo_info: NewRepoInfo;
  new_ticket: string;
  person: Person;
  roles: RoleInfo[];
  role_for: string;
  plan_project: string;
  readiness: Check[];
  remove_repo: SettingsRepo[];
  rescan_commands: SettingsRepo[];
  make_main_repo: SettingsRepo[];
  set_setup: SettingsRepo[];
  review: Diff;
  set_commands: SettingsRepo[];
  set_projects_folder: string;
  settings: Settings;
  setup_info: SetupInfo;
  snapshot: Snapshot;
  spend_report: SpendReport;
  ticket_detail: TicketDetail;
  voices: Voice[];
  wiki: Wiki;
  page_history: PageEdit[];
  save_page: string;
  restore_page: string;
}
export type Returns<K extends string> = K extends keyof Results ? Results[K] : null;

export interface WikiPage {
  path: string; space: string; title: string; text: string; edits: number;
  by: string | null; at: string | null; note: string | null; first_by: string | null; rule?: boolean;
}
export interface Wiki { pages: WikiPage[]; recent: { page: string; by: string; note: string; at: string }[]; rules_tokens?: number }
export interface PageEdit { id: number; by: string; note: string; at: string; text: string }

/* The UI's own state */

export interface PersonForm {
  name: string; role: string; runtime: string; model: string | null; bio: string; kind?: string | null; kindPicked?: boolean; rewrite?: string | null; rewriting?: boolean; voice?: string | null; face?: string;
  photoUrl?: string | null; photoPath?: string | null; removePhoto?: boolean; hasPhoto?: boolean;
}

// One shape for every drawer: which fields matter depends on `type`.
export interface Drawer {
  type: "ticket" | "person" | "hire" | "new-ticket" | "plan";
  key?: string; id?: string; dept?: number; who?: string | null; edit?: boolean; form?: PersonForm | null;
}

export interface SetupPerson { name: string; role: string; dept: string; runtime: string; voice?: string; tier?: string; model?: string; pb?: string }
export interface Setup {
  step: number; mode: string; path: string; checked?: string | null; repo: RepoCheck | null; name: string; mission: string; brief?: string;
  runtime: string; info: SetupInfo; repoInfo: NewRepoInfo; folder: string; parent: string; owner: string; private: boolean;
  people: SetupPerson[]; extra: { path: string; name: string; branch: string }[]; mainName?: string;
  color: string; logo: string | null; logoUrl: string | null; template?: string;
  ready?: Check[] | null; creating?: boolean; cloning?: string | null; pick?: { name: string; path: string } | null; justAdded?: string | null;
  gh?: { repos?: GithubRepo[]; error?: string } | null; ghLoading?: boolean;
  launch?: { done: number; steps: string[] } | null;
}

export interface Profile { name: string; photoUrl?: string | null; photo?: string | null; noPhoto?: boolean; inSetup?: boolean }

export interface State {
  companies: string[]; company: string | null; looks: Record<string, Look>; dev: string | null;
  snap: Snapshot | null; last: string; view: string;
  drawer: Drawer | null; person: Person | null; ticket: TicketDetail | null;
  addingDept: boolean; open: Record<string, boolean>; channel: string; chat: Chat | null;
  diffs: Record<string, Diff | "loading">; wiki: Wiki | null; hbEdit: boolean;
  page: string | null; wikiQ?: string; wikiNew?: boolean; wikiSpace?: string; history?: PageEdit[] | null; historyOf?: string | null; version?: number | null;
  setup: Setup | null; founder: Founder | null | undefined; profile: Profile | null; welcome?: boolean; onboarding?: boolean;
  project: string | null; rail: boolean; needs: Record<string, number>; pop: false | "menu" | "forget";
  settings: Settings | null; health: Check[] | null; ready?: Check[] | null; spend?: SpendReport;
  confirm: string | null; sheet: string | null; shown: string; customBudget?: boolean; spendBy?: string; spendAll?: number;
  voices?: Voice[] | null; roles?: RoleInfo[] | null; models?: Models | null; runtimes?: string[] | null;
  switching?: boolean; members?: string | null; msgDel?: number | null;
  mention?: { opts: (Employee | { all: true })[]; i: number } | null;
}
