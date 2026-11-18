const book = document.body.dataset.book;
const stage = document.getElementById('stage');
const frame = document.getElementById('page');
const plate = document.getElementById('plate');
const rail = document.querySelector('#rail i');
const readout = document.getElementById('percent');
const toc = document.getElementById('toc');
const tocButton = document.querySelector('.reader-contents');
const tocClose = document.querySelector('.reader-toc-close');
const scrim = document.querySelector('.reader-scrim');
const wait = document.getElementById('wait');
const retry = document.getElementById('retry');
const offlineButton = document.getElementById('offline');
const offlinePanel = document.getElementById('offline-panel');
const offlineClose = document.querySelector('.reader-offline-close');
const offlineState = document.getElementById('offline-state');
const offlineAction = document.getElementById('offline-action');

const GAP = 40;
const PAD = 24;
const SAVE_DELAY = 1200;
const WAIT_DELAY = 400;
const OFFLINE_KEY = 'kobink:offline';
const QUEUE_KEY = 'kobink:progress';
const AHEAD = navigator.connection && navigator.connection.saveData ? 1 : 4;

let manifest = null;
let index = 0;
let page = 0;
let pages = 1;
let pitch = 0;
let timer = null;
let resizing = null;
let waited = null;
let painted = null;
let saving = null;
let current = { percent: 0, chapter: '', chapter_percent: 0, page: 0, status: 'Reading' };

const clamp = (value, low, high) => Math.min(high, Math.max(low, value));
const pageUrl = (src) => `/read/${book}/res/${encodeURI(src)}?v=${encodeURIComponent(manifest.version)}`;
const records = () => read(OFFLINE_KEY, {});
const record = () => records()[book];

function read(key, fallback) {
  try {
    return JSON.parse(localStorage.getItem(key)) || fallback;
  } catch {
    return fallback;
  }
}

function write(key, value) {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch {}
}

async function start() {
  try {
    const response = await fetch(`/read/${book}/manifest`);
    if (!response.ok) throw new Error(String(response.status));
    manifest = await response.json();
  } catch {
    showRetry('Offline — tap to retry');
    return;
  }

  current = { ...current, ...manifest.progress };
  document.body.classList.toggle('fixed', manifest.layout === 'fixed');
  buildToc();
  setupOffline();
  flush();

  if (manifest.layout === 'fixed') {
    showPlate();
  } else {
    restore();
  }
}

function restore() {
  const saved = manifest.chapters.findIndex((chapter) => chapter.href === current.chapter);
  index = saved >= 0 ? saved : 0;
  openChapter(saved >= 0 ? clamp(current.chapter_percent / 100, 0, 1) : 0);
}

function openChapter(fraction) {
  frame.hidden = false;
  plate.hidden = true;
  retry.hidden = true;
  waiting();
  frame.onload = () => {
    loaded();
    readFrame(fraction);
  };
  frame.src = pageUrl(manifest.chapters[index].href);
}

function readFrame(fraction) {
  const doc = frame.contentDocument;
  if (!doc) return;

  typeset(doc);
  const target = fraction ? Math.round(fraction * (pages - 1)) : 0;
  go(target);
  watchLinks(doc);

  doc.querySelectorAll('img').forEach((image) => {
    if (!image.complete) image.addEventListener('load', reflow, { once: true });
  });

  swipe(doc);

  doc.addEventListener('click', (event) => {
    if (event.target.closest('a') || String(doc.getSelection())) return;
    next();
  });
}

