//! Dónde se guardan y de dónde salen los datos.
//!
//! - **Escritorio (Tauri):** el backend en Rust lee la bóveda, guarda el progreso en disco,
//!   escribe la nota de progreso y el widget, y sincroniza con GitHub.
//! - **Web / iPhone (PWA):** todo va contra el repo privado de datos con la API de GitHub y un
//!   token de acceso limitado a ese repo; se guarda una copia en el navegador para usarla sin red.

use base64::Engine;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use zerostudy_nucleo::config::Config;
use zerostudy_nucleo::tema::Tema;
use zerostudy_nucleo::{Contenido, Registro};

use crate::util::{local_guardar, local_leer};

#[wasm_bindgen(inline_js = r#"
export function es_tauri() { return !!(window.__TAURI__ && window.__TAURI__.core); }
export function tauri_invocar(cmd, args) { return window.__TAURI__.core.invoke(cmd, args); }
export function tauri_escuchar(ev, cb) { window.__TAURI__.event.listen(ev, (e) => cb(e.payload)); }
"#)]
extern "C" {
    fn es_tauri() -> bool;
    #[wasm_bindgen(catch)]
    fn tauri_invocar(cmd: &str, args: JsValue) -> Result<js_sys::Promise, JsValue>;
    fn tauri_escuchar(ev: &str, cb: &Closure<dyn Fn(JsValue)>);
}

pub fn escritorio() -> bool {
    es_tauri()
}

fn texto_error(e: JsValue) -> String {
    e.as_string().unwrap_or_else(|| format!("{e:?}"))
}

async fn invocar<T: DeserializeOwned>(cmd: &str, args: impl Serialize) -> Result<T, String> {
    let args = serde_wasm_bindgen::to_value(&args).map_err(|e| e.to_string())?;
    let p = tauri_invocar(cmd, args).map_err(texto_error)?;
    let v = JsFuture::from(p).await.map_err(texto_error)?;
    serde_wasm_bindgen::from_value(v).map_err(|e| e.to_string())
}

/// Llama a `cb` cada vez que el backend emite `evento` (solo escritorio).
pub fn escuchar<T: DeserializeOwned + 'static>(evento: &str, cb: impl Fn(T) + 'static) {
    if !escritorio() {
        return;
    }
    let c = Closure::<dyn Fn(JsValue)>::new(move |v: JsValue| {
        if let Ok(x) = serde_wasm_bindgen::from_value::<T>(v) {
            cb(x);
        }
    });
    tauri_escuchar(evento, &c);
    c.forget();
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Carga {
    pub config: Config,
    pub contenido: Option<Contenido>,
    pub registros: Vec<Registro>,
    pub tema: Option<Tema>,
    pub aviso: Option<String>,
    /// Sugerencia de carpeta de apuntes para el primer uso (escritorio).
    #[serde(default)]
    pub sugerencia: String,
    /// Plantilla opcional (`~/.config/zerostudy/plantilla.json`) para rellenar el asistente.
    #[serde(default)]
    pub plantilla: Option<Config>,
}

#[derive(Serialize)]
struct Nada {}

#[derive(Serialize)]
struct ConConfig<'a> {
    config: &'a Config,
}

#[derive(Serialize)]
struct ConRegistro<'a> {
    registro: &'a Registro,
}

#[derive(Serialize)]
struct ConUrl<'a> {
    url: &'a str,
}

// ───────────────────────── API común ─────────────────────────

pub async fn cargar() -> Result<Carga, String> {
    if escritorio() {
        return invocar("cargar", Nada {}).await;
    }
    let config: Config = local_leer("zerostudy-config").and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
    let contenido = local_leer("zerostudy-contenido").and_then(|t| serde_json::from_str(&t).ok());
    let registros = local_leer("zerostudy-registros").and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
    Ok(Carga { config, contenido, registros, ..Default::default() })
}

/// Copia inmediata del progreso en el navegador (la subida a GitHub va con retraso).
pub fn guardar_registros_local(todos: &[Registro]) {
    if !escritorio() {
        if let Ok(t) = serde_json::to_string(todos) {
            local_guardar("zerostudy-registros", &t);
        }
    }
}

