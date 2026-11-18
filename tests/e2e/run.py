"""E2E de la PWA de ZeroStudy con Playwright y la API de GitHub simulada.

Uso (desde la raíz del repo, con `crates/ui/dist` ya compilado con `trunk build`):
    uv run --with playwright python tests/e2e/run.py [chromium|webkit|all] [--dist RUTA]
Chromium usa /usr/bin/chromium si existe; WebKit usa el de Playwright
(`uv run --with playwright python -m playwright install webkit`).
Capturas en tests/e2e/capturas/ (ignorada por Git). Código de salida distinto de 0 si algo falla.
"""
import base64
import functools
import hashlib
import http.server
import json
import os
import re
import sys
import threading
from pathlib import Path

from playwright.sync_api import sync_playwright

AQUI = Path(__file__).resolve().parent
sys.path.insert(0, str(AQUI))
import fixture  # noqa: E402

RAIZ = AQUI.parent.parent
CAPTURAS = AQUI / "capturas"
FALLOS: list[str] = []


def comprobar(cond, msg):
    print(("  ok   " if cond else "  FALLA ") + msg, flush=True)
    if not cond:
        FALLOS.append(msg)


def servir(dist: Path):
    class H(http.server.SimpleHTTPRequestHandler):
        def log_message(self, *a):
            pass

    srv = http.server.ThreadingHTTPServer(("127.0.0.1", 0), functools.partial(H, directory=str(dist)))
    threading.Thread(target=srv.serve_forever, daemon=True).start()
    return srv, f"http://127.0.0.1:{srv.server_address[1]}/"


class GitHub:
    """API de contenidos de GitHub en memoria (la que usa la PWA)."""

    def __init__(self):
        self.archivos = {"contenido.json": json.dumps(fixture.contenido(), ensure_ascii=False)}
        self.puts = []

    def manejar(self, route):
        req = route.request
        cors = {"access-control-allow-origin": "*", "access-control-allow-headers": "*", "access-control-allow-methods": "GET,PUT,OPTIONS"}
        if req.method == "OPTIONS":
            return route.fulfill(status=204, headers=cors)
        m = re.search(r"/contents/([^?]*)", req.url)
        ruta = m.group(1) if m else ""
        if not req.headers.get("authorization", "").startswith("Bearer "):
            return route.fulfill(status=401, headers=cors, body="{}")
        accept = req.headers.get("accept", "")
        sha = lambda t: hashlib.sha1(t.encode()).hexdigest()
        if req.method == "GET":
            if ruta in self.archivos:
                if "raw" in accept:
                    return route.fulfill(status=200, headers=cors, content_type="text/plain", body=self.archivos[ruta])
                return route.fulfill(status=200, headers=cors, content_type="application/json", body=json.dumps({"name": ruta.split("/")[-1], "sha": sha(self.archivos[ruta])}))
            hijos = [k for k in self.archivos if k.startswith(ruta + "/")]
            if hijos:
                return route.fulfill(status=200, headers=cors, content_type="application/json", body=json.dumps([{"name": k.split("/")[-1], "sha": sha(self.archivos[k])} for k in hijos]))
            return route.fulfill(status=404, headers=cors, body="{}")
        if req.method == "PUT":
            cuerpo = json.loads(req.post_data)
            existe = ruta in self.archivos
            if existe and cuerpo.get("sha") != sha(self.archivos[ruta]):
                return route.fulfill(status=422, headers=cors, body="{}")
            self.archivos[ruta] = base64.b64decode(cuerpo["content"]).decode()
            self.puts.append(ruta)
            return route.fulfill(status=200 if existe else 201, headers=cors, body="{}")
        route.fulfill(status=405, headers=cors, body="{}")


def captura(page, nombre):
    CAPTURAS.mkdir(exist_ok=True)
    page.screenshot(path=str(CAPTURAS / f"{nombre}.png"))


def sin_desborde(page, donde):
    d = page.evaluate("() => document.documentElement.scrollWidth - window.innerWidth")
    comprobar(d <= 1, f"{donde}: sin scroll horizontal (desborde {d}px)")


