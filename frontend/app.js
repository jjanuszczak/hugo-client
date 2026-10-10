let pages = {
  home: {
    title: "Welcome to Northstar",
    summary: "A journal about place, people, and the small details that help us find our way.",
    path: "content/_index.md",
    type: "Page",
    section: "Home",
    tags: ["Welcome", "Journal"],
    published: true,
    body: [
      ["p", "Every place has a rhythm. You hear it in the market opening before sunrise, in the ferry that leaves on time, and in the familiar hello exchanged on the walk home."],
      ["h2", "Start with what is here"],
      ["p", "Northstar is a space for careful observation and useful stories. We look closely at the people, landscapes, and everyday decisions that shape a community over time."],
      ["quote", "“The best way to understand a place is to pay attention to what its people already know.”"],
      ["p", "In the weeks ahead, we will share field notes, conversations, and practical resources from across the coast."],
    ],
  },
  "field-notes": {
    title: "Field notes from the coast",
    summary: "A short dispatch from the shoreline, where the day begins with the tide.",
    path: "content/posts/field-notes/index.md",
    type: "Post",
    section: "Posts",
    tags: ["Field Notes", "Coast"],
    published: true,
    body: [
      ["p", "By the time the first boat reaches the pier, the stalls are open and the day has already found its pace."],
      ["h2", "A working shoreline"],
      ["p", "The harbor is not a backdrop. It is a workplace, a meeting point, and a source of practical knowledge passed between generations."],
      ["quote", "“The tide sets the schedule. We just learn how to read it.”"],
      ["p", "These notes are part of an ongoing series about how coastal communities adapt to changing seasons."],
    ],
  },
  community: {
    title: "Building a local community",
    summary: "What makes a place feel like it belongs to the people who live there?",
    path: "content/posts/community/index.md",
    type: "Post",
    section: "Posts",
    tags: ["Community", "Draft"],
    published: false,
    body: [
      ["p", "A first draft on the institutions, shared spaces, and small acts that help a community hold together."],
      ["h2", "The places between home and work"],
      ["p", "Libraries, local markets, and neighborhood groups create room for people to solve everyday problems together."],
      ["quote", "Draft note: confirm the examples and add a source before publication."],
      ["p", "This page is not published yet. The draft stays local until the author chooses to publish it through their existing workflow."],
    ],
  },
  about: {
    title: "About the journal",
    summary: "Northstar is an independent journal of people and place.",
    path: "content/about/index.md",
    type: "Page",
    section: "Pages",
    tags: ["About"],
    published: true,
    body: [
      ["p", "Northstar publishes thoughtful reporting and useful guides for people who care about their communities."],
      ["h2", "Our approach"],
      ["p", "We work from local knowledge, clear sourcing, and a belief that the details matter."],
      ["quote", "Good reporting starts with listening, then checks what it hears."],
      ["p", "Have a story idea? Use the contact link in the site footer."],
    ],
  },
  resources: {
    title: "Resources and links",
    summary: "A collection of practical references for readers and contributors.",
    path: "content/resources/index.md",
    type: "Page",
    section: "Resources",
    tags: ["Resources", "Guide"],
    published: true,
    body: [
      ["p", "These references are selected by the Northstar editorial team and updated as the work changes."],
      ["h2", "For contributors"],
      ["p", "Our contributor guide explains how to pitch a story and what to expect from the editing process."],
      ["quote", "A useful resource should help someone take the next step."],
      ["p", "Links in this sample are illustrative. Real links and local assets will come from the Hugo manifest."],
    ],
  },
};

let selectedPage = "home";
let previewMode = "native";
let inspectorTab = "overview";
let usingDemoData = true;
let projectMainMenu = [];
let projectFooterMenu = [];
let projectNavigationSnapshot = null;
let projectPreviewUrl = "";
let selectedProjectPath = "";
let toastTimer;

const $ = (selector) => document.querySelector(selector);
const $$ = (selector) => [...document.querySelectorAll(selector)];
const overviewMarkup = $("#inspector-content").innerHTML;

