//! Sincronización con el repo privado de datos (API de contenidos de GitHub).
//!
//! Sube `contenido.json` si cambió y el progreso propio, y baja el de los demás dispositivos.
//! Cada dispositivo escribe solo su archivo, así que nunca hay conflictos.

use base64::Engine;
use serde::Deserialize;
use std::process::Command;
use std::time::Duration;
use zerostudy_nucleo::config::Config;
use zerostudy_nucleo::{Contenido, Registro};

use crate::rutas;

pub struct Gh {
    repo: String,
    rama: String,
    token: String,
    cliente: reqwest::blocking::Client,
}

#[derive(Deserialize)]
struct Entrada {
    name: String,
    #[serde(default)]
    sha: String,
}

/// Token de GitHub: el de `gh` (ya iniciado sesión) o `GH_TOKEN` / `GITHUB_TOKEN`.
fn token() -> Option<String> {
    let de_gh = Command::new("gh").args(["auth", "token"]).output().ok().filter(|o| o.status.success()).map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());
    de_gh
        .filter(|t| !t.is_empty())
        .or_else(|| ["GH_TOKEN", "GITHUB_TOKEN"].iter().find_map(|v| std::env::var(v).ok()).filter(|t| !t.is_empty()))
}

impl Gh {
    pub fn nuevo(cfg: &Config) -> Result<Self, String> {
        if cfg.sync.repo.is_empty() {
            return Err("No hay repo de sincronización configurado.".into());
        }
        let token = token().ok_or("No encuentro un token de GitHub: inicia sesión con `gh auth login`.")?;
        let cliente = reqwest::blocking::Client::builder()
            .user_agent("zerostudy")
            .timeout(Duration::from_secs(25))
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self {
            repo: cfg.sync.repo.clone(),
            rama: if cfg.sync.rama.is_empty() { "main".into() } else { cfg.sync.rama.clone() },
            token,
            cliente,
        })
    }

    fn url(&self, ruta: &str) -> String {
        // `ZEROSTUDY_API_URL` solo sirve para probar contra un servidor local.
        let api = std::env::var("ZEROSTUDY_API_URL").unwrap_or_else(|_| "https://api.github.com".into());
        format!("{api}/repos/{}/contents/{}", self.repo, ruta)
    }

    fn pedir(&self, metodo: reqwest::Method, ruta: &str, accept: &str, cuerpo: Option<serde_json::Value>) -> Result<(u16, String), String> {
        let mut r = self
            .cliente
            .request(metodo.clone(), self.url(ruta))
            .bearer_auth(&self.token)
            .header("Accept", accept)
            .header("X-GitHub-Api-Version", "2022-11-28");
        if metodo == reqwest::Method::GET {
            r = r.query(&[("ref", &self.rama)]);
        }
        if let Some(c) = cuerpo {
            r = r.json(&c);
        }
        let resp = r.send().map_err(|_| "Sin conexión con GitHub".to_string())?;
        let estado = resp.status().as_u16();
        let texto = resp.text().map_err(|e| e.to_string())?;
        match estado {
            401 | 403 => Err("GitHub rechaza el token (¿sin permiso de contenido en ese repo?)".into()),
            _ => Ok((estado, texto)),
        }
    }

    pub fn leer(&self, ruta: &str) -> Result<Option<String>, String> {
        let (s, t) = self.pedir(reqwest::Method::GET, ruta, "application/vnd.github.raw+json", None)?;
        match s {
            200 => Ok(Some(t)),
            404 => Ok(None),
            otro => Err(format!("GitHub respondió {otro} al leer {ruta}")),
        }
    }

    fn sha(&self, ruta: &str) -> Result<Option<String>, String> {
        let (s, t) = self.pedir(reqwest::Method::GET, ruta, "application/vnd.github+json", None)?;
        if s == 404 {
            return Ok(None);
        }
        Ok(serde_json::from_str::<Entrada>(&t).ok().map(|e| e.sha))
    }

    pub fn listar(&self, dir: &str) -> Result<Vec<String>, String> {
        let (s, t) = self.pedir(reqwest::Method::GET, dir, "application/vnd.github+json", None)?;
        if s == 404 {
            return Ok(Vec::new());
        }
        let v: Vec<Entrada> = serde_json::from_str(&t).map_err(|_| format!("GitHub respondió {s} al listar {dir}"))?;
        Ok(v.into_iter().map(|e| e.name).filter(|n| n.ends_with(".json")).collect())
    }

    pub fn escribir(&self, ruta: &str, texto: &str, mensaje: &str) -> Result<(), String> {
        for _ in 0..2 {
            let mut cuerpo = serde_json::json!({
                "message": mensaje,
                "content": base64::engine::general_purpose::STANDARD.encode(texto.as_bytes()),
                "branch": self.rama,
            });
            if let Some(sha) = self.sha(ruta)? {
                cuerpo["sha"] = sha.into();
            }
            let (s, _) = self.pedir(reqwest::Method::PUT, ruta, "application/vnd.github+json", Some(cuerpo))?;
            match s {
                200 | 201 => return Ok(()),
                409 | 422 => continue, // otro dispositivo lo cambió entre medias: reintenta con el sha nuevo
                404 => return Err("No encuentro el repo de sincronización (¿nombre o permisos?).".into()),
                otro => return Err(format!("GitHub respondió {otro} al subir {ruta}")),
            }
        }
        Err("No se pudo subir tras reintentar".into())
    }
}

