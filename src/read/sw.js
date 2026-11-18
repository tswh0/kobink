const SHELL = "kobink-shell-v2";
const BOOK = "kobink-book-";
const PRECACHE = [
  "/offline",
  "/icon-192.png",
  "/icon-512.png",
  "/icon-maskable-512.png",
  "/apple-touch-icon.png",
  "/favicon.ico",
];
const STATIC = [
  "/style.css",
  "/page.css",
  "/reader.css",
  "/reader.js",
  "/pwa.js",
  "/shelf.js",
  "/manifest.webmanifest",
  "/icon-",
  "/favicon",
  "/apple-touch-icon",
];

let names = null;
const stopping = new Set();

self.addEventListener("install", (event) => {
  event.waitUntil(
    caches
      .open(SHELL)
      .then((cache) => cache.addAll(PRECACHE))
      .then(() => self.skipWaiting())
  );
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((keys) =>
        Promise.all(
          keys
            .filter((key) => key.startsWith("kobink") && key !== SHELL && !key.startsWith(BOOK))
            .map((key) => caches.delete(key))
        )
      )
      .then(() => self.clients.claim())
  );
});

self.addEventListener("fetch", (event) => {
  const request = event.request;

  if (request.method !== "GET") return;

  const url = new URL(request.url);

  if (url.origin !== self.location.origin) return;

  if (request.mode === "navigate") {
    event.respondWith(navigate(request, url));
    return;
  }

  if (url.pathname.startsWith("/read/") && url.pathname.includes("/res/")) {
    event.respondWith(resource(request, url));
    return;
  }

  if (url.pathname.startsWith("/read/") && url.pathname.endsWith("/manifest")) {
    event.respondWith(manifest(request, url));
    return;
  }

  if (url.pathname.endsWith("/false/image.jpg")) {
    event.respondWith(picture(request));
    return;
  }

  if (!STATIC.some((prefix) => url.pathname.startsWith(prefix))) return;

  event.respondWith(stale(request));
});

self.addEventListener("message", (event) => {
  const data = event.data || {};

  if (data.type === "offline-save") event.waitUntil(save(event, data));
  if (data.type === "offline-cancel") stopping.add(data.id);
  if (data.type === "offline-remove") event.waitUntil(remove(data.id));
});

async function navigate(request, url) {
  if (url.pathname === "/") return shelf(request);
  if (/^\/read\/[^/]+$/.test(url.pathname)) return reader(request, url);

  try {
    return await fetch(request);
  } catch {
    return (await caches.match("/offline")) || Response.error();
  }
}

async function reader(request, url) {
  try {
    return await fetch(request);
  } catch {
    const cache = await bookCache(bookId(url.pathname));
    const hit = cache && (await cache.match(request, { ignoreSearch: true }));

    return hit || (await caches.match("/offline")) || Response.error();
  }
}

async function resource(request, url) {
  const cache = await bookCache(bookId(url.pathname));

  if (cache) {
    const hit = await cache.match(request);
    if (hit) return hit;
  }

  return fetch(request);
}

async function manifest(request, url) {
  try {
    return await fetch(request);
  } catch {
    const cache = await bookCache(bookId(url.pathname));
    const hit = cache && (await cache.match(request));

    return hit || Response.error();
  }
}

async function picture(request) {
  const cache = await caches.open(SHELL);
  const hit = await cache.match(request);

  if (hit) return hit;

  const response = await fetch(request);
  if (response.ok && !response.redirected) cache.put(request, response.clone());

  return response;
}

async function shelf(request) {
  const cache = await caches.open(SHELL);
  const cached = await cache.match("/");

  const network = fetch(request)
    .then((response) => {
      if (response.ok && !response.redirected) {
        const check = response.clone();
        const stored = response.clone();

        check
          .text()
          .then((html) => {
            if (html.includes('class="shelf"')) return cache.put("/", stored);
          })
          .catch(() => {});
      }

      return response;
    })
    .catch(() => null);

  if (!cached) return (await network) || (await caches.match("/offline")) || Response.error();

  const fresh = await Promise.race([
    network,
    new Promise((resolve) => setTimeout(() => resolve(null), 4000)),
  ]);

  return fresh || cached;
}

async function stale(request) {
  const cache = await caches.open(SHELL);
  const hit = (await cache.match(request)) || (await caches.match(request));

  const network = fetch(request)
    .then((response) => {
      if (response.ok && !response.redirected) cache.put(request, response.clone());
      return response;
    })
    .catch(() => null);

  if (hit) return hit;

  return (await network) || Response.error();
}

async function bookCache(id) {
  if (!names) {
    names = new Map();

    for (const key of await caches.keys()) {
      const match = /^kobink-book-([0-9a-f-]{36})-(.+)$/.exec(key);
      if (match) names.set(match[1], key);
    }
  }

  const name = names.get(id);

  return name ? caches.open(name) : null;
}

async function save(event, data) {
  const id = data.id;
  const name = `${BOOK}${id}-${data.version}`;
  const list = [...new Set((data.urls || []).map((url) => new URL(url, self.location.origin).href))];

  stopping.delete(id);
  names = null;

  const old = (await caches.keys()).filter(
    (key) => key.startsWith(`${BOOK}${id}-`) && key !== name
  );

  await Promise.all(old.map((key) => caches.delete(key)));

  const cache = await caches.open(name);
  names = null;

  let done = 0;
  let bytes = 0;
  let next = 0;
  let failed = false;

  const fill = async () => {
    while (next < list.length && !stopping.has(id) && !failed) {
      const url = list[next];
      next += 1;

      const hit = await cache.match(url);

      if (hit) {
        done += 1;
        bytes += Number(hit.headers.get("content-length")) || 0;
        post(event.source, { type: "offline-progress", id, done, total: list.length, bytes });
        continue;
      }

      try {
        const response = await fetch(url);
        if (!response.ok || response.redirected) throw new Error(String(response.status));

        await cache.put(url, response.clone());
        done += 1;
        bytes += Number(response.headers.get("content-length")) || 0;
        post(event.source, { type: "offline-progress", id, done, total: list.length, bytes });
      } catch {
        failed = true;
      }
    }
  };

  await Promise.all([fill(), fill(), fill()]);

  const complete = done === list.length;

  post(event.source, { type: "offline-done", id, done, total: list.length, bytes, complete });
  stopping.delete(id);
}

async function remove(id) {
  names = null;

  const keys = (await caches.keys()).filter((key) => key.startsWith(`${BOOK}${id}-`));

  await Promise.all(keys.map((key) => caches.delete(key)));
}

function bookId(path) {
  return path.split("/")[2];
}

function post(client, message) {
  try {
    if (client) client.postMessage(message);
  } catch {}
}