function syncInspectorTabSelection() {
  $$(".inspector-tab").forEach((tab) => {
    const selected = tab.dataset.inspectorTab === inspectorTab;
    tab.classList.toggle("active", selected);
    tab.setAttribute("aria-selected", String(selected));
  });
}

function showToast(message) {
  const toast = $("#toast");
  toast.textContent = message;
  toast.classList.add("show");
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => toast.classList.remove("show"), 2400);
}

function renderBody(page) {
  return page.body.map(([kind, text]) => {
    if (kind === "quote") return `<blockquote>${escapeHtml(text)}</blockquote>`;
    if (/^h[1-6]$/.test(kind)) return `<${kind}>${escapeHtml(text)}</${kind}>`;
    if (kind === "list") return `<ul>${text.split("\n").map((item) => `<li>${escapeHtml(item)}</li>`).join("")}</ul>`;
    return `<p>${escapeHtml(text)}</p>`;
  }).join("");
}

function manifestInlineText(inlines = []) {
  return inlines.map((inline) => inline.text || manifestInlineText(inline.children || [])).join("");
}

function manifestBlockText(block) {
  return typeof block.text === "string" ? block.text : manifestInlineText(block.inlines || []);
}

function manifestPageToView(page) {
  const body = (page.blocks || []).flatMap((block) => {
    if (block.type === "heading") return [[`h${block.level || 2}`, manifestBlockText(block)]];
    if (block.type === "quote") return [["quote", manifestBlockText(block)]];
    if (block.type === "list") {
      const items = (block.items || []).map((item) => item.map(manifestBlockText).join(""));
      return items.length ? [["list", items.join("\n")]] : [];
    }
    return block.type === "paragraph" ? [["p", manifestBlockText(block)]] : [];
  });
  return {
    id: page.id,
    title: page.title,
    summary: page.summary || "",
    path: page.canonicalURL || page.path || "/",
    source_path: page.sourcePath || page.id,
    kind: page.kind || "page",
    type: page.kind || "page",
    section: page.section || "",
    date: page.date || "",
    draft: Boolean(page.draft),
    published: !page.draft,
    tags: [...(page.taxonomies?.tags || []), ...(page.taxonomies?.categories || [])],
    body,
    has_unsupported_blocks: page.rendering?.mode !== "native",
  };
}

function escapeHtml(value) {
  return value.replace(/[&<>"']/g, (character) => ({
    "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;",
  })[character]);
}

