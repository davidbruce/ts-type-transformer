import { getDrawerTabs, langFromSource, renderCode } from '/code_viewer.mjs';

const STYLE = `
  :root {
    color-scheme: light;
    --bg: #ffffff;
    --surface: #f7f7f7;
    --surface-2: #efefef;
    --border: #d4d4d4;
    --border-strong: #111111;
    --text: #111111;
    --muted: #555555;
    --inv-bg: #111111;
    --inv-text: #ffffff;
  }
  * {
    box-sizing: border-box;
  }
  html,
  body {
    margin: 0;
    padding: 0;
    width: 100%;
    height: 100%;
    background: var(--bg);
    color: var(--text);
    font-family: system-ui, -apple-system, sans-serif;
    overflow: hidden;
  }
  #layout {
    display: grid;
    grid-template-columns: 280px 1fr;
    width: 100%;
    height: 100%;
  }
  #sidebar {
    background: var(--surface);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  .side-head {
    padding: 14px 16px 12px;
    border-bottom: 1px solid var(--border);
  }
  .side-title-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin-bottom: 10px;
  }
  #target-title {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    letter-spacing: 0.01em;
  }
  #count {
    font-size: 11px;
    font-family: ui-monospace, SFMono-Regular, monospace;
    color: #333333;
    background: var(--bg);
    border: 1px solid #c4c4c4;
    border-radius: 3px;
    padding: 2px 8px;
    white-space: nowrap;
  }
  #filter {
    width: 100%;
    padding: 7px 10px;
    background: var(--bg);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 3px;
    font-size: 13px;
    outline: none;
  }
  #filter:focus {
    border-color: var(--border-strong);
  }
  #nav {
    flex: 1;
    overflow-y: auto;
    padding: 10px 10px 18px;
  }
  .pkg-group {
    margin-bottom: 14px;
  }
  .pkg-heading {
    margin: 0 0 6px;
    padding: 0 8px;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--muted);
  }
  .demo-link {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 7px 10px;
    margin: 2px 0;
    border-radius: 3px;
    color: var(--text);
    text-decoration: none;
    font-size: 13px;
    border: 1px solid transparent;
  }
  .demo-link:hover {
    background: var(--surface-2);
    border-color: var(--border);
  }
  .demo-link.active {
    background: var(--inv-bg);
    border-color: var(--border-strong);
    color: var(--inv-text);
  }
  .demo-title {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .demo-ind {
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-family: ui-monospace, SFMono-Regular, monospace;
    font-size: 10px;
  }
  .demo-kind {
    border: 1px solid #c4c4c4;
    border-radius: 2px;
    padding: 0 4px;
    color: var(--muted);
  }
  .demo-link.active .demo-kind {
    border-color: #777777;
    color: #e5e5e5;
  }
  .demo-dot {
    font-size: 10px;
    line-height: 1;
  }
  .demo-dot.is-ready {
    color: #111111;
  }
  .demo-dot.is-pending {
    color: #999999;
  }
  .demo-link.active .demo-dot.is-ready {
    color: #ffffff;
  }
  .demo-link.active .demo-dot.is-pending {
    color: #a3a3a3;
  }
  .side-foot {
    padding: 8px 12px;
    border-top: 1px solid var(--border);
    font-family: ui-monospace, SFMono-Regular, monospace;
    font-size: 11px;
    color: var(--muted);
    background: var(--bg);
    line-height: 1.45;
  }
  .side-foot code {
    color: var(--text);
    font-weight: 600;
  }
  #stage {
    position: relative;
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
  }
  #bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 10px 16px;
    border-bottom: 1px solid var(--border);
    background: var(--surface);
  }
  .bar-meta {
    display: flex;
    align-items: baseline;
    gap: 10px;
    min-width: 0;
  }
  .pill {
    font-size: 11px;
    font-family: ui-monospace, SFMono-Regular, monospace;
    color: #333333;
    background: var(--bg);
    border: 1px solid #c4c4c4;
    border-radius: 3px;
    padding: 2px 6px;
    white-space: nowrap;
  }
  .pill.is-ready {
    border-color: var(--border-strong);
    color: var(--text);
  }
  .pill.is-pending {
    color: var(--muted);
  }
  #active-title {
    font-size: 14px;
    font-weight: 600;
    white-space: nowrap;
  }
  #active-desc {
    font-size: 12px;
    color: var(--muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .bar-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }
  .btn {
    appearance: none;
    background: var(--bg);
    color: var(--text);
    border: 1px solid #c4c4c4;
    border-radius: 3px;
    padding: 5px 10px;
    font-size: 12px;
    text-decoration: none;
    cursor: pointer;
  }
  .btn:hover {
    border-color: var(--border-strong);
    background: var(--surface-2);
  }
  #viewport-wrap {
    position: relative;
    flex: 1;
    min-height: 0;
  }
  #viewer {
    width: 100%;
    height: 100%;
    border: 0;
    display: block;
    background: var(--bg);
  }
  #node-card {
    padding: 24px;
    height: 100%;
    overflow: auto;
    font-family: ui-monospace, SFMono-Regular, monospace;
    font-size: 13px;
    background: var(--bg);
  }
  #node-card pre {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 3px;
    padding: 14px;
    overflow-x: auto;
  }
  #node-card pre.cv-code {
    background: var(--bg);
    padding: 8px 0;
  }
  #source-drawer {
    position: absolute;
    top: 0;
    right: 0;
    bottom: 0;
    width: min(680px, 60%);
    background: var(--surface);
    border-left: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    z-index: 20;
  }
  #loading {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(255, 255, 255, 0.88);
    z-index: 10;
    pointer-events: none;
  }
  #loading-box {
    padding: 10px 14px;
    background: var(--bg);
    color: var(--text);
    border: 1px solid var(--border-strong);
    border-radius: 3px;
    font-family: ui-monospace, SFMono-Regular, monospace;
    font-size: 12px;
  }
  #loading[hidden],
  #source-drawer[hidden],
  #node-card[hidden],
  #viewer[hidden],
  #active-status[hidden] {
    display: none;
  }
  .drawer-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 10px 14px;
    border-bottom: 1px solid var(--border);
    font-family: ui-monospace, SFMono-Regular, monospace;
    font-size: 12px;
  }
  .drawer-actions {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
  }
  .drawer-tabs {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 14px;
    border-bottom: 1px solid var(--border);
    background: var(--surface);
  }
  .drawer-tab {
    appearance: none;
    background: var(--bg);
    color: var(--muted);
    border: 1px solid #c4c4c4;
    border-radius: 3px;
    padding: 3px 9px;
    font-family: ui-monospace, SFMono-Regular, monospace;
    font-size: 11px;
    cursor: pointer;
  }
  .drawer-tab:hover {
    color: var(--text);
    border-color: var(--border-strong);
  }
  .drawer-tab.active {
    background: var(--inv-bg);
    color: var(--inv-text);
    border-color: var(--border-strong);
  }
  #source-code {
    margin: 0;
    padding: 10px 0;
    flex: 1;
    overflow: auto;
    font-family: ui-monospace, SFMono-Regular, monospace;
    font-size: 12px;
    line-height: 1.55;
    background: var(--bg);
    color: var(--text);
  }
`;