function typeset(doc) {
  const width = Math.max(200, frame.clientWidth || stage.clientWidth);
  const height = Math.max(240, frame.clientHeight || stage.clientHeight);
  const narrow = width < 480;
  const pad = narrow ? 20 : PAD;
  const column = Math.max(160, width - pad * 2);
  const style = doc.getElementById('kobink-typeset') || doc.createElement('style');
  const theme = getComputedStyle(document.body);

  pitch = column + GAP;

  style.id = 'kobink-typeset';
  style.textContent = `
    html {
      height: ${height}px; overflow: hidden;
      color-scheme: light dark; background: ${theme.backgroundColor}; color: ${theme.color};
    }
    body {
      margin: 0; padding: ${pad}px; width: ${width}px; height: ${height}px; box-sizing: border-box;
      column-width: ${column}px; column-gap: ${GAP}px; column-fill: auto;
      font-family: "Iowan Old Style", "Palatino Linotype", Palatino, Georgia, serif;
      font-size: 1.05rem; line-height: 1.62; hyphens: auto;
      text-align: ${narrow ? 'start' : 'justify'}; text-indent: 0;
      background: transparent; color: inherit; overflow: visible;
      transition: transform .18s cubic-bezier(.2,.7,.3,1);
    }
    img, svg, video, table { max-width: 100%; }
    img { max-height: ${height - pad * 2 - 12}px; height: auto; object-fit: contain; }
    p { margin: 0 0 0.9em !important; text-indent: 0 !important; orphans: 2; widows: 2; }
    h1, h2, h3, h4 { line-height: 1.25; text-align: start; break-after: avoid; }
    a { color: inherit; text-decoration-color: color-mix(in srgb, currentColor 45%, transparent); }
    ::selection { background: rgb(231 183 92 / 0.4); }
  `;
  (doc.head || doc.body).append(style);
  pages = Math.max(1, Math.round((doc.body.scrollWidth + GAP) / pitch));
}

function go(target) {
  page = clamp(target, 0, pages - 1);
  const doc = frame.contentDocument;

  if (doc) doc.body.style.transform = `translateX(${-page * pitch}px)`;

  report();
}

function next() {
  if (manifest.layout === 'fixed') {
    if (page < pages - 1) {
      page += 1;
      paint();
    }

    return;
  }

  if (page < pages - 1) return go(page + 1);
  if (index < manifest.chapters.length - 1) {
    index += 1;
    return openChapter(0);
  }
}

function prev() {
  if (manifest.layout === 'fixed') {
    if (page > 0) {
      page -= 1;
      paint();
    }

    return;
  }

  if (page > 0) return go(page - 1);
  if (index > 0) {
    index -= 1;
    return openChapter(1);
  }
}

function showPlate() {
  frame.hidden = true;
  plate.hidden = false;
  pages = Math.max(1, manifest.pages.length);
  page = clamp(current.page || Math.round((current.percent / 100) * (pages - 1)), 0, pages - 1);
  paint();
}

function paint() {
  const item = manifest.pages[page];
  if (!item) return;

  const src = pageUrl(item.src);

  if (painted !== src) {
    painted = src;
    retry.hidden = true;
    plate.hidden = false;
    waiting();
    plate.src = src;
  }

  around(page);
  report();
}

const wanted = new Set();
const queued = [];
let active = 0;

function around(at) {
  for (let step = 1; step <= AHEAD; step += 1) want(at + step);
  want(at - 1);
}

function want(at) {
  const item = manifest.pages[at];
  if (!item) return;

  const src = pageUrl(item.src);
  if (wanted.has(src)) return;

  wanted.add(src);
  queued.push(src);

  if (queued.length > 8) wanted.delete(queued.shift());

  pump();
}

function pump() {
  if (!navigator.onLine) return;

  while (active < 3 && queued.length) {
    load(queued.shift(), 0);
  }
}

function load(src, attempt) {
  active += 1;

  const image = new Image();

  image.onload = () => {
    active -= 1;
    pump();
  };

  image.onerror = () => {
    active -= 1;

    if (attempt < 1) {
      setTimeout(() => {
        queued.unshift(src);
        pump();
      }, 1500);
    } else {
      wanted.delete(src);
    }

    pump();
  };

  image.src = src;
}

function waiting() {
  clearTimeout(waited);

  waited = setTimeout(() => {
    wait.hidden = false;
  }, WAIT_DELAY);
}

function loaded() {
  clearTimeout(waited);
  wait.hidden = true;
}

function showRetry(message) {
  retry.textContent = message;
  retry.hidden = false;
  plate.hidden = true;
  frame.hidden = true;
}

