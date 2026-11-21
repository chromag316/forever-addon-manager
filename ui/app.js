"use strict";
// Forever AddOn Manager – window logic. Talks to the Rust side through Tauri commands.

const T = window.__TAURI__;
const invoke = (cmd, args) => (T ? T.core.invoke(cmd, args) : window.__mock(cmd, args));
const $ = (sel) => document.querySelector(sel);
const esc = (s) => String(s ?? "").replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));
const AUTO_INTERVAL_MS = 4 * 60 * 60 * 1000;

let ov = null;                // overview from Rust: installed, untracked, settings, …
let updates = new Map();      // modId -> { name, versionLabel, websiteOnly }
let busy = null;
let error = null;
let notice = null;
let tab = "installed";
let source = "curseforge";
let catalog = [];
let catalogDone = false;
let loadingCatalog = false;
let catalogFailed = false;
let lookedUp = null;

// ---------- helpers ----------

async function run(label, fn) {
  if (busy) return false;
  busy = label;
  error = null;
  render();
  let ok = true;
  try {
    await fn();
  } catch (e) {
    error = String(e);
    ok = false;
  } finally {
    busy = null;
    await refresh();
  }
  return ok;
}

async function refresh() {
  try {
    ov = await invoke("overview");
  } catch (e) {
    error = String(e);
  }
  render();
}

const openUrl = (url) => url && (T ? T.opener.openUrl(url) : window.open(url));
const sep = () => (ov.settings.addonsPath.includes("\\") ? "\\" : "/");
const addonPath = (folder) => ov.settings.addonsPath.replace(/[\\/]+$/, "") + sep() + folder;
const reveal = (path) => T && T.opener.revealItemInDir(path);
const isInstalled = (id) => ov.installed.some((a) => a.modId === id);
const compact = new Intl.NumberFormat(undefined, { notation: "compact" });

function icon(id) {
  return `<svg><use href="#${id}"/></svg>`;
}

// ---------- rendering ----------

function render() {
  if (!ov) return;
  renderToolbar();
  renderBanners();
  renderInstalled();
  renderBrowse();
}

function renderToolbar() {
  $("#busy").hidden = !busy;
  $("#busy-text").textContent = busy || "";
  $("#btn-scan").disabled = !!busy || !ov.hasKey;
  $("#btn-update-all").disabled = !!busy || ![...updates.values()].some((u) => !u.websiteOnly);
  document.querySelectorAll(".toolbar .segmented button").forEach((b) => b.classList.toggle("active", b.dataset.tab === tab));
  $("#view-installed").hidden = tab !== "installed";
  $("#view-browse").hidden = tab !== "browse";
}

function renderBanners() {
  const parts = [];
  if (!ov.hasKey && tab !== "installed") {
    parts.push(`<div class="banner info">${icon("i-key")}<span class="text">Add your free CurseForge API key to get started.</span>
      <button data-url="https://console.curseforge.com/">Get a key</button><button data-action="settings">Open Settings…</button></div>`);
  }
  const close = (what) => `<button class="close" data-dismiss="${what}" title="Dismiss" aria-label="Dismiss">${icon("i-close")}</button>`;
  // Key problems get a direct way to fix them.
  const keyIssue = error && /API key/.test(error) ? `<button data-action="settings">Open Settings</button>` : "";
  if (error) parts.push(`<div class="banner warning" role="alert">${icon("i-warning")}<span class="text">${esc(error)}</span>${keyIssue}${close("error")}</div>`);
  if (notice) parts.push(`<div class="banner notice" role="status">${icon("i-info")}<span class="text">${esc(notice)}</span>${close("notice")}</div>`);
  $("#banners").innerHTML = parts.join("");
}

function subtitle(a) {
  if (a.kind === "curseforge") return [a.author, a.version].filter(Boolean).join(" · ");
  const src = (a.sourceURL || "").replace("https://github.com/", "");
  return [a.displayName, src].filter(Boolean).join(" · ");
}

function thumb(a) {
  if (a.kind === "git") return `<div class="thumb">${icon("i-git")}</div>`;
  if (a.kind === "folder") return `<div class="thumb">${icon("i-folder")}</div>`;
  return a.thumbnailUrl ? `<img class="thumb" src="${esc(a.thumbnailUrl)}" alt="">` : `<div class="thumb">${icon("i-piece-line")}</div>`;
}

