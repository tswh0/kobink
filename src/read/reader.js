const book = document.body.dataset.book;
const stage = document.getElementById('stage');
const frame = document.getElementById('page');
const plate = document.getElementById('plate');
const rail = document.querySelector('#rail i');
const readout = document.getElementById('percent');
const toc = document.getElementById('toc');
const tocButton = document.querySelector('.reader-contents');

const GAP = 40;
const PAD = 24;
const SAVE_DELAY = 1200;

let manifest = null;
let index = 0;
let page = 0;
let pages = 1;
let measured = { width: 0, height: 0, pitch: 0 };
let timer = null;
let resizing = null;
let current = { percent: 0, chapter: '', chapter_percent: 0, page: 0, status: 'Reading' };

const clamp = (value, low, high) => Math.min(high, Math.max(low, value));

async function start() {
  manifest = await fetch(`/read/${book}/manifest`).then((response) => response.json());
  current = { ...current, ...manifest.progress };
  document.body.classList.toggle('fixed', manifest.layout === 'fixed');
  buildToc();

  if (manifest.layout === 'fixed') {
    showPlate();
  } else {
    restore();
  }
}

function margins() {
  return (frame.clientWidth || stage.clientWidth) < 480 ? 16 : PAD;
}

function restore() {
  const saved = manifest.chapters.findIndex((chapter) => chapter.href === current.chapter);
  index = saved >= 0 ? saved : 0;
  openChapter(saved >= 0 ? clamp(current.chapter_percent / 100, 0, 1) : 0);
}

function openChapter(fraction) {
  frame.hidden = false;
  plate.hidden = true;
  frame.onload = () => readFrame(fraction);
  frame.src = `/read/${book}/res/${encodeURI(manifest.chapters[index].href)}`;
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
}

function typeset(doc) {
  const pad = margins();
  const width = Math.max(200, frame.clientWidth || stage.clientWidth);
  const height = Math.max(240, frame.clientHeight || stage.clientHeight);
  const column = Math.max(160, width - pad * 2);
  const style = doc.getElementById('kobink-typeset') || doc.createElement('style');

  measured = { width, height, pitch: column + GAP };

  style.id = 'kobink-typeset';
  style.textContent = `
    html { height: ${height}px; overflow: hidden; }
    body {
      margin: 0; padding: ${pad}px; width: ${width}px; height: ${height}px; box-sizing: border-box;
      column-width: ${column}px; column-gap: ${GAP}px; column-fill: auto;
      font-family: "Iowan Old Style", "Palatino Linotype", Palatino, Georgia, serif;
      font-size: 1.05rem; line-height: 1.62; text-align: justify; hyphens: auto;
      background: transparent; color: inherit; overflow: visible;
      transition: transform .18s cubic-bezier(.2,.7,.3,1);
    }
    img, svg, video, table { max-width: 100%; }
    img { max-height: ${height - pad * 2 - 12}px; height: auto; object-fit: contain; }
    p { margin: 0 0 1em; orphans: 2; widows: 2; }
    h1, h2, h3, h4 { line-height: 1.25; text-align: start; break-after: avoid; }
    a { color: inherit; text-decoration-color: color-mix(in srgb, currentColor 45%, transparent); }
    ::selection { background: rgb(231 183 92 / 0.4); }
  `;
  doc.head.append(style);
  pages = Math.max(1, Math.round((doc.body.scrollWidth + GAP) / measured.pitch));
}

function go(target) {
  page = clamp(target, 0, pages - 1);
  const doc = frame.contentDocument;

  if (doc) doc.body.style.transform = `translateX(${-page * measured.pitch}px)`;

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
  const saved = current.page || 0;
  const fromPercent = Math.round((current.percent / 100) * (pages - 1));
  page = clamp(saved || fromPercent, 0, pages - 1);
  paint();
}

function paint() {
  const item = manifest.pages[page];
  if (!item) return;

  plate.src = `/read/${book}/res/${encodeURI(item.src)}`;
  warm(page + 1);
  warm(page - 1);
  report();
}

function warm(at) {
  const item = manifest.pages[at];
  if (item) new Image().src = `/read/${book}/res/${encodeURI(item.src)}`;
}

function report() {
  const fixed = manifest.layout === 'fixed';
  const count = manifest.chapters.length || 1;
  const progress = fixed
    ? ((page + 1) / pages) * 100
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

  return fetch(`/read/${book}/progress`, {
    method: 'PUT',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(body),
    keepalive: true,
  }).catch(() => {});
}

function buildToc() {
  toc.replaceChildren();

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
  toc.hidden = !open;
  tocButton.setAttribute('aria-expanded', String(open));
}

function closeToc() {
  toc.hidden = true;
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

document.querySelector('.reader-next').addEventListener('click', next);
document.querySelector('.reader-prev').addEventListener('click', prev);
tocButton.addEventListener('click', toggleToc);

addEventListener('keydown', (event) => {
  if (event.key === 'ArrowRight' || event.key === ' ' || event.key === 'PageDown') {
    event.preventDefault();
    next();
  }

  if (event.key === 'ArrowLeft' || event.key === 'PageUp') {
    event.preventDefault();
    prev();
  }

  if (event.key === 'Escape') closeToc();
});

let touch = null;

stage.addEventListener('touchstart', (event) => {
  touch = event.changedTouches[0];
}, { passive: true });

stage.addEventListener('touchend', (event) => {
  if (!touch) return;

  const delta = event.changedTouches[0].clientX - touch.clientX;
  touch = null;

  if (Math.abs(delta) < 50) return;
  if (delta < 0) next();
  else prev();
}, { passive: true });

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
  if (document.visibilityState === 'hidden') save();
});

start();