function updatePage(pageId, { addTab = false } = {}) {
  const page = pages[pageId];
  if (!page) return;
  selectedPage = pageId;
  $("#breadcrumb-title").textContent = page.title;
  $("#article-title").textContent = page.title;
  $("#article-summary").textContent = page.summary;
  $("#article-body").innerHTML = renderBody(page);
  $("#article-kicker").textContent = page.section ? page.section.toUpperCase() : "SITE CONTENT";
  $("#field-title").textContent = page.title;
  $("#source-path").textContent = page.source_path || page.path;
  const isDraft = Boolean(page.draft) || page.published === false;
  $(".published-pill").textContent = isDraft ? "DRAFT" : "PUBLISHED";
  $(".published-pill").classList.toggle("draft", isDraft);
  const contentType = page.kind || page.type;
  $("#field-type").textContent = contentType ? contentType[0].toUpperCase() + contentType.slice(1) : "Page";
  $("#field-section").textContent = page.section || "Home";
  $("#field-draft").textContent = isDraft ? "Yes" : "No";
  $(".tag-list").innerHTML = (page.tags || []).map((tag) => `<span class="tag">${escapeHtml(tag)}</span>`).join("") || "<span class='muted'>No tags</span>";
  $("#field-date").textContent = usingDemoData ? "Oct 10, 2026" : formatDate(page.date);
  $(".rendering-card>div span").textContent = page.has_unsupported_blocks
    ? "Theme content is omitted from this native view"
    : usingDemoData ? "Illustrative demo blocks" : "Basic Markdown blocks shown natively";
  $("#render-label").textContent = previewMode === "native" ? "NATIVE PREVIEW" : "WEBSITE PREVIEW";
  $("#page-preview").classList.toggle("web-preview", previewMode === "web");
  const hasWebsitePreview = previewMode === "web" && !usingDemoData && Boolean(projectPreviewUrl);
  $("#page-preview").classList.toggle("hugo-website", hasWebsitePreview);
  $("#website-frame").hidden = !hasWebsitePreview;
  if (hasWebsitePreview) {
    let pagePath = "/";
    try {
      pagePath = new URL(page.path).pathname;
    } catch {
      pagePath = page.path || "/";
    }
    $("#website-frame").src = new URL(pagePath, projectPreviewUrl).href;
  }
  $(".site-masthead").classList.toggle("native-site-chrome", previewMode === "native" && usingDemoData);
  $("#page-preview .native-omission")?.remove();
  $("#page-preview .unsupported-content")?.remove();
  $("#hero-image").hidden = !usingDemoData;
  $("#demo-byline").hidden = !usingDemoData;
  if (previewMode === "native" && usingDemoData) $("#article-body").insertAdjacentHTML("beforebegin", "<div class='native-omission'>Native preview shows declared content. Full theme header and footer are shown in Website mode.</div>");
  if (page.has_unsupported_blocks) $("#article-body").insertAdjacentHTML("beforebegin", "<div class='unsupported-content'>This page contains Hugo shortcodes, raw HTML, or media blocks. They are omitted from the basic native preview.</div>");
  if (previewMode === "web" && !usingDemoData) $("#article-body").innerHTML = "<div class='unsupported-content'>The Hugo website renderer is not connected yet. This page is shown using supported native Markdown blocks.</div>" + renderBody(page);
  $$(".page-row").forEach((row) => row.classList.toggle("current", row.dataset.page === pageId));
  $$(".tab[data-page]").forEach((tab) => tab.classList.toggle("active", tab.dataset.page === pageId));
  if (addTab) addOrActivateTab(pageId);
  if (inspectorTab === "validation") renderValidation();
}

function formatDate(value) {
  if (!value) return "Not set";
  const date = new Date(value);
  return Number.isNaN(date.valueOf()) || date.getUTCFullYear() < 1971
    ? "Not set"
    : new Intl.DateTimeFormat(undefined, { dateStyle: "medium", timeZone: "UTC" }).format(date);
}

function renderProjectNavigation(snapshot) {
  const groups = new Map();
  for (const [id, page] of Object.entries(pages)) {
    const name = page.section || (page.kind === "home" ? "Home" : "Pages");
    if (!groups.has(name)) groups.set(name, []);
    groups.get(name).push([id, page]);
  }
  const list = $("#page-list");
  list.replaceChildren();
  for (const [name, items] of groups) {
    const label = document.createElement("div");
    label.className = "content-group-label";
    label.textContent = name;
    list.append(label);
    for (const [id, page] of items) {
      const row = document.createElement("button");
      row.className = "page-row" + (id === selectedPage ? " current" : "");
      row.dataset.page = id;
      const dot = document.createElement("span");
      dot.className = "page-dot " + (page.draft ? "draft" : "published");
      const title = document.createElement("span");
      title.className = "page-title";
      title.textContent = page.title;
      row.append(dot, title);
      if (page.draft) {
        const draft = document.createElement("span");
        draft.className = "draft-tag";
        draft.textContent = "DRAFT";
        row.append(draft);
      }
      list.append(row);
    }
  }
  $("#page-count").textContent = Object.keys(pages).length;
  $(".nav-item[data-view='All content'] .count").textContent = Object.keys(pages).length;
  $(".nav-item[data-view='Drafts'] .count").textContent = Object.values(pages).filter((page) => page.draft || page.published === false).length;
  $(".navigation").classList.add("connected-project");
  $(".sidebar-foot span:last-child").textContent = "Loaded from Hugo";
  const topLevel = (snapshot.main_menu || []).filter((item) => !item.parent).slice(0, 6);
  $("#site-nav").textContent = topLevel.map((item) => item.label).join("　");
  const footer = $("#page-footer");
  footer.replaceChildren();
  const brand = document.createElement("span");
  brand.textContent = snapshot.summary.name;
  footer.append(brand);
  for (const item of snapshot.footer_menu || []) {
    const link = document.createElement("span");
    link.textContent = item.label;
    footer.append(link);
  }
}

