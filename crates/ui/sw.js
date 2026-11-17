// Service worker básico de ZeroStudy: la app (HTML, wasm, JS, CSS) funciona sin red tras la primera visita.
// Las peticiones a otros orígenes (la API de GitHub) nunca pasan por la caché.
const CACHE = "zerostudy-app-v1";

self.addEventListener("install", (e) => {
  e.waitUntil(caches.open(CACHE).then((c) => c.addAll(["./", "./manifest.webmanifest"])).then(() => self.skipWaiting()));
});

self.addEventListener("activate", (e) => {
  e.waitUntil(
    caches.keys().then((ks) => Promise.all(ks.filter((k) => k !== CACHE).map((k) => caches.delete(k)))).then(() => self.clients.claim())
  );
});

self.addEventListener("fetch", (e) => {
  const req = e.request;
  const url = new URL(req.url);
  if (req.method !== "GET" || url.origin !== self.location.origin) return;
  // El HTML se pide primero a la red (para coger versiones nuevas) y cae a la caché sin conexión.
  if (req.mode === "navigate") {
    e.respondWith(
      fetch(req)
        .then((r) => { const copia = r.clone(); caches.open(CACHE).then((c) => c.put("./", copia)); return r; })
        .catch(() => caches.match("./"))
    );
    return;
  }
  // Los archivos de la app llevan hash en el nombre: caché primero.
  e.respondWith(
    caches.match(req).then((hit) => hit || fetch(req).then((r) => {
      if (r.ok) { const copia = r.clone(); caches.open(CACHE).then((c) => c.put(req, copia)); }
      return r;
    }))
  );
});
