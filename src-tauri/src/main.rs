//! Backend de la app de escritorio de ZeroStudy (Tauri 2).
//!
//! La interfaz (Leptos) es la misma que corre como PWA en el iPhone; aquí vive lo que solo puede
//! hacer el escritorio: leer la carpeta de apuntes, guardar el progreso en disco, escribir la nota
//! de progreso y el widget, y sincronizar con GitHub. Los comandos tienen los nombres y tipos que
//! espera `crates/ui/src/plataforma.rs`.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod descubrir;
mod rutas;
mod sync;

use serde::Serialize;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_opener::OpenerExt;
use zerostudy_nucleo::config::Config;
use zerostudy_nucleo::tema::Tema;
use zerostudy_nucleo::{boveda, nota, Contenido, Estado, Registro};

const NOTA_PROGRESO: &str = "00 - Progreso de repaso.md";

#[derive(Default)]
struct App {
    /// Último contenido leído (para no releer la carpeta en cada guardado).
    contenido: Mutex<Option<Contenido>>,
    /// Evita dos sincronizaciones a la vez.
    sincronizando: Mutex<()>,
    /// Número de guardados; la sincronización en segundo plano espera al último.
    guardados: AtomicU64,
}

type Compartido<'a> = State<'a, Arc<App>>;

#[derive(Serialize)]
struct Carga {
    config: Config,
    contenido: Option<Contenido>,
    registros: Vec<Registro>,
    tema: Option<Tema>,
    aviso: Option<String>,
    sugerencia: String,
    plantilla: Option<Config>,
}

#[derive(Serialize)]
struct Sincronizado {
    registros: Vec<Registro>,
    contenido: Option<Contenido>,
    estado: String,
}

fn home_dir() -> std::path::PathBuf {
    std::env::var_os("HOME").map(Into::into).unwrap_or_default()
}

fn ahora() -> String {
    chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string()
}
fn hoy() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

// ───────────────────────── lectura ─────────────────────────

/// La configuración y si existía el archivo.
fn leer_config() -> (Config, bool, Option<String>) {
    match fs::read_to_string(rutas::config()) {
        Ok(t) => match serde_json::from_str::<Config>(&t) {
            Ok(c) => (c, true, None),
            Err(e) => (Config::default(), true, Some(format!("config.json no es válido ({e}); se usa la configuración por defecto."))),
        },
        Err(_) => (Config::default(), false, None),
    }
}

fn leer_registros() -> Vec<Registro> {
    let mut v: Vec<Registro> = fs::read_dir(rutas::progreso())
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .filter_map(|p| fs::read_to_string(p).ok())
        .filter_map(|t| serde_json::from_str::<Registro>(&t).ok())
        .filter(|r| rutas::dispositivo_valido(&r.dispositivo))
        .collect();
    v.sort_by(|a, b| a.dispositivo.cmp(&b.dispositivo));
    v
}

fn leer_tema() -> Option<Tema> {
    fs::read_to_string(rutas::tema()).ok().and_then(|t| serde_json::from_str(&t).ok())
}

/// Lee los apuntes (y guarda copia) o, si esta carpeta no existe en este equipo, usa el contenido sincronizado.
fn cargar_contenido(cfg: &Config) -> Result<Contenido, String> {
    if Path::new(&cfg.boveda).is_dir() {
        let c = boveda::leer(cfg, &ahora()[..16]).map_err(|e| e.to_string())?;
        if let Ok(bytes) = serde_json::to_vec(&c) {
            let _ = rutas::escribir(&rutas::contenido(), &bytes);
        }
        return Ok(c);
    }
    fs::read_to_string(rutas::contenido())
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .ok_or_else(|| format!("No encuentro la carpeta de apuntes «{}» ni contenido sincronizado todavía.", cfg.boveda))
}

fn contenido_actual(app: &App, cfg: &Config) -> Option<Contenido> {
    if let Some(c) = app.contenido.lock().ok().and_then(|g| g.clone()) {
        return Some(c);
    }
    let c = cargar_contenido(cfg).ok()?;
    if let Ok(mut g) = app.contenido.lock() {
        *g = Some(c.clone());
    }
    Some(c)
}

// ───────────────────────── derivados ─────────────────────────