function addonRow(a) {
  const u = updates.get(a.modId);
  let status;
  if (a.ignored) status = `<span class="muted">Updates off</span>`;
  else if (u && u.websiteOnly) status = `<button class="btn small" data-action="website" data-id="${a.modId}" title="This author only allows downloads from CurseForge. Download the zip, then drop it on this window.">${esc(u.versionLabel)} on website</button>`;
  else if (u) status = `<button class="btn small primary" data-action="update" data-id="${a.modId}" ${busy ? "disabled" : ""}>Update to ${esc(u.versionLabel)}</button>`;
  else status = `<span class="ok" title="Up to date">${icon("i-check")}</span>`;
  return `<li class="row">${thumb(a)}
    <div class="info"><div class="name">${esc(a.name)}</div><div class="sub">${esc(subtitle(a))}</div></div>
    ${status}
    <button class="icon-btn" data-action="menu" data-id="${a.modId}" title="More">${icon("i-dots")}</button></li>`;
}

function renderInstalled() {
  const el = $("#view-installed");
  if (!ov.hasKey) return renderWelcome(el);
  el.classList.remove("welcome-view");
  if (!ov.installed.length && !ov.untracked.length) {
    el.innerHTML = `<div class="empty"><svg viewBox="-4 -4 108 108"><use href="#i-piece"/></svg>
      <h3>No addons found</h3><p>${esc(ov.settings.addonsPath)}</p>
      ${ov.addonsPathExists ? "" : `<p style="margin-top:10px">This folder doesn't exist. <button class="link" data-action="settings">Choose your AddOns folder…</button></p>`}</div>`;
    return;
  }
  const groups = [["curseforge", "CurseForge"], ["git", "Git"], ["folder", "Folder"]];
  let html = "";
  for (const [kind, label] of groups) {
    const items = ov.installed.filter((a) => a.kind === kind);
    if (items.length) html += `<div class="section-title">${label} (${items.length})</div><ul class="list">${items.map(addonRow).join("")}</ul>`;
  }
  if (ov.untracked.length) {
    html += `<div class="section-title">Other folders</div><ul class="list">${ov.untracked.map((f) => `<li class="row">
      <div class="thumb">${icon("i-folder")}</div><div class="info"><div class="name">${esc(f)}</div></div>
      <span class="muted">not managed</span>
      <button class="icon-btn" data-action="menu-folder" data-folder="${esc(f)}" title="More">${icon("i-dots")}</button></li>`).join("")}</ul>`;
  }
  el.innerHTML = html;
}

// First launch without an API key: explain why a key is needed and take it right here.
function renderWelcome(el) {
  if (!el.classList.contains("welcome-view")) {
    el.classList.add("welcome-view");
    el.innerHTML = `<div class="welcome">
      <img class="welcome-icon" src="app-icon.png" alt="">
      <h2>Welcome to Forever AddOn Manager</h2>
      <p class="lead">To find, install and update your WoW Forever addons, the app needs a free CurseForge API key. It only takes a minute.</p>
      <ol class="steps">
        <li><span class="num">1</span><div><strong>Get a free key</strong>
          <p>Sign in at CurseForge for Studios and copy your API key.</p>
          <button class="btn" data-url="https://console.curseforge.com/">Get a free key</button></div></li>
        <li><span class="num">2</span><div><strong>Paste it here</strong>
          <div class="field-row"><input id="welcome-key" type="password" placeholder="Your CurseForge API key" autocomplete="off" spellcheck="false">
          <button id="welcome-save" class="btn primary">Save Key</button></div>
          <p id="welcome-msg" class="welcome-msg" hidden></p></div></li>
      </ol>
      <p class="folder">Your addons will be read from <span id="welcome-path"></span>
        <button class="link" data-action="settings">Change…</button></p>
      <p id="welcome-path-warn" class="welcome-msg" hidden>This folder doesn't exist yet. Choose your WoW Forever <em>Interface/AddOns</em> folder.</p>
      <p class="muted">You can already install addons from Git or a folder on the Get Addons tab without a key.</p>
    </div>`;
    const input = $("#welcome-key");
    input.addEventListener("input", () => { $("#welcome-save").disabled = !input.value.trim(); $("#welcome-msg").hidden = true; });
    input.addEventListener("keydown", (e) => e.key === "Enter" && saveWelcomeKey());
    $("#welcome-save").onclick = saveWelcomeKey;
    $("#welcome-save").disabled = true;
  }
  $("#welcome-path").textContent = ov.settings.addonsPath;
  $("#welcome-path-warn").hidden = ov.addonsPathExists;
}

