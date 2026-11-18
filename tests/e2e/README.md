# Pruebas e2e de la PWA

`run.py` sirve `crates/ui/dist` (compílalo antes con `trunk build`), simula la API de contenidos de GitHub con
Playwright y recorre: asistente web, repaso con Feynman, simulacro con nota final, elegir tema, atlas, panel,
asignaturas y ajustes, en escritorio (1280×860) y móvil (390×844), comprobando además que no hay scroll horizontal
ni errores de consola. `fixture.py` genera un contenido sintético (sin datos reales).

```bash
uv run --with playwright python tests/e2e/run.py chromium   # usa /usr/bin/chromium si existe
uv run --with playwright python tests/e2e/run.py webkit     # el más parecido a Safari del iPhone
uv run --with playwright python tests/e2e/run.py all
```

Las capturas quedan en `tests/e2e/capturas/` (ignorada por Git). Salida distinta de 0 si algo falla.

WebKit: `uv run --with playwright python -m playwright install webkit`. En distribuciones que no son Ubuntu puede
faltar `libicu74`/`libxml2` (soname de Ubuntu); sin sudo basta extraer esos `.deb` y copiar las `.so` a
`~/.cache/ms-playwright/webkit-*/minibrowser-wpe/sys/lib/`, y ejecutar con `PLAYWRIGHT_SKIP_VALIDATE_HOST_REQUIREMENTS=1`.