function renderMenuNavigation() {
  const list = $("#page-list");
  list.replaceChildren();
  const appendGroup = (title, items) => {
    const label = document.createElement("div");
    label.className = "content-group-label";
    label.textContent = title;
    list.append(label);
    const topLevel = items.filter((item) => !item.parent);
    for (const item of topLevel) {
      const row = document.createElement("button");
      row.className = "page-row menu-row";
      row.textContent = item.label;
      if (item.url) row.dataset.menuUrl = item.url;
      list.append(row);
      for (const child of items.filter((candidate) => candidate.parent === item.label)) {
        const childRow = document.createElement("button");
        childRow.className = "page-row menu-row menu-child";
        childRow.textContent = "↳ " + child.label;
        if (child.url) childRow.dataset.menuUrl = child.url;
        list.append(childRow);
      }
    }
  };
  appendGroup("Main menu", projectMainMenu);
  appendGroup("Footer menu", projectFooterMenu);
}

function addOrActivateTab(pageId) {
  const page = pages[pageId];
  let tab = $(`.tab[data-page="${pageId}"]`);
  if (!tab) {
    tab = document.createElement("button");
    tab.className = "tab";
    tab.dataset.page = pageId;
    tab.setAttribute("role", "tab");
    tab.innerHTML = `<span class="tab-icon">▤</span><span>${escapeHtml(page.title)}</span><span class="tab-close">×</span>`;
    $("#tabs-bar").insertBefore(tab, $(".add-tab"));
    tab.addEventListener("click", (event) => {
      if (event.target.closest(".tab-close")) {
        if (pageId === "home") return showToast("The home page tab stays open.");
        tab.remove();
        if (selectedPage === pageId) updatePage("home");
      } else updatePage(pageId);
    });
  }
  tab.classList.add("active");
}

function resetTabs(pageId) {
  $$("#tabs-bar .tab[data-page]").forEach((tab) => tab.remove());
  addOrActivateTab(pageId);
}

function renderValidation() {
  const page = pages[selectedPage];
  const isDraft = Boolean(page.draft) || page.published === false;
  const length = page.summary?.length || 0;
  const detail = usingDemoData
    ? "These are illustrative checks. Real results will come from Hugo data and local asset validation."
    : "First pass from Hugo's resolved page index and local Markdown. Hugo remains authoritative for rendered output.";
  $("#inspector-content").innerHTML = `<div class="validation-view"><h3>Checks for ${escapeHtml(page.title)}</h3><div class="validation-row ${length < 120 ? "warn" : "info"}"><span class="validation-icon">${length < 120 ? "!" : "i"}</span><div><strong>${usingDemoData ? "Meta description is short" : "Page summary length"}</strong><span>${length} characters${length < 120 ? ". Consider adding more detail." : "."}</span></div></div><div class="validation-row info"><span class="validation-icon">i</span><div><strong>${page.has_unsupported_blocks ? "Theme content detected" : "Basic content blocks supported"}</strong><span>${page.has_unsupported_blocks ? "Shortcodes, raw HTML, or media are omitted from this native view." : "Headings, paragraphs, quotes, and simple list items are shown."}</span></div></div><div class="validation-row info"><span class="validation-icon">i</span><div><strong>Draft state</strong><span>${isDraft ? "This page is marked as a draft." : "This page is published by Hugo."}</span></div></div><p class="folder-note">${detail}</p></div>`;
}

function setPreview(mode) {
  previewMode = mode;
  $$(".mode-option").forEach((button) => button.classList.toggle("active", button.dataset.mode === mode));
  updatePage(selectedPage);
  if (mode === "web") showToast("Website mode represents Hugo-rendered theme output.");
}