/// Widget del escritorio y nota de progreso en la bóveda (solo si existe).
fn escribir_derivados(cfg: &Config, c: &Contenido, registros: &[Registro]) {
    let e = Estado::desde(registros);
    let w = e.widget(c, &hoy(), &ahora());
    if let Ok(bytes) = serde_json::to_vec_pretty(&w) {
        let _ = rutas::escribir(&rutas::widget(), &bytes);
    }
    let raiz = Path::new(&cfg.boveda);
    if !cfg.boveda.is_empty() && raiz.is_dir() {
        let _ = rutas::escribir_si_cambia(&raiz.join(NOTA_PROGRESO), nota::progreso(c, &e, &hoy()).as_bytes());
    }
}

// ───────────────────────── comandos ─────────────────────────

#[tauri::command]
async fn cargar(app: Compartido<'_>) -> Result<Carga, String> {
    let app = app.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let (config, existia, mut aviso) = leer_config();
        // En el primer uso: plantilla opcional y, si no, una bóveda de Obsidian encontrada en el equipo.
        let plantilla = if existia { None } else { descubrir::leer_plantilla(&rutas::plantilla()) };
        let sugerencia = if existia || plantilla.is_some() { String::new() } else { descubrir::sugerir_boveda(&home_dir()).unwrap_or_default() };
        let contenido = if config.configurado {
            match cargar_contenido(&config) {
                Ok(c) => Some(c),
                Err(e) => {
                    aviso = Some(e);
                    None
                }
            }
        } else {
            None
        };
        if let Ok(mut g) = app.contenido.lock() {
            *g = contenido.clone();
        }
        let registros = leer_registros();
        if let Some(c) = &contenido {
            escribir_derivados(&config, c, &registros);
        }
        Carga { config, contenido, registros, tema: leer_tema(), aviso, sugerencia, plantilla }
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
async fn guardar_config(app: Compartido<'_>, config: Config) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(&config).map_err(|e| e.to_string())?;
    rutas::escribir(&rutas::config(), &bytes).map_err(|e| format!("No he podido guardar la configuración: {e}"))?;
    // Si cambió la carpeta o las asignaturas, el contenido en memoria ya no vale.
    if let Ok(mut g) = app.contenido.lock() {
        *g = None;
    }
    Ok(())
}

/// Lee los apuntes con una configuración (asistente de primer uso), sin guardarla.
#[tauri::command]
async fn analizar(config: Config) -> Result<Contenido, String> {
    tauri::async_runtime::spawn_blocking(move || {
        if config.boveda.trim().is_empty() {
            return Err("Indica la carpeta de apuntes.".to_string());
        }
        boveda::leer(&config, &ahora()[..16]).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn guardar_registro(handle: AppHandle, app: Compartido<'_>, registro: Registro) -> Result<String, String> {
    if !rutas::dispositivo_valido(&registro.dispositivo) {
        return Err("Nombre de dispositivo no válido (solo letras, números, - y _).".into());
    }
    let bytes = serde_json::to_vec_pretty(&registro).map_err(|e| e.to_string())?;
    rutas::escribir(&rutas::progreso().join(format!("{}.json", registro.dispositivo)), &bytes).map_err(|e| format!("No he podido guardar el progreso: {e}"))?;
    let (cfg, _, _) = leer_config();
    if let Some(c) = contenido_actual(&app, &cfg) {
        escribir_derivados(&cfg, &c, &leer_registros());
    }
    if cfg.configurado && !cfg.sync.repo.is_empty() {
        sincronizar_luego(handle, app.inner().clone());
    }
    Ok("GUARDADO".into())
}

#[tauri::command]
async fn sincronizar(app: Compartido<'_>) -> Result<Sincronizado, String> {
    let app = app.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _g = app.sincronizando.lock().map_err(|_| "sincronización bloqueada".to_string())?;
        hacer_sync(&app).map(|(r, _)| r)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn abrir(handle: AppHandle, url: String) -> Result<(), String> {
    if !["obsidian://", "https://", "http://"].iter().any(|p| url.starts_with(p)) {
        return Err("Solo se abren enlaces obsidian://, http:// o https://.".into());
    }
    handle.opener().open_url(url, None::<&str>).map_err(|e| e.to_string())
}

// ───────────────────────── sincronización ─────────────────────────

/// Sincroniza y devuelve lo que ve la interfaz y si bajó progreso de otros dispositivos.
fn hacer_sync(app: &App) -> Result<(Sincronizado, bool), String> {
    let (cfg, _, _) = leer_config();
    if cfg.sync.repo.is_empty() {
        return Err("No hay repo de sincronización configurado.".into());
    }
    let hay_carpeta = !cfg.boveda.is_empty() && Path::new(&cfg.boveda).is_dir();
    let local = if hay_carpeta { contenido_actual(app, &cfg) } else { None };
    let r = sync::sincronizar(&cfg, local.as_ref())?;
    let contenido = match (&r.contenido, local) {
        (Some(nuevo), _) => {
            if let Ok(mut g) = app.contenido.lock() {
                *g = Some(nuevo.clone());
            }
            Some(nuevo.clone())
        }
        (None, l) => l,
    };
    let registros = leer_registros();
    if let Some(c) = &contenido {
        escribir_derivados(&cfg, c, &registros);
    }
    let estado = format!("SINCRONIZADO {}", &ahora()[11..16]);
    Ok((Sincronizado { registros, contenido: r.contenido, estado }, r.bajados > 0))
}

/// Sincroniza en segundo plano unos segundos después del último guardado.
fn sincronizar_luego(handle: AppHandle, app: Arc<App>) {
    let n = app.guardados.fetch_add(1, Ordering::SeqCst) + 1;
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(8));
        if app.guardados.load(Ordering::SeqCst) != n {
            return; // llegó otro guardado: sincronizará ese
        }
        let Ok(_g) = app.sincronizando.try_lock() else { return };
        if let Ok((s, bajados)) = hacer_sync(&app) {
            if bajados {
                let _ = handle.emit("registros", &s.registros);
            }
        }
    });
}

// ───────────────────────── vigilancia ─────────────────────────

fn modificado(p: &Path) -> Option<SystemTime> {
    fs::metadata(p).and_then(|m| m.modified()).ok()
}

/// Firma de los archivos de progreso de los demás dispositivos (cambia si otro sincroniza).
fn firma_ajenos(propio: &str) -> Vec<(String, Option<SystemTime>)> {
    let mut v: Vec<_> = fs::read_dir(rutas::progreso())
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| (e.file_name().to_string_lossy().into_owned(), modificado(&e.path())))
        .filter(|(n, _)| n.ends_with(".json") && *n != format!("{propio}.json"))
        .collect();
    v.sort();
    v
}

/// Cada 2 s: avisa a la interfaz si cambió el tema de ZeroLayer o el progreso de otro dispositivo.
fn vigilar(handle: AppHandle) {
    thread::spawn(move || {
        let (mut tema, mut ajenos) = (modificado(&rutas::tema()), firma_ajenos(&leer_config().0.dispositivo));
        loop {
            thread::sleep(Duration::from_secs(2));
            let t = modificado(&rutas::tema());
            if t != tema {
                tema = t;
                if let Some(x) = leer_tema() {
                    let _ = handle.emit("tema", &x);
                }
            }
            let f = firma_ajenos(&leer_config().0.dispositivo);
            if f != ajenos {
                ajenos = f;
                let _ = handle.emit("registros", &leer_registros());
            }
        }
    });
}

// ───────────────────────── arranque ─────────────────────────

fn crear_ventana(app: &AppHandle, ruta: &str) -> tauri::Result<()> {
    WebviewWindowBuilder::new(app, "main", WebviewUrl::App(format!("index.html#/{ruta}").into()))
        .title("ZeroStudy")
        .inner_size(1280.0, 860.0)
        .min_inner_size(380.0, 520.0)
        .background_color(tauri::window::Color(8, 11, 16, 255))
        .build()
        .map(|_| ())
}

fn main() {
    let estado = Arc::new(App::default());
    tauri::Builder::default()
        // Una sola instancia: abrir de nuevo (o con `--repaso`) enfoca la ventana existente.
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.unminimize();
                let _ = w.show();
                let _ = w.set_focus();
            }
            if args.iter().any(|a| a == "--repaso") {
                let _ = app.emit("ruta", "repaso");
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .manage(estado)
        .invoke_handler(tauri::generate_handler![cargar, guardar_config, analizar, guardar_registro, sincronizar, abrir])
        .setup(|app| {
            let repaso = std::env::args().any(|a| a == "--repaso");
            crear_ventana(app.handle(), if repaso { "repaso" } else { "panel" })?;
            vigilar(app.handle().clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("no he podido arrancar ZeroStudy");
}