async function saveWelcomeKey() {
  const input = $("#welcome-key"), msg = $("#welcome-msg"), save = $("#welcome-save");
  const key = input.value.trim();
  if (!key) return;
  save.disabled = true;
  msg.hidden = false;
  msg.textContent = "Checking the key with CurseForge…";
  try {
    // Check first, save only if CurseForge accepts it.
    const report = await invoke("test_key", { key });
    if (!report.includes("✅")) {
      msg.textContent = report.split("\n").filter((l) => !l.startsWith("Key length")).map((l) =>
        l.includes("Network error") ? "Couldn't connect to CurseForge. Check your internet connection or firewall."
        : l.includes("HTTP") ? "CurseForge didn't accept this key." : l).join(" ")
        || "CurseForge didn't accept this key.";
      save.disabled = false;
      return;
    }
    await invoke("set_key", { key });
    input.value = "";
    await startup(); // key accepted: scan the AddOns folder and check for updates
  } catch (e) {
    msg.textContent = String(e);
    save.disabled = false;
  }
}

function catalogRow(m) {
  let action;
  if (isInstalled(m.id)) action = `<span class="muted">Installed</span>`;
  else if (m.allowModDistribution === false) action = `<button class="btn small" data-action="open" data-url="${esc(websiteOf(m))}" title="This author only allows downloads from CurseForge. Download the zip, then drop it on this window.">Website</button>`;
  else action = `<button class="btn small" data-action="install" data-id="${m.id}" ${busy ? "disabled" : ""}>Install</button>`;
  const logo = m.logo && m.logo.thumbnailUrl ? `<img class="thumb" src="${esc(m.logo.thumbnailUrl)}" alt="">` : `<div class="thumb">${icon("i-piece-line")}</div>`;
  return `<li class="row">${logo}<div class="info"><div class="name">${esc(m.name)}</div><div class="sub summary">${esc(m.summary)}</div></div>
    ${m.downloadCount ? `<span class="count">${compact.format(m.downloadCount)}</span>` : ""}${action}</li>`;
}

function websiteOf(m) {
  return (m.links && m.links.websiteUrl) || (m.slug ? `https://www.curseforge.com/wow/addons/${m.slug}` : "");
}

function renderBrowse() {
  document.querySelectorAll("#source-picker button").forEach((b) => b.classList.toggle("active", b.dataset.source === source));
  for (const s of ["curseforge", "git", "folder"]) $(`#src-${s}`).hidden = s !== source;

  const q = $("#cf-filter").value.trim();
  const projectId = /^\d+$/.test(q) ? Number(q) : null;
  let results;
  if (projectId) results = lookedUp && lookedUp.id === projectId ? [lookedUp] : [];
  else if (q) results = catalog.filter((m) => (m.name + " " + (m.summary || "")).toLowerCase().includes(q.toLowerCase()));
  else results = catalog;
  $("#cf-list").innerHTML = results.map(catalogRow).join("");
  $("#cf-retry").hidden = !catalogFailed || loadingCatalog;
  const noMatch = q && !projectId && !results.length;
  $("#cf-note").textContent = !ov.hasKey ? "Add your API key in Settings to browse CurseForge."
    : projectId ? ""
    : catalogFailed ? "Couldn't load more addons."
    : noMatch ? (catalogDone ? `No addons match “${q}”.` : `No addons match “${q}” yet. Still loading…`)
    : loadingCatalog ? (catalog.length ? "Loading more…" : "Loading…")
    : catalogDone ? `All ${catalog.length} addons loaded`
    : "";

  $("#git-list").innerHTML = ov.installed.filter((a) => a.kind === "git").map(addonRow).join("");
  $("#folder-list").innerHTML = ov.installed.filter((a) => a.kind === "folder").map(addonRow).join("");
  $("#git-install").disabled = !!busy || !$("#git-url").value.trim();
  $("#folder-install").disabled = !!busy || !$("#folder-path").value.trim();
}