const styleEl = document.createElement('style');
styleEl.textContent = STYLE;
document.head.appendChild(styleEl);

document.body.innerHTML = `
  <div id="layout">
    <aside id="sidebar">
      <div class="side-head">
        <div class="side-title-row">
          <h1 id="target-title">tsrs examples</h1>
          <span id="count">0</span>
        </div>
        <input id="filter" type="search" placeholder="Filter package or demo..." />
      </div>
      <nav id="nav"></nav>
      <div class="side-foot">
        Pre-build: <code id="prebuild-cmd">npm run precompile</code>
      </div>
    </aside>
    <main id="stage">
      <header id="bar">
        <div class="bar-meta">
          <span id="active-pkg" class="pill"></span>
          <span id="active-title">Loading...</span>
          <span id="active-desc"></span>
        </div>
        <div class="bar-actions">
          <span id="active-status" class="pill" hidden></span>
          <button id="view-source-btn" class="btn" type="button">View Source</button>
          <a id="open-direct" class="btn" href="#" target="_blank" rel="noopener">Open Direct</a>
        </div>
      </header>
      <div id="viewport-wrap">
        <iframe id="viewer" title="Demo Viewer"></iframe>
        <div id="loading" hidden>
          <div id="loading-box">Loading demo...</div>
        </div>
        <div id="node-card" hidden>
          <p id="node-desc"></p>
          <pre id="node-cmd"></pre>
          <pre id="node-source"></pre>
        </div>
        <aside id="source-drawer" hidden>
          <div class="drawer-head">
            <span id="source-path"></span>
            <div class="drawer-actions">
              <button id="copy-source" class="btn" type="button">Copy</button>
              <button id="close-source" class="btn" type="button">Close</button>
            </div>
          </div>
          <div id="source-tabs" class="drawer-tabs" role="tablist"></div>
          <pre id="source-code"></pre>
        </aside>
      </div>
    </main>
  </div>
`;