def asistente(page, base, gh, etiqueta):
    page.goto(base)
    page.wait_for_selector("text=Dónde están tus apuntes", timeout=30000)
    captura(page, f"{etiqueta}-01-asistente")
    page.fill('input[placeholder="usuario/repo"]', "usuario/zerostudy-datos")
    page.fill('input[type="password"]', "github_pat_falso")
    page.click("text=Analizar apuntes")
    page.wait_for_selector("text=Lo que hemos encontrado", timeout=15000)
    comprobar("2 asignaturas" in page.inner_text("body") or page.locator(".resumen-analisis b").first.inner_text() == "2", "asistente: analiza el contenido del repo simulado")
    captura(page, f"{etiqueta}-02-asistente-analisis")
    page.click("text=Ver metodologías")
    page.wait_for_selector("text=Cómo quieres estudiar")
    # activa todas las metodologías para probar Feynman, intercalado y mapas
    for _ in range(10):  # la lista se vuelve a pintar tras cada clic
        apagados = page.locator('button.conmutador[aria-checked="false"]')
        if not apagados.count():
            break
        apagados.first.click()
        page.wait_for_timeout(100)
    captura(page, f"{etiqueta}-03-asistente-metodos")
    page.click("text=Empezar ▶")
    page.wait_for_selector(".lateral, nav", timeout=15000)
    page.wait_for_selector("nav a.activa:has-text('Panel')", timeout=15000)
    comprobar(True, "asistente: termina y muestra el panel")
    cfg = json.loads(page.evaluate("() => localStorage.getItem('zerostudy-config')"))
    comprobar(cfg["configurado"] and cfg["sync"]["repo"] == "usuario/zerostudy-datos", "asistente: guarda la configuración")


def ir(page, ruta):
    page.evaluate("r => { location.hash = r }", f"#/{ruta}")
    page.wait_for_timeout(300)


def avanzar_tarjeta(page):
    """Responde la tarjeta actual de un repaso normal."""
    if page.locator(".opcion").count():
        page.locator(".opcion:not([disabled])").first.click()
        page.wait_for_selector(".opcion[disabled]")
        sig = page.locator("button:has-text('Siguiente')")
        (sig if sig.count() else page.locator("button.b")).first.click()
        return
    if page.locator("textarea.explicar").count():
        page.fill("textarea.explicar", "Lo explico con mis palabras.")
        page.click("button:has-text('Comparar')")
    else:
        page.click("button:has-text('Mostrar respuesta')")
    page.click("button.b")


def repaso(page, etiqueta):
    ir(page, "repaso")
    page.wait_for_selector("text=Empezar sesión")
    captura(page, f"{etiqueta}-10-repaso-config")
    page.click(".chip:has-text('Todo aleatorio')")
    page.click(".chip:has-text('20')")
    page.click("text=Empezar sesión")
    vistas_feynman = False
    capturada = False
    for _ in range(80):
        page.wait_for_selector(".tarjeta, .resumen-final")
        if page.locator(".resumen-final").count():
            break
        if page.locator("textarea.explicar").count():
            if not vistas_feynman:
                captura(page, f"{etiqueta}-12-feynman")
            vistas_feynman = True
        elif not capturada:
            captura(page, f"{etiqueta}-11-repaso-tarjeta")
            capturada = True
        avanzar_tarjeta(page)
    page.wait_for_selector(".resumen-final", timeout=15000)
    txt = page.inner_text(".resumen-final")
    comprobar("SESIÓN COMPLETADA" in txt, "repaso: llega a «Sesión completada»")
    comprobar(vistas_feynman, "feynman: las preguntas abiertas piden explicación antes de comparar")
    captura(page, f"{etiqueta}-13-repaso-fin")
    sin_desborde(page, "repaso final")
    registros = json.loads(page.evaluate("() => localStorage.getItem('zerostudy-registros')"))
    comprobar(sum(len(r["eventos"]) for r in registros) >= 18, "repaso: guarda los eventos en el navegador al instante")
    # «Repaso» del menú en la pantalla final: la ruta no cambia y debe volver igualmente al selector.
    page.locator("a[href='#/repaso']").first.click()
    page.wait_for_selector("text=Empezar sesión", timeout=5000)
    comprobar(page.locator(".resumen-final").count() == 0, "repaso: «Repaso» del menú en la pantalla final vuelve al selector")


def simulacro(page, gh, etiqueta):
    if page.locator("button:has-text('Otra sesión')").count():
        page.click("button:has-text('Otra sesión')")
    page.wait_for_selector("text=Empezar sesión")
    page.click(".chip:has-text('Simulacro')")
    page.click(".chip:has-text('40')")
    page.click("text=Empezar sesión")
    page.wait_for_selector("text=SIMULACRO · PREGUNTA 1")
    captura(page, f"{etiqueta}-20-simulacro")
    n = 0
    while page.locator("text=SIN CORRECCIÓN HASTA EL FINAL").count():
        page.locator(".opcion").first.click()
        n += 1
        page.wait_for_timeout(120)
        if n > 60:
            break
    page.wait_for_selector("text=SIMULACRO TERMINADO", timeout=10000)
    comprobar(n == 12, f"simulacro: solo preguntas tipo test ({n} de 12)")
    txt = page.inner_text(".resumen-final")
    m = re.search(r"\n(\d+),(\d)\n(\d+) de (\d+) correctas", txt)
    ok = m and abs(float(f"{m[1]}.{m[2]}") - 10 * int(m[3]) / int(m[4])) < 0.06
    comprobar(bool(ok), f"simulacro: la nota final es coherente con los aciertos ({txt[:70]!r})")
    captura(page, f"{etiqueta}-21-simulacro-fin")
    sin_desborde(page, "simulacro final")
    page.wait_for_timeout(5500)  # la subida va 4 s después del último cambio
    reg = json.loads(gh.archivos.get("progreso/movil.json", "{}"))
    comprobar(len(reg.get("simulacros", [])) == 1 and reg["simulacros"][0]["n"] == 12, "simulacro: se guarda y se sube al repo simulado")