// ---------- actions ----------

async function scan() {
  await run("Scanning AddOns…", async () => {
    await invoke("scan");
    await loadUpdates();
  });
}

async function loadUpdates() {
  const list = await invoke("check_updates");
  updates = new Map(list.map((u) => [u.modId, u]));
}

async function checkUpdates() {
  await run("Checking for updates…", loadUpdates);
}

async function updateAll(auto) {
  if (![...updates.values()].some((u) => !u.websiteOnly)) return;
  await run("Updating…", async () => {
    const r = await invoke("update_all");
    for (const [id, u] of updates) if (r.updated.includes(u.name)) updates.delete(id);
    if (r.updated.length) notice = `${auto ? "Automatically updated" : "Updated"} ${r.updated.length} addon${r.updated.length === 1 ? "" : "s"}: ${r.updated.join(", ")}`;
    if (r.failed.length) throw new Error(`Couldn't update: ${r.failed.join(", ")}. Try its Update button.`);
  });
}

async function autoUpdate() {
  if (ov && ov.autoUpdates) await updateAll(true);
}

async function startup() {
  await refresh();
  if (ov.hasKey && !ov.installed.length) await scan();
  else await checkUpdates();
  await autoUpdate();
}

async function loadMore(batches) {
  if (loadingCatalog || catalogDone || !ov.hasKey) return;
  loadingCatalog = true;
  catalogFailed = false;
  render();
  try {
    for (let i = 0; i < batches; i++) {
      const page = await invoke("browse", { excluding: catalog.map((m) => m.id) });
      if (!page.length) { catalogDone = true; break; }
      catalog.push(...page);
    }
  } catch (e) {
    // Stop auto-loading until the user presses Retry, so a network problem can't loop.
    catalogFailed = true;
    error = String(e);
  }
  loadingCatalog = false;
  render();
  fillIfNeeded();
}

// Infinite scroll: load the next batch whenever the list footer is near the visible area.
function footerNearView() {
  const footer = $("#cf-sentinel");
  if (footer.offsetParent === null) return false; // tab or source not visible
  const main = $("main").getBoundingClientRect();
  return footer.getBoundingClientRect().top < main.bottom + 600;
}

function fillIfNeeded() {
  const browsingCatalog = tab === "browse" && source === "curseforge" && !/^\d+$/.test($("#cf-filter").value.trim());
  if (browsingCatalog && !catalogFailed && footerNearView()) loadMore(1);
}

function showMenu(button, items) {
  const menu = $("#menu");
  menu.innerHTML = items.map((it, i) => (it === "-" ? "<hr>" : `<button data-i="${i}" class="${it.danger ? "danger" : ""}">${esc(it.label)}</button>`)).join("");
  menu.hidden = false;
  const r = button.getBoundingClientRect();
  const w = menu.offsetWidth, h = menu.offsetHeight;
  menu.style.left = Math.max(8, Math.min(r.right - w, innerWidth - w - 8)) + "px";
  menu.style.top = (r.bottom + h + 8 > innerHeight ? r.top - h - 4 : r.bottom + 4) + "px";
  menu.onclick = (e) => {
    const b = e.target.closest("button[data-i]");
    if (!b) return;
    menu.hidden = true;
    items[Number(b.dataset.i)].run();
  };
}

function addonMenu(button, a) {
  const items = [];
  if (a.websiteUrl) items.push({ label: a.kind === "git" ? "Open Repository" : "Open on CurseForge", run: () => openUrl(a.websiteUrl) });
  if (a.kind !== "curseforge") items.push({ label: a.kind === "git" ? "Reinstall from Git" : "Reinstall from Folder", run: () => run(`Reinstalling ${a.name}…`, () => invoke("reinstall", { modId: a.modId }).then(() => updates.delete(a.modId))) });
  if (a.kind === "folder" && a.sourceURL) items.push({ label: "Show Source Folder", run: () => reveal(a.sourceURL) });
  if (a.folders.length) items.push({ label: "Show in Folder", run: () => reveal(addonPath(a.folders[0])) });
  items.push({ label: a.ignored ? "Turn Updates On" : "Turn Updates Off", run: async () => {
    await invoke("set_ignored", { modId: a.modId, ignored: !a.ignored });
    if (!a.ignored) updates.delete(a.modId);
    await refresh();
  } });
  items.push("-", { label: "Uninstall", danger: true, run: async () => {
    const yes = T ? await T.dialog.ask(`Remove ${a.name} (${a.folders.join(", ")}) from your AddOns folder?`, { title: "Uninstall", kind: "warning" }) : confirm(`Uninstall ${a.name}?`);
    if (yes) await run(`Removing ${a.name}…`, () => invoke("uninstall", { modId: a.modId }).then(() => updates.delete(a.modId)));
  } });
  showMenu(button, items);
}