const manifest = await (await fetch('./examples.json')).json();
const targetTitle = document.getElementById('target-title');
const countEl = document.getElementById('count');
const filterInput = document.getElementById('filter');
const nav = document.getElementById('nav');
const prebuildCmd = document.getElementById('prebuild-cmd');
const activePkg = document.getElementById('active-pkg');
const activeTitle = document.getElementById('active-title');
const activeDesc = document.getElementById('active-desc');
const activeStatus = document.getElementById('active-status');
const openDirect = document.getElementById('open-direct');
const viewer = document.getElementById('viewer');
const loading = document.getElementById('loading');
const loadingBox = document.getElementById('loading-box');
const nodeCard = document.getElementById('node-card');
const nodeDesc = document.getElementById('node-desc');
const nodeCmd = document.getElementById('node-cmd');
const nodeSource = document.getElementById('node-source');
const sourceDrawer = document.getElementById('source-drawer');
const sourcePath = document.getElementById('source-path');
const sourceTabs = document.getElementById('source-tabs');
const sourceCode = document.getElementById('source-code');
const viewSourceBtn = document.getElementById('view-source-btn');
const copySourceBtn = document.getElementById('copy-source');
const closeSourceBtn = document.getElementById('close-source');

document.title = `${manifest.label} Examples — tsrs_proto`;
targetTitle.textContent = manifest.label;
prebuildCmd.textContent = `node build.mjs`;

const canPollStatus = Boolean(
  import.meta?.hot || document.querySelector('script[src*="@vite/client"]'),
);

let statusSupported = false;
let targetStatus = null;

async function refreshStatus() {
  if (!canPollStatus) return;
  try {
    const res = await fetch(`/__api/status?target=${encodeURIComponent(manifest.target)}`, {
      cache: 'no-store',
    });
    if (!res.ok) return;
    const data = await res.json();
    if (data?.target) {
      statusSupported = true;
      targetStatus = data.target;
    }
  } catch {
    statusSupported = false;
  }
}

function updateCount() {
  countEl.textContent =
    statusSupported && targetStatus
      ? `${targetStatus.compiled}/${targetStatus.total} built`
      : `${manifest.examples.length} examples`;
}

function updateNavItemStatus(itemId, isReady) {
  const link = [...nav.querySelectorAll('.demo-link')].find(el => el.dataset.id === itemId);
  const dot = link?.querySelector('.demo-dot');
  if (!dot) return;
  dot.className = 'demo-dot ' + (isReady ? 'is-ready' : 'is-pending');
  dot.textContent = isReady ? '●' : '○';
  dot.title = isReady ? 'Pre-compiled' : 'Not compiled yet';
}

