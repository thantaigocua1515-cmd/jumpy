// Jumpy web cache — cache-first for the game files so repeat visits load fast.
// Bump CACHE_VERSION whenever jumpy.js / jumpy_bg.wasm change.
const CACHE_VERSION = 'jumpy-cache-v3-mobile1';
const CORE = ['./', './index.html', './jumpy.js', './jumpy_bg.wasm', './manifest.webmanifest'];

self.addEventListener('install', (event) => {
  event.waitUntil(
    caches.open(CACHE_VERSION).then((cache) =>
      Promise.all(CORE.map((url) => cache.add(url).catch(() => {})))
    ).then(() => self.skipWaiting())
  );
});

self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches.keys().then((keys) =>
      Promise.all(keys.filter((k) => k !== CACHE_VERSION).map((k) => caches.delete(k)))
    ).then(() => self.clients.claim())
  );
});

self.addEventListener('fetch', (event) => {
  const req = event.request;
  if (req.method !== 'GET' || new URL(req.url).origin !== self.location.origin) return;
  event.respondWith(
    caches.match(req, { ignoreSearch: true }).then((hit) => {
      if (hit) return hit;
      return fetch(req).then((resp) => {
        if (resp && resp.ok) {
          const copy = resp.clone();
          caches.open(CACHE_VERSION).then((cache) => cache.put(req, copy)).catch(() => {});
        }
        return resp;
      });
    })
  );
});