// ---------- settings ----------

async function openSettings() {
  const s = ov.settings;
  $("#set-key").value = "";
  $("#set-key").placeholder = ov.hasKey ? "Saved (paste a new key to replace it)" : "Paste your key";
  $("#key-report").hidden = true;
  $("#set-path").value = s.addonsPath;
  $("#set-release").value = String(s.releaseType);
  $("#set-auto").checked = ov.autoUpdates;
  $("#app-version").textContent = `Version ${ov.appVersion}`;
  $("#settings").showModal();
  if (ov.hasKey && $("#set-gvt").options.length < 2) {
    try {
      const types = await invoke("version_types");
      $("#set-gvt").innerHTML = types.map((t) => `<option value="${t.id}">${esc(t.name)} (${t.id})</option>`).join("");
    } catch { /* keep the default option */ }
  }
  $("#set-gvt").value = String(s.gameVersionTypeId);
}

async function saveSettings() {
  const wasAuto = ov.autoUpdates;
  await invoke("save_settings", { settings: {
    addonsPath: $("#set-path").value.trim(),
    releaseType: Number($("#set-release").value),
    gameVersionTypeId: Number($("#set-gvt").value),
    autoUpdates: $("#set-auto").checked,
  } });
  await refresh();
  if (!wasAuto && ov.autoUpdates) { await checkUpdates(); await autoUpdate(); }
}

// ---------- events ----------

document.addEventListener("click", async (e) => {
  const menu = $("#menu");
  if (!menu.hidden && !e.target.closest("#menu")) menu.hidden = true;

  const t = e.target.closest("[data-tab],[data-source],[data-action],[data-dismiss],[data-url]");
  if (!t) return;
  if (t.dataset.tab) {
    tab = t.dataset.tab;
    render();
    if (tab === "browse" && !catalog.length) loadMore(3);
    return;
  }
  if (t.dataset.source) { source = t.dataset.source; render(); fillIfNeeded(); return; }
  if (t.dataset.dismiss) { if (t.dataset.dismiss === "error") error = null; else notice = null; render(); return; }
  const id = Number(t.dataset.id);
  switch (t.dataset.action) {
    case "settings": openSettings(); break;
    case "update": {
      const u = updates.get(id);
      run(`Updating ${u ? u.name : ""}…`, () => invoke("update", { modId: id }).then(() => updates.delete(id)));
      break;
    }
    case "website": openUrl((ov.installed.find((a) => a.modId === id) || {}).websiteUrl); break;
    case "install": {
      const m = catalog.concat(lookedUp ? [lookedUp] : []).find((x) => x.id === id);
      run(`Installing ${m ? m.name : ""}…`, () => invoke("install_mod", { modId: id }));
      break;
    }
    case "open": openUrl(t.dataset.url); break;
    case "menu": addonMenu(t, ov.installed.find((a) => a.modId === id)); break;
    case "menu-folder": showMenu(t, [{ label: "Show in Folder", run: () => reveal(addonPath(t.dataset.folder)) }]); break;
    default:
      if (t.dataset.url) { e.preventDefault(); openUrl(t.dataset.url); }
  }
});

$("#btn-scan").onclick = scan;
$("#btn-update-all").onclick = () => updateAll(false);
$("#btn-settings").onclick = openSettings;
$("#cf-retry").onclick = () => { catalogFailed = false; loadMore(1); };
$("main").addEventListener("scroll", fillIfNeeded, { passive: true });
window.addEventListener("resize", fillIfNeeded);