def vistas(page, etiqueta):
    ir(page, "temas")
    page.wait_for_selector("text=SUGERENCIA", timeout=8000)
    page.fill("input[placeholder*='uscar'], input.buscar", "trait")
    page.wait_for_timeout(300)
    comprobar("Traits" in page.inner_text("main"), "elegir tema: la búsqueda encuentra la sesión de Traits")
    captura(page, f"{etiqueta}-30-temas")
    sin_desborde(page, "elegir tema")
    page.fill("input[placeholder*='uscar'], input.buscar", "")
    page.locator("button:has-text('Estudiar'), button:has-text('▶')").first.click()
    page.wait_for_selector(".tarjeta")
    comprobar(True, "elegir tema: «Estudiar» abre una sesión")
    page.keyboard.press("Escape")

    ir(page, "atlas")
    page.wait_for_selector("circle.nodo", timeout=8000)
    comprobar(page.locator("circle.nodo").count() == 8, f"atlas: 8 nodos ({page.locator('circle.nodo').count()})")
    captura(page, f"{etiqueta}-40-atlas")
    page.locator("circle.nodo").first.click(force=True)
    page.wait_for_timeout(300)
    comprobar(page.locator(".ficha-concepto.abierta").count() == 1, "atlas: pulsar un nodo abre su ficha")
    captura(page, f"{etiqueta}-41-atlas-ficha")
    page.fill("input.buscar", "tcp")
    page.keyboard.press("Enter")
    page.wait_for_timeout(300)
    comprobar("TCP" in page.inner_text(".ficha-concepto"), "atlas: buscar y Enter abre el concepto")
    sin_desborde(page, "atlas")
    for v in ("panel", "unidades", "ajustes"):
        ir(page, v)
        page.wait_for_timeout(300)
        captura(page, f"{etiqueta}-5-{v}")
        sin_desborde(page, v)


def recarga(page, base):
    page.goto(base)
    page.wait_for_selector("nav a.activa:has-text('Panel')", timeout=15000)
    comprobar(True, "recarga: la app arranca configurada desde el almacenamiento local")


def correr(p, nombre, base, movil):
    etiqueta = f"{nombre}-{'movil' if movil else 'escritorio'}"
    print(f"== {etiqueta}", flush=True)
    if nombre == "chromium":
        exe = "/usr/bin/chromium" if os.path.exists("/usr/bin/chromium") else None
        nav = p.chromium.launch(executable_path=exe, args=["--no-sandbox"])
    else:
        nav = p.webkit.launch()
    kw = {"viewport": {"width": 390, "height": 844}, "device_scale_factor": 2, "is_mobile": True, "has_touch": True} if movil else {"viewport": {"width": 1280, "height": 860}}
    ctx = nav.new_context(service_workers="block", **kw)
    gh = GitHub()
    ctx.route("https://api.github.com/**", gh.manejar)
    page = ctx.new_page()
    errores = []
    page.on("pageerror", lambda e: errores.append(str(e)))
    page.on("console", lambda m: errores.append(m.text) if m.type == "error" and "404" not in m.text else None)
    page.on("response", lambda r: errores.append(f"{r.status} {r.url}") if r.status == 404 and r.url.startswith(base) else None)
    try:
        asistente(page, base, gh, etiqueta)
        sin_desborde(page, "panel")
        repaso(page, etiqueta)
        simulacro(page, gh, etiqueta)
        vistas(page, etiqueta)
        recarga(page, base)
    except Exception as e:  # noqa: BLE001
        captura(page, f"{etiqueta}-error")
        comprobar(False, f"excepción: {str(e).splitlines()[0]}")
    comprobar(not errores, f"sin errores de consola/página {errores[:3]}")
    nav.close()


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    dist = Path(sys.argv[sys.argv.index("--dist") + 1]) if "--dist" in sys.argv else RAIZ / "crates/ui/dist"
    if "--dist" in sys.argv:
        args = [a for a in args if a != str(dist)]
    navegadores = ["chromium", "webkit"] if (not args or args[0] == "all") else [args[0]]
    srv, base = servir(dist)
    with sync_playwright() as p:
        for n in navegadores:
            for movil in (False, True):
                correr(p, n, base, movil)
    srv.shutdown()
    print(f"\n{len(FALLOS)} fallo(s)")
    sys.exit(1 if FALLOS else 0)


if __name__ == "__main__":
    main()
