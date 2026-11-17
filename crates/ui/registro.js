// Registra el service worker solo en la versión web (no dentro de Tauri).
if ("serviceWorker" in navigator && !window.__TAURI__ && /^https?:$/.test(location.protocol)) {
  navigator.serviceWorker.register("./sw.js").catch(() => {});
}