let lookupTimer;
$("#cf-filter").addEventListener("input", () => {
  render();
  fillIfNeeded();
  const q = $("#cf-filter").value.trim();
  clearTimeout(lookupTimer);
  if (/^\d+$/.test(q)) {
    lookupTimer = setTimeout(async () => {
      try { lookedUp = await invoke("lookup", { modId: Number(q) }); } catch (e) { error = String(e); }
      render();
    }, 300);
  }
});

const installGit = async () => {
  const url = $("#git-url").value.trim();
  if (url && (await run("Installing from Git…", () => invoke("install_git", { url })))) $("#git-url").value = "";
  render();
};
$("#git-url").addEventListener("input", render);
$("#git-url").addEventListener("keydown", (e) => e.key === "Enter" && installGit());
$("#git-install").onclick = installGit;

const installFolder = async () => {
  const path = $("#folder-path").value.trim();
  if (path && (await run("Copying folder…", () => invoke("install_folder", { path })))) $("#folder-path").value = "";
  render();
};
$("#folder-path").addEventListener("input", render);
$("#folder-path").addEventListener("keydown", (e) => e.key === "Enter" && installFolder());
$("#folder-install").onclick = installFolder;
$("#folder-choose").onclick = async () => {
  if (!T) return;
  const dir = await T.dialog.open({ directory: true, title: "Choose an addon folder (it contains a .toc file)" });
  if (dir) { $("#folder-path").value = dir; render(); }
};

$("#set-key-save").onclick = async () => {
  const hadKey = ov.hasKey;
  const key = $("#set-key").value.trim();
  try {
    if (key) {
      // Check with CurseForge first; don't replace a working key with one it rejects.
      $("#key-report").hidden = false;
      $("#key-report").textContent = "Checking the key with CurseForge…";
      const report = await invoke("test_key", { key });
      if (!report.includes("✅")) {
        $("#key-report").textContent = report + "\n\nThe key was not saved.";
        return;
      }
    }
    await invoke("set_key", { key });
    $("#set-key").value = "";
    $("#key-report").hidden = true;
    await refresh();
    $("#set-key").placeholder = ov.hasKey ? "Saved (paste a new key to replace it)" : "Paste your key";
    if (!hadKey && ov.hasKey) startup();
  } catch (e) { error = String(e); render(); }
};
$("#set-key-test").onclick = async () => {
  // Tests the key typed in the field, or the saved key if the field is empty.
  const key = $("#set-key").value.trim();
  $("#key-report").hidden = false;
  $("#key-report").textContent = "Testing…";
  $("#key-report").textContent = await invoke("test_key", key ? { key } : {}).catch(String);
};
$("#set-path-choose").onclick = async () => {
  if (!T) return;
  const dir = await T.dialog.open({ directory: true, defaultPath: $("#set-path").value, title: "Choose the Interface/AddOns folder" });
  if (dir) { $("#set-path").value = dir; saveSettings(); }
};
for (const id of ["#set-path", "#set-release", "#set-gvt", "#set-auto"]) $(id).addEventListener("change", saveSettings);

// Drag and drop zips (for addons whose authors only allow downloads from CurseForge).
if (T) {
  T.webview.getCurrentWebview().onDragDropEvent(async (ev) => {
    const p = ev.payload;
    const zips = (p.paths || []).filter((x) => x.toLowerCase().endsWith(".zip"));
    if (p.type === "enter") $("#drop").hidden = !zips.length;
    else if (p.type === "leave") $("#drop").hidden = true;
    else if (p.type === "drop") {
      $("#drop").hidden = true;
      if (!zips.length || busy) return;
      tab = "installed";
      await run("Installing from zip…", async () => {
        await invoke("install_zips", { paths: zips });
        await loadUpdates();
      });
    }
  });
}

document.addEventListener("keydown", (e) => {
  if ((e.metaKey || e.ctrlKey) && e.key === ",") { e.preventDefault(); openSettings(); }
  if (e.key === "Escape") $("#menu").hidden = true;
});

// ---------- start ----------

async function boot() {
  if (!T) {
    // Opened in a plain browser (design preview): load sample data.
    await new Promise((res) => { const s = document.createElement("script"); s.src = "dev-mock.js"; s.onload = res; document.head.append(s); });
  }
  await startup();
  setInterval(async () => {
    if (!ov || !ov.autoUpdates || busy) return;
    await checkUpdates();
    await autoUpdate();
  }, AUTO_INTERVAL_MS);
}
boot();
