import { candidates, openHire, openPerson, openTicket, readPerson, renderDrawer, saveNewTicket } from "./drawers.ts";
import { $, esc, first, initials, toast } from "./lib.ts";
import { PALETTE, drawFace, face, icon } from "./look.ts";
import { md } from "./md.ts";
import { chatToEnd, refresh, render } from "./render.ts";
import { renderPop, renderSide, toggleRail } from "./shell.ts";
import { S, byId, call, invoke, myName } from "./state.ts";
import type { PersonForm } from "./types.ts";
import { restartToUpdate } from "./update.ts";
import { pickMention, updateMentions } from "./views/chat.ts";
import { approvalCopy } from "./views/inbox.ts";
import { currentPage } from "./views/wiki.ts";
import { addExtraRepo, addRepo, backToYou, checkRepo, hasPhoto, loadGithub, makeRepo, readSetup, recheckTools, saveProfile, setTemplate, startSetup } from "./views/onboarding.ts";
import { loadSection, loadSpend, openSettings, renderSheet, saved, setTheme } from "./views/settings.ts";

// Every click, submit and key, handled once on the document. Elements say what they do with data-* attributes.
/* Actions */
document.addEventListener("mousedown", e => {
  const target = e.target as HTMLInputElement;
  const o = target.closest?.<HTMLElement>("[data-mpick]");
  if (o) { e.preventDefault(); pickMention(Number(o.dataset.mpick)); }
});