function updateActiveStatusPill(item) {
  if (!statusSupported || !targetStatus || !item) {
    activeStatus.hidden = true;
    return;
  }
  const isReady = Boolean(targetStatus.items?.[item.id]);
  activeStatus.hidden = false;
  activeStatus.textContent = isReady ? 'pre-compiled' : 'on-demand';
  activeStatus.className = 'pill ' + (isReady ? 'is-ready' : 'is-pending');
}

let current = null;
let activeSeq = 0;
let drawerSeq = 0;
let activeLoadCleanup = null;

function renderNav(query) {
  const q = query.trim().toLowerCase();
  const groups = new Map();
  for (const item of manifest.examples) {
    if (
      q &&
      !item.id.toLowerCase().includes(q) &&
      !item.title.toLowerCase().includes(q) &&
      !item.package.toLowerCase().includes(q) &&
      !(item.description ?? '').toLowerCase().includes(q)
    ) {
      continue;
    }
    if (!groups.has(item.package)) groups.set(item.package, []);
    groups.get(item.package).push(item);
  }
  nav.innerHTML = '';
  for (const [pkg, items] of groups) {
    const section = document.createElement('section');
    section.className = 'pkg-group';
    const h = document.createElement('h2');
    h.className = 'pkg-heading';
    h.textContent = pkg;
    section.appendChild(h);
    for (const item of items) {
      const a = document.createElement('a');
      a.className = 'demo-link' + (current?.id === item.id ? ' active' : '');
      a.href = `#${item.id}`;
      a.dataset.id = item.id;

      const titleSpan = document.createElement('span');
      titleSpan.className = 'demo-title';
      titleSpan.textContent = item.title;
      a.appendChild(titleSpan);

      const indSpan = document.createElement('span');
      indSpan.className = 'demo-ind';
      if (item.kind === 'node') {
        const kindBadge = document.createElement('span');
        kindBadge.className = 'demo-kind';
        kindBadge.textContent = 'cli';
        indSpan.appendChild(kindBadge);
      }
      if (statusSupported && targetStatus) {
        const dot = document.createElement('span');
        const isReady = Boolean(targetStatus.items?.[item.id]);
        dot.className = 'demo-dot ' + (isReady ? 'is-ready' : 'is-pending');
        dot.textContent = isReady ? '●' : '○';
        dot.title = isReady ? 'Pre-compiled' : 'Not compiled yet';
        indSpan.appendChild(dot);
      }
      if (indSpan.childNodes.length > 0) {
        a.appendChild(indSpan);
      }

      section.appendChild(a);
    }
    nav.appendChild(section);
  }
}

async function activate(id) {
  const item = manifest.examples.find(e => e.id === id) ?? manifest.examples[0];
  if (!item) return;
  const seq = ++activeSeq;
  if (activeLoadCleanup) {
    activeLoadCleanup();
    activeLoadCleanup = null;
  }

  current = item;
  sourceDrawer.hidden = true;
  activePkg.textContent = item.package;
  activeTitle.textContent = item.title;
  activeDesc.textContent = item.description ?? '';
  updateActiveStatusPill(item);
  openDirect.href = new URL(item.kind === 'node' ? item.source : item.path, location.href).href;

  for (const el of nav.querySelectorAll('.demo-link')) {
    el.classList.toggle('active', el.dataset.id === item.id);
  }
  updateCount();

  const isCompiled = statusSupported && targetStatus ? Boolean(targetStatus.items?.[item.id]) : null;

  if (item.kind === 'node') {
    loading.hidden = true;
    viewer.hidden = true;
    viewer.removeAttribute('src');
    nodeCard.hidden = false;
    nodeDesc.textContent = `${item.title} — Node benchmark / CLI example`;
    const baseCmd = item.command ?? `node ./${item.package}/...`;
    nodeCmd.textContent =
      isCompiled === false
        ? `# Pre-compile first:\nnode build.mjs ${item.id}\n\n# Run:\n${baseCmd}`
        : baseCmd;
    const srcText = await (await fetch(item.source)).text();
    if (seq !== activeSeq) return;
    renderCode(nodeSource, srcText, langFromSource(manifest.target, item.source));
    window.__galleryReady = {
      target: manifest.target,
      label: manifest.label,
      count: manifest.examples.length,
      activeId: item.id,
    };
    return;
  }

  nodeCard.hidden = true;
  viewer.hidden = false;
  loadingBox.textContent =
    isCompiled === true
      ? `Loading pre-compiled ${manifest.label} · ${item.id}...`
      : isCompiled === false
        ? `Compiling ${manifest.label} · ${item.id} on demand...`
        : `Loading ${manifest.label} · ${item.id}...`;
  loading.hidden = false;

  await new Promise(resolve => {
    const cleanup = () => {
      viewer.removeEventListener('load', onLoad);
    };
    const onLoad = () => {
      cleanup();
      activeLoadCleanup = null;
      resolve();
    };
    activeLoadCleanup = () => {
      cleanup();
      resolve();
    };
    viewer.addEventListener('load', onLoad);
    viewer.src = new URL(item.path, location.href).href;
  });

  if (seq !== activeSeq) return;
  loading.hidden = true;

  if (statusSupported) {
    await refreshStatus();
    if (seq !== activeSeq) return;
    updateNavItemStatus(item.id, Boolean(targetStatus?.items?.[item.id]));
    updateActiveStatusPill(item);
    updateCount();
  }

  window.__galleryReady = {
    target: manifest.target,
    label: manifest.label,
    count: manifest.examples.length,
    activeId: item.id,
  };
}