/// Qué ha pasado en una sincronización.
#[derive(Default)]
pub struct Resultado {
    pub subidos: usize,
    pub bajados: usize,
    /// Contenido descargado (solo si este equipo no tiene la carpeta de apuntes).
    pub contenido: Option<Contenido>,
}

fn hash_contenido(c: &Contenido) -> String {
    use sha1::{Digest, Sha1};
    let mut sin_hora = c.clone();
    sin_hora.generado.clear();
    let bytes = serde_json::to_vec(&sin_hora).unwrap_or_default();
    Sha1::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect()
}

/// `local`: el contenido leído de la carpeta de apuntes de este equipo, si la hay.
pub fn sincronizar(cfg: &Config, local: Option<&Contenido>) -> Result<Resultado, String> {
    let gh = Gh::nuevo(cfg)?;
    let mut r = Resultado::default();

    // 1. contenido.json, solo si cambió desde la última subida
    if let Some(c) = local {
        let marca = format!("{}:{}", cfg.sync.repo, hash_contenido(c));
        if std::fs::read_to_string(rutas::hash_subido()).ok().as_deref() != Some(marca.as_str()) {
            let texto = serde_json::to_string(c).map_err(|e| e.to_string())?;
            gh.escribir("contenido.json", &texto, "contenido: actualizado desde los apuntes")?;
            let _ = rutas::escribir(&rutas::hash_subido(), marca.as_bytes());
            r.subidos += 1;
        }
    }

    // 2. mi progreso
    let mio = format!("{}.json", cfg.dispositivo);
    if rutas::dispositivo_valido(&cfg.dispositivo) {
        if let Ok(texto) = std::fs::read_to_string(rutas::progreso().join(&mio)) {
            if let Ok(local) = serde_json::from_str::<Registro>(&texto) {
                let remoto = gh.leer(&format!("progreso/{mio}"))?.and_then(|t| serde_json::from_str::<Registro>(&t).ok());
                if remoto.as_ref() != Some(&local) {
                    gh.escribir(&format!("progreso/{mio}"), &serde_json::to_string_pretty(&local).unwrap_or(texto), &format!("progreso: {}", cfg.dispositivo))?;
                    r.subidos += 1;
                }
            }
        }
    }

    // 3. el de los demás dispositivos
    for nombre in gh.listar("progreso")? {
        let dispositivo = nombre.trim_end_matches(".json");
        if nombre == mio || !rutas::dispositivo_valido(dispositivo) {
            continue;
        }
        let Some(texto) = gh.leer(&format!("progreso/{nombre}"))? else { continue };
        let Ok(reg) = serde_json::from_str::<Registro>(&texto) else { continue };
        if reg.dispositivo != dispositivo {
            continue;
        }
        let bytes = serde_json::to_vec_pretty(&reg).map_err(|e| e.to_string())?;
        if rutas::escribir_si_cambia(&rutas::progreso().join(&nombre), &bytes).map_err(|e| e.to_string())? {
            r.bajados += 1;
        }
    }

    // 4. sin carpeta de apuntes aquí: el contenido viene del repo
    if local.is_none() {
        if let Some(t) = gh.leer("contenido.json")? {
            let c: Contenido = serde_json::from_str(&t).map_err(|e| format!("contenido.json del repo no es válido: {e}"))?;
            let _ = rutas::escribir(&rutas::contenido(), t.as_bytes());
            r.contenido = Some(c);
        }
    }
    Ok(r)
}