document.addEventListener("click", async e => {
  const target = e.target as HTMLInputElement;
  if (S.pop && !target.closest("#pop, [data-profile], [data-pop]")) { S.pop = false; renderPop(); }
  if (target.id === "sheet") { S.sheet = null; return renderSheet(); }
  if (S.switching && !target.closest(".switch")) { S.switching = false; renderSide(); }
  if (S.members && !target.closest(".members-pop, [data-members]")) { S.members = null; render(); }
  if (target.id === "scrim") { S.drawer = null; return renderDrawer(); }
  const el = target.closest<HTMLButtonElement>("[data-update-restart],[data-plan-del],[data-talk],[data-commits],[data-welcome],[data-step-to],[data-repo-rm],[data-repo-add],[data-face-set],[data-commands-reset],[data-vpick],[data-msg-del],[data-theme-pick],[data-budget],[data-budget-custom],[data-look-color],[data-look-logo],[data-you-photo],[data-you-nophoto],[data-dismiss],[data-dismiss-q],[data-pr-sign],[data-discard],[data-cmd-del],[data-members],[data-replan],[data-new-ticket],[data-nt],[data-start],[data-more],[data-switch],[data-settings-tab],[data-keep-going],[data-dm],[data-face],[data-avatar-photo],[data-avatar-clear],[data-skip-setup],[data-su-agent],[data-hb-edit],[data-wk-page],[data-wk-new],[data-wk-history],[data-wk-version],[data-wk-restore],[data-wk-delete],[data-learn],[data-agent],[data-model],[data-copy],[data-term],[data-recheck],[data-hide-start],[data-brief],[data-pause],[data-pick-fill],[data-edit-person],[data-fire],[data-make-lead],[data-dept-cancel],[data-remove],[data-goto],[data-rail],[data-plan-new],[data-project],[data-pop],[data-settings],[data-sheet],[data-sheet-close],[data-toc],[data-close-open],[data-ship],[data-reveal],[data-health],[data-confirm],[data-forget],[data-channel],[data-mention],[data-profile],[data-pf],[data-open-url],[data-ticket],[data-stop],[data-new-co],[data-su],[data-tpl],[data-retry],[data-hb-del],[data-hb-raw],[data-diff],[data-hide-diff],[data-co],[data-view],[data-person],[data-hire],[data-add-dept],[data-close],[data-answer],[data-approve],[data-pick]");
  if (!el) return;
  const d = el.dataset;
  if (d.updateRestart) return restartToUpdate();
  if (d.planDel) { await call("delete_ticket", { company: S.company, key: d.planDel }, `Removed ${d.planDel} from the plan`).catch(() => {}); return; }
  if (d.dismissQ) { el.disabled = true; await call("dismiss_question", { company: S.company, id: Number(d.dismissQ) }, "Dismissed").catch(() => (el.disabled = false)); return; }
  if (d.discard) {
    el.disabled = true;
    try { await invoke("discard_ticket", { company: S.company, key: d.discard }); toast(`Discarded the work. ${d.discard} is back in the backlog`); S.confirm = null; } catch (err) { el.disabled = false; return toast(String(err), true); }
    return refresh(true);
  }
  if (d.dismiss) { el.disabled = true; await call("dismiss_plan", { company: S.company, key: d.dismiss }, "Plan dismissed").catch(() => (el.disabled = false)); return; }
  if (d.replan) {
    const brief = $("#brief-text").value.trim();
    if (!brief) return $("#brief-text").focus();
    await call("kickoff", { company: S.company, brief, by: null }, "Saved. Your team is planning it").catch(() => {});
    return;
  }
  if (d.newTicket) { S.drawer = { type: "new-ticket", who: byId(d.newTicket) ? d.newTicket : null }; renderDrawer(); return $("#nt-title")?.focus(); }
  if (d.nt) { e.preventDefault(); return saveNewTicket(d.nt === "later"); }
  if (d.start) {
    el.disabled = true;
    await call("start_ticket", { company: S.company, key: d.start }, `${d.start} is starting`).catch(() => (el.disabled = false));
    return;
  }
  if (d.channel) { S.channel = d.channel; S.chat = null; render(); return refresh(true); }
  if (d.members) { S.members = S.members === d.members ? null : d.members; return render(); }
  if (d.mention) {
    if (S.members) { S.members = null; render(); }
    const input = $("#say-text"), name = first(byId(d.mention)?.name || d.mention);
    input.value = (input.value.trim() ? input.value.trimEnd() + " " : "") + "@" + name + " ";
    return input.focus();
  }
  if (d.skipSetup) { if (await saveProfile(S.profile!.photo, S.profile!.noPhoto)) { S.profile = null; toast(`Welcome back, ${myName()}`); refresh(true); } return; }
  if (d.hideStart) { try { localStorage.setItem("onecorp.started." + S.company, "hide"); } catch {} return render(); }
  if (d.brief) { S.view = "wiki"; S.page = "Company/What we're building"; S.hbEdit = true; S.wikiNew = false; await refresh(true); return $("#hb-text")?.focus(); }
  if (d.pause) {
    const on = d.pause === "1";
    await call("set_paused", { company: S.company, paused: on }, on ? `${S.company} is paused. Running work finishes, nothing new starts` : `${S.company} is back at work`).catch(() => {});
    return;
  }
  if (d.suAgent) { readSetup(); const [i, k] = d.suAgent.split(":"); const p = S.setup!.people[Number(i)]; if (p.runtime !== k) p.model = ""; p.runtime = k; return render(); }
  if (d.agent) { Object.assign(S.drawer!.form ||= {} as PersonForm, readPerson(), { runtime: d.agent, model: null }); return renderDrawer(); }
  if (d.welcome) { S.welcome = false; render(); return $("#pf-name")?.focus(); }
  if (d.stepTo) {
    if (d.stepTo === "You") return backToYou();
    readSetup(); S.setup!.step = ["Connect", "Company", "Repo", "Team"].indexOf(d.stepTo); return render();
  }
  if ("vpick" in d) {
    $("#pf-v").value = d.vpick!;
    el.parentElement!.querySelectorAll(".vchip").forEach(b => b.setAttribute("aria-pressed", String(b === el)));
    $("#pf-vs").textContent = (S.voices || []).find(v => v.key === d.vpick)?.sample || "Picked to fit their role. Change it any time.";
    return;
  }
  if ("model" in d && el.classList.contains("mchip") && !el.classList.contains("vchip")) {
    $("#pf-m").value = d.model!;
    el.parentElement!.querySelectorAll(".mchip").forEach(b => b.setAttribute("aria-pressed", String(b === el)));
    return;
  }
  if (d.copy) { navigator.clipboard?.writeText(d.copy).then(() => toast("Copied. Paste it in Terminal")).catch(() => toast(d.copy!)); return; }
  if (d.term) { await invoke("in_terminal", { command: d.term }).catch(e => toast(String(e))); recheckTools(); return; }
  if (d.recheck) { S.setup!.ready = null; render(); S.setup!.ready = await invoke("readiness").catch(() => []); return render(); }
  if (d.pickFill) {
    const keep = readPerson(), c = candidates(keep.role)[Number(d.pickFill)];
    S.drawer!.form = { ...keep, name: c.name, runtime: c.runtime, model: c.model };
    renderDrawer(); return $("#pf-n")?.focus();
  }
  if ("editPerson" in d) { S.drawer!.edit = !!d.editPerson; S.drawer!.form = null; S.confirm = null; renderDrawer(); return $("#pf-n")?.focus(); }
  if (d.makeLead) { await call("make_lead", { company: S.company, id: d.makeLead }, "They lead the department now").catch(() => {}); return openPerson(d.makeLead); }
  if (d.fire) {
    el.disabled = true;
    const who = first(byId(d.fire)?.name || d.fire);
    try { await invoke("fire", { company: S.company, id: d.fire }); } catch (err) { el.disabled = false; return toast(String(err), true); }
    toast(`${who} has left the company`); S.confirm = null; S.drawer = null; renderDrawer(); return refresh(true);
  }
  if (d.deptCancel) { S.addingDept = false; return render(); }
  if (d.goto) {
    S.view = "inbox"; S.drawer = null; render();
    const card = document.getElementById("in-" + d.goto);
    if (card) { card.scrollIntoView({ behavior: "smooth", block: "center" }); card.classList.add("flash"); setTimeout(() => card.classList.remove("flash"), 1600); }
    return;
  }
  if (d.rail) return toggleRail();
  if (d.dm) { S.view = "chat"; S.channel = "dm:" + d.dm; S.chat = null; S.drawer = null; render(); await refresh(true); return $("#say-text")?.focus(); }
  if (d.face && S.drawer?.form) {
    S.drawer!.form.face = d.face;
    el.parentElement!.querySelectorAll(".fpick").forEach(b => b.setAttribute("aria-pressed", String(b === el)));
    if (!S.drawer!.form!.photoUrl) $("#drawer .dhead .with-agent").firstElementChild!.outerHTML = drawFace(d.face, "neutral");
    return;
  }
  if (d.avatarPhoto) {
    const path = await window.__TAURI__?.dialog?.open({ multiple: false, title: "Pick a photo", filters: [{ name: "Photos", extensions: ["png", "jpg", "jpeg", "webp", "gif"] }] }).catch(() => null);
    if (!path) return;
    try { Object.assign(S.drawer!.form!, readPerson(), { photoUrl: await invoke("image_preview", { path: String(path) }), photoPath: String(path), removePhoto: false }); } catch (err) { return toast(String(err), true); }
    return renderDrawer();
  }
  if (d.avatarClear) { Object.assign(S.drawer!.form!, readPerson(), { photoUrl: null, photoPath: null, removePhoto: true }); return renderDrawer(); }
  if (d.planNew) { S.drawer = { type: "plan" }; renderDrawer(); return $("#pj-name")?.select(); }
  if ("project" in d) { S.project = d.project || null; return render(); }
  if (d.switch) { S.switching = !S.switching; return renderSide(); }
  if (d.toc) return document.getElementById(d.toc)?.scrollIntoView({ behavior: "smooth", block: "start" });
  if (d.closeOpen) { S.open[d.closeOpen] = false; return render(); }
  if (d.ship) {
    try { await invoke("set_ship", { company: S.company, mode: d.ship }); S.settings = await invoke("settings", { company: S.company }); toast(d.ship === "local" ? "Finished work merges on this Mac" : "Finished work opens pull requests"); }
    catch (err) { toast(String(err), true); }
    renderSheet(); return refresh(true);
  }
  if (d.reveal) return window.__TAURI__!.opener!.revealItemInDir(d.reveal).catch(err => toast(String(err), true));
  if (d.health) { S.health = null; renderSheet(); S.ready = await invoke("readiness").catch(() => S.ready); S.health = await invoke("health").catch(() => []); return renderSheet(); }
  if ("confirm" in d) {
    // One open panel per card: asking to discard closes the reply you were writing.
    if (d.confirm?.startsWith("discard:")) S.open["chg-" + d.confirm.slice(8)] = false;
    S.confirm = d.confirm || null; if (S.drawer?.type === "ticket") renderDrawer(); else if (S.view === "inbox") render(); renderSheet(); return $("#del-name")?.focus(); }
  if (d.remove) {
    el.disabled = true;
    try { await invoke("remove_ticket", { company: S.company, key: d.remove }); }
    catch (err) { el.disabled = false; return toast(String(err), true); }
    toast(`Deleted ${d.remove}`); S.confirm = null; S.drawer = null; renderDrawer(); return refresh(true);
  }
  if (d.forget) {
    el.disabled = true;
    try { await invoke("forget_founder"); } catch (err) { el.disabled = false; return toast(String(err), true); }
    S.founder = null; S.confirm = null; S.pop = false; S.sheet = null; S.profile = { name: "" }; renderPop(); renderSheet(); return render();
  }
  if (d.more) { if (/^(chg|ans)-/.test(d.more)) S.confirm = null; S.open[d.more] = !S.open[d.more]; render(); return document.getElementById(d.more)?.focus(); }
  if (d.profile) return openSettings("you");
  if (d.pop) { S.pop = d.pop === "menu" && S.pop === "menu" ? false : d.pop as "menu" | "forget"; renderPop(); return $("#pf-name")?.focus(); }
  if (d.settings) return openSettings();
  if (d.sheet) { S.sheet = d.sheet; S.confirm = null; S.customBudget = false; renderSheet(); return loadSection(d.sheet); }
  if (d.budget !== undefined) { S.customBudget = false; await invoke("set_budget", { company: S.company, dollars: Number(d.budget) }).then(saved).catch(err => toast(String(err), true)); await loadSpend(); return refresh(true); }
  if (d.budgetCustom) { S.customBudget = true; renderSheet(); return $("#budget-in")?.focus(); }
  if (d.lookColor) {
    await invoke("set_look", { company: S.company, color: d.lookColor, logo: null, removeLogo: true }).then(saved).catch(err => toast(String(err), true));
    await refresh(true); return renderSheet();
  }
  if (d.lookLogo) {
    const path = await window.__TAURI__?.dialog?.open({ multiple: false, title: "Pick a logo", filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg", "webp", "svg", "gif"] }] }).catch(() => null);
    if (!path) return;
    await invoke("set_look", { company: S.company, color: S.looks[S.company ?? ""]?.color || PALETTE[0], logo: String(path), removeLogo: false }).then(saved).catch(err => toast(String(err), true));
    await refresh(true); return renderSheet();
  }
  if (d.youPhoto) {
    const path = await window.__TAURI__?.dialog?.open({ multiple: false, title: "Pick a photo of you", filters: [{ name: "Photos", extensions: ["png", "jpg", "jpeg", "webp", "gif"] }] }).catch(() => null);
    if (path && await saveProfile(String(path))) { saved(); render(); renderSheet(); }
    return;
  }
  if (d.msgDel) {
    const id = Number(d.msgDel);
    // First click asks, second deletes. Nothing else to confirm in chat, so no dialog.
    if (S.msgDel !== id) { S.msgDel = id; return render(); }
    S.msgDel = null;
    await invoke("delete_message", { company: S.company, id }).catch(err => toast(String(err), true));
    return refresh(true);
  }
  if (d.talk) {
    try { await invoke("set_talk", { company: S.company, mode: d.talk }); S.settings!.talk = d.talk; saved(); } catch (err) { toast(String(err), true); }
    return renderSheet();
  }
  if (d.prSign) {
    const on = d.prSign === "true";
    try { await invoke("set_pr_sign", { company: S.company, on }); S.settings!.pr_sign = on; saved(); } catch (err) { toast(String(err), true); }
    return renderSheet();
  }
  if (d.commits) {
    try { await invoke("set_commits", { company: S.company, mode: d.commits }); S.settings!.commits = d.commits; saved(); } catch (err) { toast(String(err), true); }
    return renderSheet();
  }
  if (d.cmdDel) {
    const r = S.settings!.repos.find(x => x.name === d.cmdDel)!;
    const text = r.commands.filter(cmd => cmd !== d.cmd).join("\n");
    S.settings!.repos = await invoke("set_commands", { company: S.company, repo: d.cmdDel, text }).catch(err => (toast(String(err), true), S.settings!.repos));
    return renderSheet();
  }
  if (d.commandsReset) {
    S.settings!.repos = await invoke("set_commands", { company: S.company, repo: d.commandsReset, text: null }).catch(err => (toast(String(err), true), S.settings!.repos));
    saved(); return renderSheet();
  }
  if (d.repoRm) {
    try { S.settings!.repos = await invoke("remove_repo", { company: S.company, name: d.repoRm }); toast(`Removed ${d.repoRm}. Its files and finished tickets stay`); } catch (err) { toast(String(err), true); }
    renderSheet(); return refresh(true);
  }
  if (d.repoAdd) {
    const path = d.repoAdd === "pick" ? await window.__TAURI__?.dialog?.open({ directory: true, multiple: false, title: "Pick another repo" }).catch(() => null) : $("#repo-path")?.value.trim();
    if (!path) return $("#repo-path")?.focus();
    try { S.settings!.repos = await invoke("add_repo", { company: S.company, path: String(path), name: null }); toast(`Added ${S.settings!.repos.at(-1)?.name}. Tickets can work in it now`); } catch (err) { toast(String(err), true); }
    renderSheet(); return refresh(true);
  }

  if (d.faceSet) {
    try { localStorage.setItem("onecorp.faces", d.faceSet); } catch {}
    render(); return renderSheet();
  }
  if (d.themePick) { setTheme(d.themePick); return renderSheet(); }
  if (d.youNophoto) { if (await saveProfile(null, true)) { saved(); render(); renderSheet(); } return; }
  if (d.settingsTab) return openSettings(d.settingsTab);
  if (d.keepGoing) { await call("keep_going_today", { company: S.company }, "Going on for today. The budget applies again tomorrow").catch(() => {}); return; }
  if (d.sheetClose) { S.sheet = null; return renderSheet(); }
  if (d.pf) {
    if (d.pf === "cancel") { S.pop = false; return renderPop(); }
    if (d.pf === "face") {
      if (S.profile) { S.profile!.photo = S.profile!.photoUrl = null; S.profile!.noPhoto = true; return render(); }
      if (await saveProfile(null, true)) { render(); renderPop(); }
      return;
    }
    if (d.pf === "pick") {
      if (!window.__TAURI__?.dialog) return;
      const path = await window.__TAURI__!.dialog!.open({ multiple: false, title: "Pick a photo of you", filters: [{ name: "Photos", extensions: ["png", "jpg", "jpeg", "webp", "gif"] }] }).catch(() => null);
      if (!path) return;
      // During onboarding the photo comes first, before your name: shown now, saved when you continue.
      if (S.profile) {
        try { S.profile!.photoUrl = await invoke("image_preview", { path: String(path) }); S.profile!.photo = String(path); S.profile!.noPhoto = false; } catch (err) { toast(String(err), true); }
        return render();
      }
      if (await saveProfile(String(path))) { render(); renderPop(); }
      return;
    }
  }
  if (d.openUrl) {
    e.preventDefault();
    return window.__TAURI__ ? invoke("open_link", { url: d.openUrl }).catch(err => toast(String(err), true)) : window.open(d.openUrl, "_blank");
  }
  if (d.ticket) return openTicket(d.ticket);
  if (d.stop) {
    el.disabled = true;
    await call("stop", { company: S.company, key: d.stop }, `Stopped ${d.stop}. Retry it anytime.`).catch(() => (el.disabled = false));
    return;
  }
  if (d.newCo) { S.switching = false; return startSetup(); }
  if (d.tpl) { readSetup(); setTemplate(d.tpl); return render(); }
  if (d.su) {
    readSetup();
    const s = S.setup!;
    if (d.su === "cancel") { S.setup = null; S.onboarding = false; S.view = "company"; return refresh(true); }
    if (d.su === "back") { if (!s.step) return backToYou(); s.step--; return render(); }
    if (d.su === "check") return checkRepo(s.path);
    if (d.su.startsWith("color-")) { s.color = d.su.slice(6); s.logo = s.logoUrl = null; return render(); }
    if (d.su === "logo-remove") { s.logo = s.logoUrl = null; return render(); }
    if (d.su === "logo") {
      const path = await window.__TAURI__!.dialog!.open({ multiple: false, title: "Pick a logo", filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg", "webp", "svg", "gif"] }] }).catch(() => null);
      if (!path) return;
      try { s.logoUrl = await invoke("image_preview", { path: String(path) }); s.logo = String(path); } catch (e) { toast(String(e), true); }
      return render();
    }
    if (d.su === "mode-existing" || d.su === "mode-new") { s.mode = d.su.slice(5); return render(); }
    if (d.su === "mode-github") {
      s.mode = "github";
      if (s.gh?.error) s.gh = null;
      loadGithub(); render();
      return $("#gh-q")?.focus();
    }
    // Picking a repo asks where it goes first. Clone does the rest.
    if (d.su.startsWith("clone:")) {
      const name = d.su.slice(6);
      s.pick = { name, path: await invoke("clone_target", { name }).catch(() => `${s.repoInfo?.parent || ""}/${name.split("/").pop()}`) };
      render();
      return $("[data-su='clone-go']")?.focus();
    }
    if (d.su === "clone-cancel") { s.pick = null; return render(); }
    if (d.su === "clone-dir") {
      const dir = await window.__TAURI__?.dialog?.open({ directory: true, multiple: false, title: `Where should ${s.pick!.name} go?`, defaultPath: s.repoInfo?.parent }).catch(() => null);
      if (dir) { s.pick!.path = `${String(dir).replace(/\/$/, "")}/${s.pick!.name.split("/").pop()}`; render(); }
      return;
    }
    if (d.su === "clone-go") {
      const path = $("#clone-path")?.value.trim() || s.pick!.path;
      s.cloning = s.pick!.name; s.pick = null; render();
      try {
        const name = s.cloning;
        const done = await invoke("clone_repo", { name, path }); s.cloning = null;
        const path2 = done;
        // The first pick is the main repo. Picks after that join it as more repos.
        // Back to the list either way, so you see it land there.
        s.mode = "existing"; s.justAdded = path2;
        if (s.repo?.ok) await addExtraRepo(path2);
        else await checkRepo(path2);
        setTimeout(() => { if (S.setup === s) s.justAdded = null; }, 2500);
      }
      catch (e) { s.cloning = null; render(); toast(String(e), true); }
      return;
    }
    if (d.su === "create-repo") return makeRepo();
    if (d.su === "toggle-gh") { s.owner = s.owner ? "" : (s.repoInfo.owners[0] || ""); return render(); }
    if (d.su === "add-person") { s.people.push({ name: "", role: "Engineer", dept: "Engineering", runtime: s.runtime, tier: "balanced" }); render(); return document.getElementById(`su-p${s.people.length - 1}-name`)?.focus(); }
    if (d.su.startsWith("del-")) { s.people.splice(Number(d.su.slice(4)), 1); return render(); }
    if (d.su === "parent") {
      const path = await window.__TAURI__!.dialog!.open({ directory: true, multiple: false, title: "Where should the new repo go?" }).catch(() => null);
      if (path) { s.parent = String(path); render(); }
      return;
    }
    if (d.su === "projects-dir") {
      const dir = await window.__TAURI__?.dialog?.open({ directory: true, multiple: false, title: "Where do you keep your projects?", defaultPath: s.repoInfo?.parent }).catch(() => null);
      if (!dir) return;
      try { const saved = await invoke("set_projects_folder", { path: String(dir) }); s.repoInfo.parent = s.parent = saved; toast("Clones go there from now on"); } catch (err) { toast(String(err), true); }
      return render();
    }
    if (d.su === "add-path") { const v = $("#su-add")?.value; await addRepo(v); return; }
    if (d.su === "rdel-main") {
      s.mainName = undefined;
      // The next repo steps up as the main one.
      const next = s.extra.shift();
      if (next) return checkRepo(next.path);
      s.repo = null; s.path = ""; s.checked = null; return render();
    }
    if (d.su.startsWith("rdel-")) { s.extra.splice(Number(d.su.slice(5)), 1); return render(); }
    if (d.su === "pick") {
      const path = await window.__TAURI__!.dialog!.open({ directory: true, multiple: false, title: "Pick the company's repo" }).catch(() => null);
      if (path) addRepo(String(path));
      return;
    }
  }
  if (d.retry) {
    el.disabled = true;
    await call("retry", { company: S.company, key: d.retry }, `${d.retry} is running again`).catch(() => (el.disabled = false));
    return;
  }
  if (d.hbEdit) { S.hbEdit = d.hbEdit === "1"; S.wikiNew = false; S.confirm = null; render(); return $("#hb-text")?.focus(); }
  if (d.wkPage !== undefined) { S.page = d.wkPage; S.hbEdit = false; S.wikiNew = false; S.history = null; S.version = null; render(); return document.querySelector(".wk-main")?.scrollTo(0, 0); }
  if (d.wkNew !== undefined) { S.wikiNew = true; S.wikiSpace = d.wkNew || currentPage()?.space; S.hbEdit = false; render(); return $("#wk-title")?.focus(); }
  if (d.wkHistory !== undefined) {
    if (!d.wkHistory) { S.history = null; S.version = null; return render(); }
    S.history = await invoke("page_history", { company: S.company, path: d.wkHistory }).catch(e => (toast(String(e), true), null));
    S.historyOf = d.wkHistory; S.version = S.history?.[0]?.id ?? null; return render();
  }
  if (d.wkVersion) { S.version = Number(d.wkVersion); return render(); }
  if (d.wkRestore) {
    const path = await call("restore_page", { company: S.company, id: Number(d.wkRestore) }, "Restored. Everyone reads it from their next ticket").catch(() => null);
    if (path) { S.history = null; S.version = null; S.page = path; render(); }
    return;
  }
  if (d.wkDelete) { await call("delete_page", { company: S.company, path: d.wkDelete }, "Page deleted. Its history stays").then(() => { S.hbEdit = false; S.confirm = null; S.page = null; render(); }).catch(() => {}); return; }
  if (d.learn) { await call("learn_repo", { company: S.company }, "Codebase updated from the repo").catch(() => {}); return; }
  if (d.diff) {
    S.diffs[d.diff] = "loading"; render();
    try { S.diffs[d.diff] = await invoke("review", { company: S.company, key: d.diff }); }
    catch (err) { delete S.diffs[d.diff]; toast(String(err), true); }
    return render();
  }
  if (d.hideDiff) { delete S.diffs[d.hideDiff]; return render(); }
  if (d.co) { S.switching = false; S.setup = null; S.company = d.co; S.snap = null; S.last = ""; render(); await call("use_company", { name: d.co }); }
  else if (d.view) { S.view = d.view; S.hbEdit = false; S.wikiNew = false; if (d.close) S.drawer = null; render(); if (d.view === "wiki") refresh(true); }
  else if (d.person) openPerson(d.person);
  else if (d.hire) { openHire(Number(d.hire)); }
  else if (d.addDept) { S.addingDept = true; S.view = "company"; render(); $("#dept-name")?.focus(); }
  else if (d.close) { S.drawer = null; renderDrawer(); }
  else if (d.answer) { el.disabled = true; await call("answer", { company: S.company, id: Number(d.answer), text: d.val }, d.said || (d.val === "no" ? "Left out of the wiki" : "Answered")).catch(() => (el.disabled = false)); }
  else if (d.approve) {
    const t = S.snap?.reviews.find(t => t.key === d.approve) || (S.ticket?.key === d.approve ? S.ticket : null);
    const copy = approvalCopy(t);
    el.disabled = true; el.textContent = copy.busy;
    await call("approve", { company: S.company, key: d.approve }, copy.done).catch(() => { el.disabled = false; el.textContent = copy.label; });
  }

});

document.addEventListener("submit", async e => {
  e.preventDefault();
  const f = e.target as HTMLFormElement;
  if (f.id === "plan-form") {
    const name = $("#pj-name").value.trim(), goal = $("#pj-goal").value.trim(), by = $("#pj-by").value || null;
    if (!name) return $("#pj-name").focus();
    if (!goal && !S.snap!.kickoff.brief) { toast("Say what the project should do", true); return $("#pj-goal").focus(); }
    const btn = f.querySelector<HTMLButtonElement>(".btn.primary")!; btn.disabled = true; btn.textContent = "Starting…";
    const key = await call("plan_project", { company: S.company, name, goal, by }, `${first(byId(by)?.name || "Your planner")} is planning ${name}. The plan shows up in your Inbox`).catch(() => null);
    if (!key) { btn.disabled = false; btn.textContent = "Plan it"; return; }
    S.drawer = null; S.view = "board"; S.project = key; return render();
  }
  if (f.id === "brief-form") {
    await call("save_brief", { company: S.company, brief: $("#brief-text").value }, "Brief saved. Everyone reads it from their next ticket").then(() => { S.open.brief = false; render(); }).catch(() => {});
    return;
  }
  if (f.id === "profile") {
    const firstTime = !!S.profile;
    if (!(await saveProfile(S.profile?.photo, S.profile?.noPhoto))) return;
    if (!firstTime) { S.pop = false; renderPop(); render(); return toast("Saved"); }
    // Back from setup: pick up where you were.
    if (S.profile!.inSetup) { S.profile = null; return render(); }
    S.profile = null;
    // Coming back to your companies, not starting a new one.
    if (S.companies.length) { toast(`Welcome back, ${myName()}`); return refresh(true); }
    S.onboarding = true;
    return startSetup();
  }
  if (f.id === "setup") {
    readSetup();
    const s = S.setup!;
    if (s.step === 0) {
      const ok = (s.ready || []).some(h => ["Claude Code", "Codex"].includes(h.what) && h.level === "ok");
      if (!ok) return toast("Connect Claude Code or Codex first. Your employees need one to work", true);
      s.step = 1; render(); return $("#su-name")?.focus();
    }
    if (s.step === 1) {
      if (!/^[A-Za-z0-9_-]+$/.test(s.name)) { toast("A company name can use letters, digits, - and _, like snowztech or my-notes", true); return $("#su-name")?.focus(); }
      if (S.companies.includes(s.name)) return toast(`There's already a company called ${s.name}`, true);
      if (!s.folder) s.folder = s.name.toLowerCase();
      s.step = 2; render(); return $("#su-path")?.focus();
    }
    if (s.step === 2) {
      // A path still in the box gets added first, so typing it and pressing Continue just works.
      const typed = $("#su-add")?.value.trim();
      if (typed) await addRepo(typed);
      if (!s.repo?.ok) { if (!typed) toast("Add the repo your team works in", true); return $("#su-add")?.focus(); }
      s.step = 3; render();
      return;
    }
    if (s.step === 3) {
      // The same check the core makes, early, so you fix it here and not after a failed create.
      const ids = s.people.filter(p => p.name.trim()).map(p => p.name.trim().split(/\s+/)[0].toLowerCase());
      const twice = ids.find((id, i) => ids.indexOf(id) !== i);
      if (twice) return toast(`Two people would both be ${twice}. Give one of them a different first name`, true);
      s.step = 4; return render();
    }
    // Launching: the real steps tick by while the company is set up, at a pace you can read.
    const people = s.people.filter(p => p.name.trim()), real = people.some(p => p.runtime !== "fake");
    const lead = people.find(p => /eng/i.test(p.dept || "")) || people[0];
    s.launch = { done: 0, steps: [`Founding ${s.name}`, people.length ? `Hiring ${people.length} ${people.length === 1 ? "person" : "people"}` : "Opening the office",
      `Connecting ${s.repo!.path.split("/").pop()}`, real && lead ? `${first(lead.name)} starts writing the wiki` : "Reading the repo",
      ...(s.brief && people.length ? ["Planning the first version"] : [])] };
    render();
    const tick = setInterval(() => { if (s.launch!.done < s.launch!.steps.length - 1) { s.launch!.done++; render(); } }, 420);
    const pace = new Promise(r => setTimeout(r, 420 * s.launch!.steps.length));
    try {
      const [name] = await Promise.all([invoke("create_company", { name: s.name, repo: s.repo!.path, repoName: s.mainName || null, repos: s.extra, mission: s.mission, people, brief: s.brief || null, color: s.color, logo: s.logo }), pace]);
      clearInterval(tick); s.launch!.done = s.launch!.steps.length; render();
      await new Promise(r => setTimeout(r, 450));
      S.setup = null; S.onboarding = false; S.company = name; S.view = "company"; S.snap = null; S.last = "";
      await refresh(true);
    } catch (e) { clearInterval(tick); s.launch = null; render(); toast(String(e), true); }
    return;
  }
  if (f.id === "budget-form") {
    const v = Number($("#budget-in").value || 0);
    S.customBudget = false;
    await call("set_budget", { company: S.company, dollars: v }, "Saved").catch(() => {});
    return loadSpend();
  }
  if (f.id === "you-form") { $("#pf-name").blur(); return; }
  if (f.id === "del-co") {
    const name = S.settings!.name;
    if ($("#del-name").value.trim() !== name) { toast(`Type ${name} to confirm`, true); return $("#del-name").focus(); }
    const btn = f.querySelector<HTMLButtonElement>(".danger")!; btn.disabled = true; btn.textContent = "Deleting…";
    try { await invoke("delete_company", { name }); } catch (err) { btn.disabled = false; btn.textContent = `Delete ${name}`; return toast(String(err), true); }
    toast(`Deleted ${name}. Your repo is untouched.`);
    S.company = null; S.snap = null; S.last = ""; S.settings = null; S.confirm = null; S.sheet = null; S.view = "company"; renderSheet();
    return refresh(true);
  }
  if (f.id === "say") {
    const input = $("#say-text"), text = input.value.trim();
    if (!text) return input.focus();
    input.value = ""; updateMentions(input); chatToEnd();
    await call("say", { company: S.company, channel: S.channel, text }).catch(() => { input.value = text; updateMentions(input); });
    return $("#say-text")?.focus();
  }
  if (f.id === "dept-form") {
    const name = $("#dept-name").value.trim();
    if (!name) return $("#dept-name").focus();
    const id = await call("add_department", { company: S.company, name }, `${name} is set up. Hire its first person.`).catch(() => null);
    if (id !== null) { S.addingDept = false; render(); openHire(id); }
  } else if (f.id === "wk-redraft") {
    await call("redraft_wiki", { company: S.company, by: $<HTMLSelectElement>("#wk-by").value || null }, "Drafting from the repo. Pages fill in as each draft lands").then(() => { S.open.redraft = false; render(); }).catch(() => {});
  } else if (f.id === "hb-raw") {
    const space = $("#wk-space").value.trim(), title = $("#wk-title").value.trim();
    if (!space) return $("#wk-space").focus();
    if (!title) return $("#wk-title").focus();
    const path = await call("save_page", { company: S.company, path: `${space}/${title}`, text: $("#hb-text").value, from: f.dataset.from || null }, "Saved. Everyone reads it from their next ticket").catch(() => null);
    if (path) { S.page = path; S.hbEdit = false; S.wikiNew = false; S.history = null; render(); }
  } else if (f.id === "hire-form" || f.id === "emp-form") {
    const p = readPerson();
    if (!p.name) { toast("Give them a name", true); return $("#pf-n").focus(); }
    const btn = f.querySelector<HTMLButtonElement>(".btn.primary")!; btn.disabled = true;
    try {
      if (f.id === "hire-form") {
        await invoke("hire", { company: S.company, ...p, department: S.drawer!.dept });
        toast(`${first(p.name)} joined. Give them a ticket whenever you're ready`); S.drawer = null; renderDrawer();
      } else {
        await invoke("update_employee", { company: S.company, id: S.drawer!.id, ...p });
        const f = S.drawer!.form!;
        await invoke("set_avatar", { company: S.company, id: S.drawer!.id, face: f.face === p.name ? null : f.face, photo: f.photoPath, removePhoto: !!f.removePhoto });
        await invoke("set_cap", { company: S.company, id: S.drawer!.id, dollars: Number($("#pf-cap")?.value || 0) });
        toast("Saved"); S.drawer!.edit = false; await openPerson(S.drawer!.id!);
      }
      await refresh(true);
    } catch (err) { btn.disabled = false; toast(String(err), true); }
  } else if (f.dataset.reply) {
    const input = f.querySelector("textarea")!, text = input.value.trim();
    if (!text) return input.focus();
    // Ticked, the answer also goes in the wiki as a decision, so nobody has to ask again.
    const keep = f.querySelector<HTMLInputElement>("[data-keep]");
    const q = S.snap?.inbox.find(x => x.id === Number(f.dataset.reply));
    const ok = await call("answer", { company: S.company, id: Number(f.dataset.reply), text }, keep?.checked ? "Answered, and saved to Decisions" : "Answered").then(() => true).catch(() => false);
    if (ok && keep?.checked && q) await invoke("add_to_wiki", { company: S.company, page: "Company/Decisions", text: `${q.question.split("\n")[0].slice(0, 160)} → ${text}` }).catch(e => toast(String(e), true));
  } else if (f.dataset.changes) {
    const input = f.querySelector("textarea")!, feedback = input.value.trim();
    if (!feedback) return input.focus();
    await call("request_changes", { company: S.company, key: f.dataset.changes, feedback }, "Sent back. It's saved to their memory too.").catch(() => {});
  }
});

// The drawn face follows your name as you type it.
export const faceLater: { t?: ReturnType<typeof setTimeout> } = {};

document.addEventListener("input", e => {
  const target = e.target as HTMLInputElement;
  // Filters the repo list in place: redrawing would take the cursor out of the search box.
  if (target.id === "gh-q") {
    const q = target.value.trim().toLowerCase();
    document.querySelectorAll<HTMLElement>(".gh-list li").forEach(li => (li.hidden = !!q && !(li.dataset.name || "").includes(q)));
    return;
  }
  if (target.id === "say-text") updateMentions(target);
  if (target.id === "su-name" && !S.setup?.logoUrl) { const b = $(".co-card .logo"); if (b) b.textContent = (target.value.trim() || "?")[0].toUpperCase(); }
  if (target.id === "hb-text") $("#hb-preview").innerHTML = md(target.value).html;
  if (target.id === "pf-n" && S.drawer?.type === "hire") {
    clearTimeout(faceLater.t);
    faceLater.t = setTimeout(() => { const pic = $(".hire-face"); if (pic) { pic.innerHTML = face(target.value || "New hire", "neutral"); pic.classList.remove("swap"); void pic.offsetWidth; pic.classList.add("swap"); } }, 400);
  }
  if (target.id === "pf-name" && S.profile && !hasPhoto()) { const m = $(".welcome .mono-av"); if (m) m.innerHTML = target.value.trim() ? esc(initials(target.value)) : icon("user"); }
});

document.addEventListener("change", async e => {
  const target = e.target as HTMLInputElement;
  if (target.id === "projects-path" && S.setup) {
    try { const saved = await invoke("set_projects_folder", { path: target.value }); S.setup!.repoInfo.parent = S.setup!.parent = saved; toast("Clones go there from now on"); } catch (err) { toast(String(err), true); }
    return render();
  }
  if (target.dataset?.cmdAdd && target.value.trim()) {
    // Pasting several lines, or a list from a Claude Code settings file, adds them all.
    const repo = target.dataset.cmdAdd, r = S.settings!.repos.find(x => x.name === repo)!;
    try { S.settings!.repos = await invoke("set_commands", { company: S.company, repo, text: [...r.commands, target.value].join("\n") }); saved(); } catch (err) { toast(String(err), true); }
    renderSheet();
    return $(`[data-cmd-add="${CSS.escape(repo)}"]`)?.focus();
  }

  if (target.dataset?.price) {
    await invoke("set_price", { company: S.company, runtime: target.dataset.price, perMillion: Math.max(0, Number(target.value) || 0) }).then(saved).catch(err => toast(String(err), true));
    return loadSpend();
  }
  if (target.id === "pf-name" && target.closest("#you-form")) {
    if (target.value.trim() && target.value.trim() !== S.founder?.name && await saveProfile()) { saved(); render(); }
    return;
  }
  if (target.dataset?.planAssign) {
    const employee = target.value;
    if (employee) await call("assign", { company: S.company, key: target.dataset.planAssign, employee }, `Given to ${first(byId(employee)?.name)}`).catch(() => {});
    return;
  }
  if (target.dataset?.assign) {
    const key = target.dataset.assign, employee = target.value;
    if (employee) await call("assign", { company: S.company, key, employee }, `Given to ${first(byId(employee)?.name)}`).catch(() => {});
    return openTicket(key);
  }
  if (target.id === "move" && target.value) {
    await call("move_to_department", { company: S.company, employee: target.dataset.move, department: Number(target.value) }, "Moved").catch(() => {});
    if (S.drawer?.type === "person") openPerson(S.drawer!.id!);
  }
});

// A one-line input flattens a pasted list, so a multi-line paste is added as it is.
document.addEventListener("paste", e => {
  const target = e.target as HTMLInputElement;
  const box = target.closest?.<HTMLInputElement>("[data-cmd-add]"), text = e.clipboardData?.getData("text") || "";
  if (!box || !text.includes("\n")) return;
  e.preventDefault();
  box.value = text;
  box.dispatchEvent(new Event("change", { bubbles: true }));
});

document.addEventListener("keydown", e => {
  const target = e.target as HTMLInputElement;
  // A reply under a card: ⌘↵ sends it, Esc closes it.
  const reply = target.closest?.<HTMLFormElement>("form.reply");
  if (reply && e.key === "Enter" && (e.metaKey || e.ctrlKey)) { e.preventDefault(); return reply.requestSubmit(); }
  if (reply && e.key === "Escape") { const cancel = reply.querySelector<HTMLElement>("[data-close-open]"); if (cancel) { e.preventDefault(); return cancel.click(); } }
  if (target.id === "say-text" && S.mention) {
    const n = S.mention.opts.length;
    if (e.key === "ArrowDown" || e.key === "ArrowUp") { e.preventDefault(); S.mention.i = (S.mention.i + (e.key === "ArrowDown" ? 1 : n - 1)) % n; return updateMentions(target); }
    if (e.key === "Enter" || e.key === "Tab") { e.preventDefault(); return pickMention(S.mention.i); }
    if (e.key === "Escape") { e.preventDefault(); S.mention = null; $("#mpop").hidden = true; return; }
  }
  if ((e.metaKey || e.ctrlKey) && (e.code === "Backslash" || e.key === "\\" || e.key.toLowerCase() === "b") && !target.closest?.("input, textarea")) { e.preventDefault(); return toggleRail(); }
  if (e.key === "Escape" && S.sheet) { S.sheet = null; return renderSheet(); }
  if (e.key === "Escape" && S.switching) { S.switching = false; return renderSide(); }
  if (e.key === "Escape" && S.pop) { S.pop = false; return renderPop(); }
  if (e.key === "Escape" && S.members) { S.members = null; return render(); }
  if (e.key === "Escape") { if (S.drawer) { S.drawer = null; renderDrawer(); } else if (S.addingDept) { S.addingDept = false; render(); } }
  if ((e.metaKey || e.ctrlKey) && e.key === "," && S.snap && !S.setup && !S.profile) { e.preventDefault(); return openSettings(); }
  if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k" && S.snap && !S.setup && !S.profile) { e.preventDefault(); S.drawer = { type: "new-ticket" }; renderDrawer(); $("#nt-title")?.focus(); }
});