function report() {
  const fixed = manifest.layout === 'fixed';
  const count = manifest.chapters.length || 1;
  const ended = !fixed && index === count - 1 && page === pages - 1;
  const progress = fixed
    ? ((page + 1) / pages) * 100
    : ended
      ? 100
      : ((index + (pages > 1 ? page / pages : 0)) / count) * 100;

  current.percent = Math.round(clamp(progress, 0, 100) * 10) / 10;
  current.chapter = fixed
    ? (manifest.pages[page] || {}).href || current.chapter
    : manifest.chapters[index].href;
  current.chapter_percent = fixed ? 0 : pages > 1 ? (page / pages) * 100 : 0;
  current.page = fixed ? page : 0;

  rail.style.width = `${current.percent}%`;
  readout.textContent = `${Math.round(current.percent)}%`;
  highlight();
  schedule();
}

function schedule() {
  clearTimeout(timer);
  timer = setTimeout(save, SAVE_DELAY);
}

function save() {
  clearTimeout(timer);

  const body = {
    percent: current.percent,
    chapter: current.chapter || '',
    chapter_percent: current.chapter_percent || 0,
    page: current.page || 0,
    status: current.status || 'Reading',
  };

  if (!navigator.onLine) {
    queue(body);
    return;
  }

  fetch(`/read/${book}/progress`, {
    method: 'PUT',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(body),
    keepalive: true,
  })
    .then((response) => {
      if (response.ok || response.status === 404) {
        const pending = read(QUEUE_KEY, {});

        if (pending[book]) {
          delete pending[book];
          write(QUEUE_KEY, pending);
        }

        return;
      }

      queue(body);
    })
    .catch(() => queue(body));
}

function queue(body) {
  const pending = read(QUEUE_KEY, {});
  pending[book] = body;
  write(QUEUE_KEY, pending);
}

function flush() {
  const pending = read(QUEUE_KEY, {});

  Object.keys(pending).forEach((id) => {
    fetch(`/read/${id}/progress`, {
      method: 'PUT',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(pending[id]),
      keepalive: true,
    })
      .then((response) => {
        if (!response.ok && response.status !== 404) return;

        const rest = read(QUEUE_KEY, {});
        delete rest[id];
        write(QUEUE_KEY, rest);
      })
      .catch(() => {});
  });
}

function setupOffline() {
  if (manifest.layout !== 'fixed' || !('serviceWorker' in navigator) || !window.isSecureContext) {
    offlineButton.hidden = true;
    return;
  }

  offlineButton.hidden = false;
  syncOffline();
  paintOffline();
  verifyOffline();
}

async function verifyOffline() {
  const saved = record();
  if (!saved || !saved.complete || !('caches' in window)) return;

  let present = false;

  try {
    present = Boolean(await caches.match(`/read/${book}/manifest`));
  } catch {}

  if (present) return;

  const all = records();
  delete all[book];
  write(OFFLINE_KEY, all);
  paintOffline();
}

function syncOffline() {
  const saved = record();
  if (!saved || saved.version === manifest.version) return;

  const all = records();
  delete all[book];
  write(OFFLINE_KEY, all);

  workerReady().then((worker) => {
    if (worker) worker.postMessage({ type: 'offline-remove', id: book });
  });
}

function paintOffline() {
  const saved = record();

  offlineButton.classList.toggle('saved', Boolean(saved && saved.complete && !saving));

  if (saving) {
    offlineButton.textContent = `${Math.round((saving.done / saving.total) * 100)}%`;
  } else if (saved && saved.complete) {
    offlineButton.textContent = 'Offline';
  } else if (saved) {
    offlineButton.textContent = 'Resume';
  } else {
    offlineButton.textContent = 'Save';
  }

  renderPanel();
}

function renderPanel() {
  const saved = record();

  if (saving) {
    offlineState.textContent = `Saving ${saving.done} of ${saving.total}.`;
    offlineAction.textContent = 'Stop';
    offlineAction.dataset.action = 'stop';
  } else if (saved && saved.complete) {
    offlineState.textContent = `${saved.pages} pages · ${size(saved.bytes)} on this device.`;
    offlineAction.textContent = 'Remove';
    offlineAction.dataset.action = 'remove';
  } else if (saved) {
    offlineState.textContent = `Paused at ${saved.done} of ${saved.total}.`;
    offlineAction.textContent = 'Resume';
    offlineAction.dataset.action = 'save';
  } else {
    offlineState.textContent = `${manifest.pages.length} pages · kept on this device.`;
    offlineAction.textContent = 'Save';
    offlineAction.dataset.action = 'save';
  }
}