function toggleInspector(forceOpen) {
  const inspector = $("#inspector");
  const hidden = typeof forceOpen === "boolean" ? !forceOpen : !inspector.classList.contains("inspector-hidden");
  inspector.classList.toggle("inspector-hidden", hidden);
  $(".app-frame").classList.toggle("inspector-closed", hidden);
  $(".workspace").classList.toggle("workspace-wide", hidden);
}

function showValidation() {
  inspectorTab = "validation";
  syncInspectorTabSelection();
  renderValidation();
}

function showOverview() {
  inspectorTab = "overview";
  syncInspectorTabSelection();
  $("#inspector-content").innerHTML = overviewMarkup;
  updatePage(selectedPage);
}

async function openRepository() {
  if (window.__TAURI__?.dialog?.open) {
    const selected = await window.__TAURI__.dialog.open({ directory: true, multiple: false, title: "Open Hugo repository" });
    if (selected) inspectSelectedFolder(selected);
    return;
  }
  $("#folder-input").click();
  showToast("Choose a Hugo project folder in the picker.");
}

async function inspectSelectedFolder(path) {
  $("#build-state").textContent = "Checking Hugo project…";
  $(".status-indicator").style.background = "#d3a856";
  try {
    if (window.__TAURI__?.core?.invoke) {
      const snapshot = await window.__TAURI__.core.invoke("load_project", { path });
      const summary = snapshot.summary;
      const manifest = snapshot.manifest;
      const viewPages = (manifest.pages || []).map(manifestPageToView);
      const mainMenu = (manifest.navigation || []).filter((item) => item.menu === "main");
      const footerMenu = (manifest.navigation || []).filter((item) => item.menu === "footer");
      selectedProjectPath = summary.path;
      $("#project-name").textContent = summary.name;
      $("#root-label").textContent = summary.path;
      $("#hugo-version").textContent = summary.hugo_version ?? "Hugo not found";
      $("#page-count").textContent = summary.content_count;
      if (viewPages.length === 0) throw new Error("Hugo returned no pages the MVP can show.");
      usingDemoData = false;
      projectMainMenu = mainMenu;
      projectFooterMenu = footerMenu;
      projectPreviewUrl = snapshot.preview_url;
      projectNavigationSnapshot = {
        summary: { name: manifest.site?.title || summary.name },
        main_menu: mainMenu,
        footer_menu: footerMenu,
        pages: viewPages,
      };
      pages = Object.fromEntries(viewPages.map((page) => [page.id, page]));
      $("#demo-badge").textContent = "LOCAL HUGO SITE";
      $("#site-title").textContent = manifest.site?.title || summary.name;
      renderProjectNavigation(projectNavigationSnapshot);
      const firstPage = viewPages.find((page) => page.kind === "home") || viewPages[0];
      selectedPage = firstPage.id;
      resetTabs(selectedPage);
      updatePage(selectedPage);
      $("#build-state").textContent = viewPages.length + " pages loaded from Hugo";
      showToast(summary.name + ": loaded " + viewPages.length + " pages and " + mainMenu.length + " main-menu links.");
    } else {
      const folderName = path.split(/[\\/]/).filter(Boolean).at(-1) || "Local repository";
      $("#project-name").textContent = folderName;
      $("#root-label").textContent = path;
      $("#build-state").textContent = "Folder selected · connect the native backend to inspect Hugo";
      showToast("The browser preview cannot read selected files. Open this folder in the desktop app to load its Hugo pages.");
    }
  } catch (error) {
    $("#build-state").textContent = "Could not inspect project";
    showToast(String(error));
  } finally {
    $(".status-indicator").style.background = "#45ad80";
  }
}

