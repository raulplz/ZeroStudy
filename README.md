# ZeroStudy

App de estudio hecha en Rust sobre tus propios apuntes en Markdown (una bóveda de Obsidian sirve).
Lee las preguntas de tus notas y te da:

- **Repaso espaciado** (cajas Leitner: 1, 2, 4, 8 y 16 días), simulacros de test cronometrados y técnica Feynman.
- **Elegir tema**: busca en sesiones, conceptos y enunciados, y te sugiere la sesión más floja.
- **Atlas** de conceptos: grafo con zoom y búsqueda, puentes entre asignaturas y las preguntas de cada concepto.
- **Panel** con plazos, racha, mapa de actividad y dominio por asignatura.
- **Metodologías** activables (espaciada, recuperación, intercalado, Feynman, mapas, aprender haciendo, Pomodoro), recomendadas según tu material y tu objetivo.

La misma interfaz corre como **app de escritorio** (Tauri 2) y como **PWA** en el iPhone o cualquier navegador.

> Este repositorio solo contiene código. Tus preguntas, conceptos y apuntes **nunca** van aquí:
> se guardan en tu equipo y, si quieres sincronizar, en un repo privado aparte.

## Cómo está hecho

| Carpeta | Qué es |
|---|---|
| `crates/nucleo` | Modelo, repaso espaciado, atlas, metodologías, configuración y lector de la bóveda (feature `boveda`, solo escritorio). Sin relojes ni disco: compila igual a WebAssembly. |
| `crates/ui` | Interfaz con Leptos 0.8 (CSR) compilada con Trunk. Estilo `estilo.css` derivado del tema ZERØ. |
| `src-tauri` | Backend de escritorio: lee la carpeta, guarda el progreso, escribe la nota de progreso y el widget, sincroniza con GitHub. |

## Compilar

Requisitos: Rust estable con el target `wasm32-unknown-unknown`, `trunk` y `cargo-tauri` 2.x
(y las dependencias de sistema de Tauri para tu distribución).

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk cargo-tauri --locked

cargo test --workspace --features zerostudy-nucleo/boveda   # tests (núcleo y sincronización)
cd crates/ui && trunk build --release                # PWA en crates/ui/dist
cd ../../src-tauri && cargo tauri build --no-bundle  # binario en target/release/zerostudy
```

Pruebas de la PWA en navegador (Chromium y WebKit, escritorio y móvil 390×844, con la API de GitHub simulada):
`cd crates/ui && trunk build && cd ../.. && uv run --with playwright python tests/e2e/run.py all`
(WebKit: `uv run --with playwright python -m playwright install webkit`). Más detalle en [tests/e2e/README.md](tests/e2e/README.md).

Para desarrollar la interfaz sin Tauri: `cd crates/ui && trunk serve` y abre <http://localhost:1420>.

## Primer uso

Al abrirla por primera vez, un asistente de tres pasos pregunta dónde están tus apuntes, qué asignaturas son
de clase y cuáles propias, para qué estudias y qué metodologías activar (vienen preseleccionadas las que
mejor encajan con tu material). Todo se puede cambiar luego en **Ajustes**.

- En escritorio, el asistente sugiere una bóveda de Obsidian que encuentre en `~/Documents`, `~/Obsidian` o bajo una carpeta `NOTAS` de `~/Work`.
  Si existe `~/.config/zerostudy/plantilla.json` (misma forma que `config.json`; ejemplo en [docs/plantilla.ejemplo.json](docs/plantilla.ejemplo.json)),
  rellena el asistente con sus valores. La plantilla vive fuera del repo.
- Cada subcarpeta de la carpeta de apuntes es una asignatura o tema. Una pregunta es un callout `> [!question]-`.
- ZeroStudy solo escribe en tu carpeta la nota `00 - Progreso de repaso.md`.
- Configuración: `~/.config/zerostudy/config.json`. Progreso: `~/.local/share/zerostudy/progreso/<dispositivo>.json`.
- El tema se toma de `~/.config/zerolayer/theme.json` (ZeroLayer) y cambia en vivo.
- `zerostudy --repaso` abre directamente el repaso (o enfoca la ventana si ya está abierta).
- `~/.local/share/zerostudy/widget.json` resume pendientes, racha, dominio y próximo plazo para un widget del escritorio.

## Sincronización e iPhone

El progreso es un **registro de eventos por dispositivo**: cada dispositivo escribe solo su archivo
(`progreso/<dispositivo>.json`), así que no hay conflictos al sincronizar y el estado de cada tarjeta se
reconstruye reproduciendo los eventos de todos.

1. Crea un repo **privado** de datos (por ejemplo `usuario/zerostudy-datos`) y escribe `usuario/repo` en el asistente o en Ajustes.
2. **Escritorio:** usa el token de `gh auth token` (inicia sesión antes con `gh auth login`). Sube `contenido.json` cuando cambian tus apuntes y tu progreso, y baja el de los demás dispositivos.
3. **iPhone (PWA):** sirve `crates/ui/dist` por HTTPS (por ejemplo con GitHub Pages desde este repo, que solo lleva código), ábrelo en Safari y usa *Compartir → Añadir a pantalla de inicio*.
   En el asistente pon el repo y un **token fino** (*fine-grained personal access token*) limitado a ese único repo privado con el permiso
   *Contents: Read and write*. El token se guarda solo en el navegador del móvil. Tras la primera visita la app funciona sin conexión y sube el progreso cuando vuelve la red.

## Licencia

MIT, ver [LICENSE](LICENSE).