function size(value) {
  if (!value) return '0 KB';
  if (value < 1024 * 1024) return `${Math.max(1, Math.round(value / 1024))} KB`;

  return `${Math.round(value / 1024 / 1024)} MB`;
}

async function startOffline() {
  if (saving) return;

  closePanels();

  const assets = [...document.querySelectorAll('link[rel="stylesheet"], script[src]')].map(
    (node) => node.getAttribute('href') || node.getAttribute('src')
  );

  const urls = [
    `/read/${book}`,
    `/read/${book}/manifest`,
    ...assets,
    ...manifest.pages.map((item) => pageUrl(item.src)),
  ];

  saving = { done: 0, total: urls.length, bytes: 0 };
  paintOffline();

  const worker = await workerReady();

  if (!worker) {
    saving = null;
    paintOffline();
    return;
  }

  if (navigator.storage && navigator.storage.persist) navigator.storage.persist().catch(() => {});

  worker.postMessage({ type: 'offline-save', id: book, version: manifest.version, urls });
}

function workerReady() {
  if (!('serviceWorker' in navigator)) return Promise.resolve(null);

  return Promise.race([
    navigator.serviceWorker.ready.then((registration) => registration.active),
    new Promise((resolve) => setTimeout(() => resolve(null), 3000)),
  ]).catch(() => null);
}

function openOffline() {
  closeToc();
  offlinePanel.hidden = false;
  scrim.hidden = false;
  renderPanel();
}

function closeOffline() {
  offlinePanel.hidden = true;
}

function closePanels() {
  closeToc();
  closeOffline();
  scrim.hidden = true;
}

function buildToc() {
  toc.querySelectorAll('h2, ol').forEach((node) => node.remove());

  const title = document.createElement('h2');
  title.textContent = 'Contents';
  const list = document.createElement('ol');

  manifest.chapters.forEach((chapter, at) => {
    const item = document.createElement('li');
    const button = document.createElement('button');
    button.type = 'button';
    button.textContent = chapter.label || `Chapter ${at + 1}`;
    button.dataset.at = at;
    button.addEventListener('click', () => {
      closeToc();
      index = at;

      if (manifest.layout === 'fixed') {
        page = clamp(at, 0, pages - 1);
        paint();
      } else {
        openChapter(0);
      }
    });

    item.append(button);
    list.append(item);
  });

  toc.append(title, list);
  highlight();
}

function highlight() {
  toc.querySelectorAll('button').forEach((button) => {
    button.classList.toggle('current', Number(button.dataset.at) === (manifest.layout === 'fixed' ? page : index));
  });
}

function toggleToc() {
  const open = toc.hidden;

  if (open) closeOffline();

  toc.hidden = !open;
  scrim.hidden = !open;
  tocButton.setAttribute('aria-expanded', String(open));

  if (open) {
    requestAnimationFrame(() => {
      const here = toc.querySelector('button.current');
      if (here) here.scrollIntoView({ block: 'center' });
    });
  }
}

function closeToc() {
  toc.hidden = true;
  scrim.hidden = true;
  tocButton.setAttribute('aria-expanded', 'false');
}

function resolve(base, href) {
  const parts = base.split('/');
  parts.pop();

  for (const segment of href.split('/')) {
    if (segment === '.' || segment === '') continue;
    if (segment === '..') parts.pop();
    else parts.push(segment);
  }

  return parts.join('/');
}

function watchLinks(doc) {
  doc.addEventListener(
    'click',
    (event) => {
      const anchor = event.target.closest ? event.target.closest('a[href]') : null;
      if (!anchor) return;

      const href = anchor.getAttribute('href');
      if (!href) return;

      event.preventDefault();

      if (href.startsWith('#')) {
        const target = doc.getElementById(href.slice(1));
        if (target) target.scrollIntoView();
        return;
      }

      const [path, hash] = href.split('#');
      const resolved = resolve(manifest.chapters[index].href, path);
      const at = manifest.chapters.findIndex((chapter) => chapter.href === resolved);

      if (at >= 0) {
        index = at;
        openChapter(0);
      } else if (hash) {
        const target = doc.getElementById(hash);
        if (target) target.scrollIntoView();
      }
    },
    true
  );
}