pub async fn guardar_config(c: &Config) -> Result<(), String> {
    if escritorio() {
        return invocar("guardar_config", ConConfig { config: c }).await;
    }
    local_guardar("zerostudy-config", &serde_json::to_string(c).map_err(|e| e.to_string())?);
    Ok(())
}

/// Lee el contenido con una configuración (asistente de primer uso).
pub async fn analizar(c: &Config) -> Result<Contenido, String> {
    if escritorio() {
        return invocar("analizar", ConConfig { config: c }).await;
    }
    let gh = Github::desde(c)?;
    let texto = gh.leer("contenido.json").await?.ok_or("El repo no tiene contenido.json todavía: sincroniza primero desde el ordenador.")?;
    let contenido: Contenido = serde_json::from_str(&texto).map_err(|e| e.to_string())?;
    local_guardar("zerostudy-contenido", &texto);
    Ok(contenido)
}

/// Guarda el registro de este dispositivo. Devuelve un texto de estado para la barra lateral.
pub async fn guardar_registro(c: &Config, r: &Registro, todos: &[Registro]) -> Result<String, String> {
    if escritorio() {
        return invocar("guardar_registro", ConRegistro { registro: r }).await;
    }
    local_guardar("zerostudy-registros", &serde_json::to_string(todos).map_err(|e| e.to_string())?);
    match Github::desde(c) {
        Ok(gh) => match gh.escribir(&format!("progreso/{}.json", c.dispositivo), &serde_json::to_string_pretty(r).unwrap()).await {
            Ok(()) => Ok("SUBIDO A GITHUB".into()),
            Err(_) => Ok("GUARDADO EN EL MÓVIL · SE SUBIRÁ CON RED".into()),
        },
        Err(_) => Ok("GUARDADO EN EL NAVEGADOR".into()),
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Sincronizado {
    pub registros: Vec<Registro>,
    pub contenido: Option<Contenido>,
    pub estado: String,
}

/// Trae el progreso de los demás dispositivos y sube el propio.
pub async fn sincronizar(c: &Config, propio: Option<&Registro>) -> Result<Sincronizado, String> {
    if escritorio() {
        return invocar("sincronizar", Nada {}).await;
    }
    let gh = Github::desde(c)?;
    let contenido = match gh.leer("contenido.json").await? {
        Some(t) => {
            local_guardar("zerostudy-contenido", &t);
            serde_json::from_str(&t).ok()
        }
        None => None,
    };
    let mut registros: Vec<Registro> = Vec::new();
    for nombre in gh.listar("progreso").await? {
        if let Some(t) = gh.leer(&format!("progreso/{nombre}")).await? {
            if let Ok(r) = serde_json::from_str::<Registro>(&t) {
                if r.dispositivo != c.dispositivo {
                    registros.push(r);
                }
            }
        }
    }
    if let Some(p) = propio {
        gh.escribir(&format!("progreso/{}.json", c.dispositivo), &serde_json::to_string_pretty(p).unwrap()).await?;
        registros.push(p.clone());
    }
    local_guardar("zerostudy-registros", &serde_json::to_string(&registros).unwrap());
    Ok(Sincronizado { registros, contenido, estado: format!("SINCRONIZADO {}", &crate::util::ahora()[11..16]) })
}

pub fn abrir(url: &str) {
    if escritorio() {
        let url = url.to_string();
        wasm_bindgen_futures::spawn_local(async move {
            let _: Result<(), String> = invocar("abrir", ConUrl { url: &url }).await;
        });
    } else if let Some(w) = web_sys::window() {
        let _ = w.location().set_href(url);
    }
}

// ───────────────────────── GitHub (solo web) ─────────────────────────

pub fn token() -> String {
    local_leer("zerostudy-token").unwrap_or_default()
}
pub fn guardar_token(t: &str) {
    local_guardar("zerostudy-token", t.trim());
}

struct Github {
    repo: String,
    rama: String,
    token: String,
}

#[derive(Deserialize)]
struct Entrada {
    name: String,
    #[serde(default)]
    sha: String,
}

impl Github {
    fn desde(c: &Config) -> Result<Self, String> {
        let token = token();
        if c.sync.repo.is_empty() || token.is_empty() {
            return Err("Falta el repo o el token de GitHub (Ajustes).".into());
        }
        Ok(Self { repo: c.sync.repo.clone(), rama: if c.sync.rama.is_empty() { "main".into() } else { c.sync.rama.clone() }, token })
    }

    fn url(&self, ruta: &str) -> String {
        format!("https://api.github.com/repos/{}/contents/{}?ref={}", self.repo, ruta, self.rama)
    }

    async fn pedir(&self, metodo: &str, url: &str, accept: &str, cuerpo: Option<String>) -> Result<(u16, String), String> {
        let h = web_sys::Headers::new().map_err(texto_error)?;
        h.set("Authorization", &format!("Bearer {}", self.token)).map_err(texto_error)?;
        h.set("Accept", accept).map_err(texto_error)?;
        h.set("X-GitHub-Api-Version", "2022-11-28").map_err(texto_error)?;
        let init = web_sys::RequestInit::new();
        init.set_method(metodo);
        init.set_headers(&h);
        init.set_mode(web_sys::RequestMode::Cors);
        if let Some(c) = cuerpo {
            init.set_body(&JsValue::from_str(&c));
        }
        let req = web_sys::Request::new_with_str_and_init(url, &init).map_err(texto_error)?;
        let w = web_sys::window().ok_or("sin ventana")?;
        let resp: web_sys::Response = JsFuture::from(w.fetch_with_request(&req)).await.map_err(|_| "Sin conexión con GitHub".to_string())?.into();
        let texto = JsFuture::from(resp.text().map_err(texto_error)?).await.map_err(texto_error)?.as_string().unwrap_or_default();
        Ok((resp.status(), texto))
    }

    async fn leer(&self, ruta: &str) -> Result<Option<String>, String> {
        let (s, t) = self.pedir("GET", &self.url(ruta), "application/vnd.github.raw", None).await?;
        match s {
            200 => Ok(Some(t)),
            404 => Ok(None),
            401 | 403 => Err("GitHub rechaza el token (¿caducado o sin permiso de contenido en ese repo?)".into()),
            _ => Err(format!("GitHub respondió {s}")),
        }
    }

    async fn listar(&self, dir: &str) -> Result<Vec<String>, String> {
        let (s, t) = self.pedir("GET", &self.url(dir), "application/vnd.github+json", None).await?;
        if s == 404 {
            return Ok(Vec::new());
        }
        let v: Vec<Entrada> = serde_json::from_str(&t).map_err(|_| format!("GitHub respondió {s}"))?;
        Ok(v.into_iter().map(|e| e.name).filter(|n| n.ends_with(".json")).collect())
    }

    async fn sha(&self, ruta: &str) -> Result<Option<String>, String> {
        let (s, t) = self.pedir("GET", &self.url(ruta), "application/vnd.github+json", None).await?;
        if s == 404 {
            return Ok(None);
        }
        Ok(serde_json::from_str::<Entrada>(&t).ok().map(|e| e.sha))
    }

    async fn escribir(&self, ruta: &str, texto: &str) -> Result<(), String> {
        for _ in 0..2 {
            let sha = self.sha(ruta).await?;
            let mut cuerpo = serde_json::json!({
                "message": format!("progreso: {ruta}"),
                "content": base64::engine::general_purpose::STANDARD.encode(texto.as_bytes()),
                "branch": self.rama,
            });
            if let Some(s) = sha {
                cuerpo["sha"] = s.into();
            }
            let url = format!("https://api.github.com/repos/{}/contents/{}", self.repo, ruta);
            let (s, _) = self.pedir("PUT", &url, "application/vnd.github+json", Some(cuerpo.to_string())).await?;
            match s {
                200 | 201 => return Ok(()),
                409 | 422 => continue, // otro dispositivo lo cambió entre medias: reintenta con el sha nuevo
                otro => return Err(format!("GitHub respondió {otro} al subir")),
            }
        }
        Err("No se pudo subir tras reintentar".into())
    }
}