$("#open-project").addEventListener("click", openRepository);
$("#project-menu").addEventListener("click", openRepository);
$("#folder-input").addEventListener("change", (event) => {
  const file = event.target.files?.[0];
  if (!file) return;
  const folder = file.webkitRelativePath.split("/")[0];
  inspectSelectedFolder(folder);
  event.target.value = "";
});
$("#refresh-button").addEventListener("click", () => {
  if (!usingDemoData && selectedProjectPath) {
    inspectSelectedFolder(selectedProjectPath);
    return;
  }
  $("#build-state").textContent = "Refreshing preview…";
  $(".status-indicator").style.background = "#d3a856";
  setTimeout(() => {
    $("#build-state").textContent = usingDemoData ? "Watching local files" : "Refresh requires a new Hugo load";
    $(".status-indicator").style.background = "#45ad80";
    showToast(usingDemoData ? "Demo preview refreshed." : "Reopen the Hugo repository to reload its content.");
  }, 450);
});
$("#inspector-toggle").addEventListener("click", () => toggleInspector());
$("#close-inspector").addEventListener("click", () => toggleInspector(false));
$(".mode-switch").addEventListener("click", (event) => {
  const button = event.target.closest(".mode-option");
  if (button) setPreview(button.dataset.mode);
});
$(".inspector-tabs").addEventListener("click", (event) => {
  const button = event.target.closest(".inspector-tab");
  if (!button) return;
  if (button.dataset.inspectorTab === "validation") showValidation();
  else showOverview();
});
$("#inspector-content").addEventListener("click", (event) => {
  if (event.target.closest("[data-show-validation]")) showValidation();
});
$("#page-list").addEventListener("click", (event) => {
  const row = event.target.closest(".page-row");
  if (!row) return;
  if (row.dataset.menuUrl) {
    const targetPath = (() => {
      try { return new URL(row.dataset.menuUrl, projectPreviewUrl).pathname; }
      catch { return row.dataset.menuUrl; }
    })();
    const target = Object.entries(pages).find(([, page]) => {
      try { return new URL(page.path).pathname === targetPath; }
      catch { return page.path === targetPath; }
    });
    if (target) updatePage(target[0], { addTab: true });
    else showToast("This menu link is available in Website preview.");
  } else if (row.dataset.page) {
    updatePage(row.dataset.page, { addTab: true });
  }
});
$("#tabs-bar").addEventListener("click", (event) => {
  if (event.target.closest(".add-tab")) showToast("Choose a page from the content list.");
  const tab = event.target.closest(".tab[data-page]");
  if (tab && !tab.querySelector(".tab-close")) updatePage(tab.dataset.page);
});
$(".navigation").addEventListener("click", (event) => {
  const nav = event.target.closest(".nav-item");
  if (!nav) return;
  $$(".nav-item").forEach((item) => item.classList.toggle("selected", item === nav));
  if (nav.dataset.view === "Drafts") {
    if (!usingDemoData) renderProjectNavigation(projectNavigationSnapshot);
    $$(".page-row").forEach((row) => row.hidden = !pages[row.dataset.page] || pages[row.dataset.page].published);
    showToast("Showing draft content.");
  } else if (nav.dataset.view === "All content") {
    if (!usingDemoData) renderProjectNavigation(projectNavigationSnapshot);
    else $$(".page-row").forEach((row) => row.hidden = false);
  } else if (nav.dataset.view === "Menus") {
    if (usingDemoData) showToast("Hugo menu entries appear when a site is connected.");
    else renderMenuNavigation();
  }
});
$("#open-source").addEventListener("click", () => showToast(pages[selectedPage].source_path || pages[selectedPage].path));
$(".share-button").addEventListener("click", () => showToast("Page links are copied from the Hugo manifest in the native app."));
$$('[data-rail="search"]').forEach((button) => button.addEventListener("click", () => showToast("Search will cover page titles, summaries, and paths.")));
$$('[data-rail="builds"]').forEach((button) => button.addEventListener("click", () => showToast("Build history appears here when a local Hugo project is connected.")));
$$('[data-rail="settings"]').forEach((button) => button.addEventListener("click", () => showToast("Settings are planned for project and watcher preferences.")));
$$('[data-rail="home"]').forEach((button) => button.addEventListener("click", () => showToast("Open a local Hugo repository to start reviewing content.")));

updatePage("home");