async function loadDrawerTab(tabs, tab) {
  const seq = ++drawerSeq;
  for (const btn of sourceTabs.querySelectorAll('.drawer-tab')) {
    const isActive = btn.dataset.tabId === tab.id;
    btn.classList.toggle('active', isActive);
    btn.setAttribute('aria-selected', String(isActive));
  }
  sourcePath.textContent = `./${tab.relPath}`;
  sourceCode.classList.remove('cv-code');
  sourceCode.textContent = 'Loading...';
  delete sourceCode.dataset.rawSource;
  const srcText = await (await fetch(`./${tab.relPath}`)).text();
  if (seq !== drawerSeq) return;
  renderCode(sourceCode, srcText, langFromSource(manifest.target, tab.relPath));
}

viewSourceBtn.addEventListener('click', async () => {
  if (!current) return;
  const tabs = getDrawerTabs(manifest.target, current);
  sourceTabs.innerHTML = '';
  for (const tab of tabs) {
    const btn = document.createElement('button');
    btn.type = 'button';
    btn.role = 'tab';
    btn.className = 'drawer-tab';
    btn.dataset.tabId = tab.id;
    btn.textContent = tab.label;
    btn.addEventListener('click', () => loadDrawerTab(tabs, tab));
    sourceTabs.appendChild(btn);
  }
  sourceDrawer.hidden = false;
  await loadDrawerTab(tabs, tabs[0]);
});

copySourceBtn.addEventListener('click', async () => {
  const text = sourceCode.dataset.rawSource ?? sourceCode.textContent ?? '';
  if (!text) return;
  let copied = false;
  if (navigator.clipboard?.writeText) {
    try {
      await navigator.clipboard.writeText(text);
      copied = true;
    } catch {
      copied = false;
    }
  }
  if (!copied) {
    const ta = document.createElement('textarea');
    ta.value = text;
    ta.style.position = 'fixed';
    ta.style.opacity = '0';
    document.body.appendChild(ta);
    ta.select();
    try {
      copied = document.execCommand('copy');
    } catch {
      copied = false;
    }
    ta.remove();
  }
  if (copied) {
    copySourceBtn.textContent = 'Copied';
    setTimeout(() => {
      if (copySourceBtn.textContent === 'Copied') copySourceBtn.textContent = 'Copy';
    }, 1200);
  }
});

closeSourceBtn.addEventListener('click', () => {
  sourceDrawer.hidden = true;
});

filterInput.addEventListener('input', () => renderNav(filterInput.value));
window.addEventListener('hashchange', () => {
  activate(location.hash.slice(1));
});

await refreshStatus();
updateCount();
renderNav('');
await activate(location.hash.slice(1));