matchMedia('(prefers-color-scheme: dark)').addEventListener('change', reflow);

document.querySelector('.reader-next').addEventListener('click', next);
document.querySelector('.reader-prev').addEventListener('click', prev);
tocButton.addEventListener('click', toggleToc);
tocClose.addEventListener('click', closeToc);
offlineClose.addEventListener('click', closePanels);
scrim.addEventListener('click', closePanels);
plate.addEventListener('load', loaded);
plate.addEventListener('error', () => {
  loaded();
  retry.hidden = false;
  plate.hidden = true;
});

retry.addEventListener('click', () => {
  if (manifest && manifest.layout === 'fixed') {
    retry.hidden = true;
    plate.hidden = false;
    painted = null;
    paint();
    return;
  }

  location.reload();
});

offlineButton.addEventListener('click', () => {
  const saved = record();

  if (saving) {
    openOffline();
    return;
  }

  if (!saved || !saved.complete) {
    startOffline();
    return;
  }

  openOffline();
});

offlineAction.addEventListener('click', async () => {
  const action = offlineAction.dataset.action;

  if (action === 'stop') {
    const worker = await workerReady();
    if (worker) worker.postMessage({ type: 'offline-cancel', id: book });
    return;
  }

  if (action === 'remove') {
    const worker = await workerReady();
    if (worker) worker.postMessage({ type: 'offline-remove', id: book });

    const all = records();
    delete all[book];
    write(OFFLINE_KEY, all);
    saving = null;
    paintOffline();
    closePanels();
    return;
  }

  startOffline();
});

if ('serviceWorker' in navigator) {
  navigator.serviceWorker.addEventListener('message', (event) => {
    const data = event.data || {};
    if (data.id !== book) return;

    if (data.type === 'offline-progress') {
      saving = { done: data.done, total: data.total, bytes: data.bytes };
      paintOffline();
    }

    if (data.type === 'offline-done') {
      saving = null;

      const all = records();
      all[book] = {
        version: manifest.version,
        pages: manifest.pages.length,
        done: data.done,
        total: data.total,
        bytes: data.bytes,
        complete: data.complete,
      };
      write(OFFLINE_KEY, all);
      paintOffline();
    }
  });
}

addEventListener('keydown', (event) => {
  if (event.key === 'ArrowRight' || event.key === ' ' || event.key === 'PageDown') {
    event.preventDefault();
    next();
  }

  if (event.key === 'ArrowLeft' || event.key === 'PageUp') {
    event.preventDefault();
    prev();
  }

  if (event.key === 'Escape') closePanels();
});

addEventListener('online', () => {
  flush();
  pump();
});

function swipe(target) {
  let start = null;

  target.addEventListener('touchstart', (event) => {
    start = event.changedTouches[0];
  }, { passive: true });

  target.addEventListener('touchend', (event) => {
    if (!start) return;

    const delta = event.changedTouches[0].clientX - start.clientX;
    start = null;

    if (Math.abs(delta) < 50) return;
    if (delta < 0) next();
    else prev();
  }, { passive: true });
}

swipe(stage);

function reflow() {
  clearTimeout(resizing);

  resizing = setTimeout(() => {
    if (!manifest || manifest.layout === 'fixed' || frame.hidden) return;

    const doc = frame.contentDocument;
    if (!doc) return;

    const fraction = pages > 1 ? page / pages : 0;
    typeset(doc);
    go(Math.round(fraction * (pages - 1)));
  }, 180);
}

addEventListener('resize', reflow);
addEventListener('orientationchange', reflow);

if (window.ResizeObserver) {
  new ResizeObserver(() => reflow()).observe(stage);
}

if (window.visualViewport) {
  window.visualViewport.addEventListener('resize', reflow);
}

addEventListener('pagehide', save);
document.addEventListener('visibilitychange', () => {
  if (document.visibilityState === 'hidden') {
    save();
  } else {
    flush();
  }
});

start();
