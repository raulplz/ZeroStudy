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
    let de_gh =
        Command::new("gh").args(["auth", "token"]).output().ok().filter(|o| o.status.success()).map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());
    de_gh.filter(|t| !t.is_empty()).or_else(|| ["GH_TOKEN", "GITHUB_TOKEN"].iter().find_map(|v| std::env::var(v).ok()).filter(|t| !t.is_empty()))
}

impl Gh {
    pub fn nuevo(cfg: &Config) -> Result<Self, String> {
        if cfg.sync.repo.is_empty() {
            return Err("No hay repo de sincronización configurado.".into());
        }
        let token = token().ok_or("No encuentro un token de GitHub: inicia sesión con `gh auth login`.")?;
        let cliente = reqwest::blocking::Client::builder().user_agent("zerostudy").timeout(Duration::from_secs(25)).build().map_err(|e| e.to_string())?;
        Ok(Self { repo: cfg.sync.repo.clone(), rama: if cfg.sync.rama.is_empty() { "main".into() } else { cfg.sync.rama.clone() }, token, cliente })
    }

    fn url(&self, ruta: &str) -> String {
        // `ZEROSTUDY_API_URL` solo sirve para probar contra un servidor local.
        let api = std::env::var("ZEROSTUDY_API_URL").unwrap_or_else(|_| "https://api.github.com".into());
        format!("{api}/repos/{}/contents/{}", self.repo, ruta)
    }

    fn pedir(&self, metodo: reqwest::Method, ruta: &str, accept: &str, cuerpo: Option<serde_json::Value>) -> Result<(u16, String), String> {
        let mut r =
            self.cliente.request(metodo.clone(), self.url(ruta)).bearer_auth(&self.token).header("Accept", accept).header("X-GitHub-Api-Version", "2022-11-28");
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
#[derive(Debug, Default)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    /// Servidor mínimo con la forma de la API de contenidos de GitHub.
    #[derive(Default)]
    struct Estado {
        archivos: HashMap<String, String>,
        peticiones: Vec<String>,
        /// Respuestas forzadas para las próximas peticiones PUT (p. ej. 422).
        forzar_put: Vec<u16>,
        /// Si es true, todo responde 401.
        rechazar: bool,
    }

    struct Mock {
        url: String,
        e: Arc<Mutex<Estado>>,
    }

    fn sha(t: &str) -> String {
        use sha1::{Digest, Sha1};
        Sha1::digest(t.as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
    }

    fn mock() -> Mock {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", l.local_addr().unwrap());
        let e = Arc::new(Mutex::new(Estado::default()));
        let e2 = e.clone();
        std::thread::spawn(move || {
            for conn in l.incoming().flatten() {
                let e = e2.clone();
                std::thread::spawn(move || atender(conn, e));
            }
        });
        Mock { url, e }
    }

    fn atender(conn: std::net::TcpStream, e: Arc<Mutex<Estado>>) {
        let mut w = conn.try_clone().unwrap();
        let mut r = BufReader::new(conn);
        loop {
            let mut linea = String::new();
            if r.read_line(&mut linea).unwrap_or(0) == 0 {
                return;
            }
            let mut p = linea.split_whitespace();
            let (metodo, ruta) = (p.next().unwrap_or("").to_string(), p.next().unwrap_or("").to_string());
            let (mut largo, mut accept, mut auth) = (0usize, String::new(), String::new());
            loop {
                let mut h = String::new();
                r.read_line(&mut h).unwrap();
                let h = h.trim_end().to_string();
                if h.is_empty() {
                    break;
                }
                let h_min = h.to_lowercase();
                if let Some(v) = h_min.strip_prefix("content-length:") {
                    largo = v.trim().parse().unwrap_or(0);
                } else if let Some(v) = h_min.strip_prefix("accept:") {
                    accept = v.trim().to_string();
                } else if h_min.starts_with("authorization:") {
                    auth = h[14..].trim().to_string();
                }
            }
            let mut cuerpo = vec![0u8; largo];
            r.read_exact(&mut cuerpo).unwrap();
            let ruta = ruta.split('?').next().unwrap().split("/contents/").nth(1).unwrap_or("").to_string();
            let (codigo, texto) = responder(&e, &metodo, &ruta, &accept, &auth, &cuerpo);
            let resp = format!("HTTP/1.1 {codigo} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{texto}", texto.len());
            if w.write_all(resp.as_bytes()).is_err() {
                return;
            }
        }
    }

    fn responder(e: &Mutex<Estado>, metodo: &str, ruta: &str, accept: &str, auth: &str, cuerpo: &[u8]) -> (u16, String) {
        let mut s = e.lock().unwrap();
        s.peticiones.push(format!("{metodo} {ruta}"));
        if s.rechazar || !auth.starts_with("Bearer ") {
            return (401, "{}".into());
        }
        match metodo {
            "GET" => {
                if let Some(t) = s.archivos.get(ruta) {
                    if accept.contains("raw") {
                        return (200, t.clone());
                    }
                    return (200, serde_json::json!({"name": ruta.rsplit('/').next(), "sha": sha(t)}).to_string());
                }
                let mut hijos: Vec<_> = s
                    .archivos
                    .iter()
                    .filter(|(k, _)| k.starts_with(&format!("{ruta}/")))
                    .map(|(k, v)| serde_json::json!({"name": k.rsplit('/').next(), "sha": sha(v)}))
                    .collect();
                hijos.sort_by_key(|h| h["name"].to_string());
                if hijos.is_empty() {
                    (404, "{}".into())
                } else {
                    (200, serde_json::Value::Array(hijos).to_string())
                }
            }
            "PUT" => {
                if !s.forzar_put.is_empty() {
                    return (s.forzar_put.remove(0), "{}".into());
                }
                let v: serde_json::Value = serde_json::from_slice(cuerpo).unwrap();
                let existe = s.archivos.get(ruta).map(|t| sha(t));
                let dado = v["sha"].as_str().map(String::from);
                if existe.is_some() && existe != dado {
                    return (422, "{}".into()); // como GitHub: hace falta el sha actual
                }
                let texto = base64::engine::general_purpose::STANDARD.decode(v["content"].as_str().unwrap()).unwrap();
                s.archivos.insert(ruta.to_string(), String::from_utf8(texto).unwrap());
                (if existe.is_some() { 200 } else { 201 }, "{}".into())
            }
            _ => (405, "{}".into()),
        }
    }

    /// Entorno aislado: HOME/XDG en un directorio temporal, sin `gh` y con token falso.
    struct Entorno {
        dir: PathBuf,
        _g: std::sync::MutexGuard<'static, ()>,
    }

    fn entorno(nombre: &str, url: &str) -> Entorno {
        static CERROJO: Mutex<()> = Mutex::new(());
        let g = CERROJO.lock().unwrap_or_else(|p| p.into_inner());
        let dir = std::env::temp_dir().join(format!("zs-sync-{nombre}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        // SAFETY: los tests de este módulo se serializan con el cerrojo.
        unsafe {
            std::env::set_var("HOME", &dir);
            std::env::set_var("XDG_CONFIG_HOME", dir.join("c"));
            std::env::set_var("XDG_DATA_HOME", dir.join("d"));
            std::env::set_var("PATH", "");
            std::env::set_var("GH_TOKEN", "token-falso");
            std::env::set_var("ZEROSTUDY_API_URL", url);
        }
        Entorno { dir, _g: g }
    }

    impl Drop for Entorno {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    fn cfg(dispositivo: &str) -> Config {
        let mut c = Config { dispositivo: dispositivo.into(), ..Config::default() };
        c.sync.repo = "usuario/datos".into();
        c
    }

    fn registro(d: &str, n: u8) -> Registro {
        let mut r = Registro::nuevo(d);
        for i in 0..n {
            r.anotar(&format!("2026-11-1{i}T10:00:00"), "q1", 3);
        }
        r
    }

    fn guardar_local(r: &Registro) {
        let ruta = rutas::progreso().join(format!("{}.json", r.dispositivo));
        rutas::escribir(&ruta, serde_json::to_string(r).unwrap().as_bytes()).unwrap();
    }

    fn contenido(nombre: &str) -> Contenido {
        Contenido { boveda: nombre.into(), generado: "2026-11-18T10:00".into(), ..Default::default() }
    }

    fn poner(m: &Mock, ruta: &str, t: &str) {
        m.e.lock().unwrap().archivos.insert(ruta.into(), t.into());
    }

    fn puts(m: &Mock) -> usize {
        m.e.lock().unwrap().peticiones.iter().filter(|p| p.starts_with("PUT")).count()
    }

    #[test]
    fn sube_contenido_y_progreso_propio_y_es_idempotente() {
        let m = mock();
        let _e = entorno("sube", &m.url);
        guardar_local(&registro("portatil", 2));
        let c = contenido("Apuntes");
        let r = sincronizar(&cfg("portatil"), Some(&c)).unwrap();
        assert_eq!((r.subidos, r.bajados), (2, 0));
        {
            let s = m.e.lock().unwrap();
            assert!(s.archivos.contains_key("contenido.json"));
            let subido: Registro = serde_json::from_str(&s.archivos["progreso/portatil.json"]).unwrap();
            assert_eq!(subido, registro("portatil", 2));
        }
        // Segunda vez: nada cambió, no se sube nada.
        let antes = puts(&m);
        let r = sincronizar(&cfg("portatil"), Some(&c)).unwrap();
        assert_eq!((r.subidos, r.bajados), (0, 0));
        assert_eq!(puts(&m), antes);
        // Cambia el progreso local: se actualiza (con el sha del archivo existente).
        guardar_local(&registro("portatil", 3));
        let r = sincronizar(&cfg("portatil"), Some(&c)).unwrap();
        assert_eq!(r.subidos, 1);
        let s = m.e.lock().unwrap();
        assert_eq!(serde_json::from_str::<Registro>(&s.archivos["progreso/portatil.json"]).unwrap().eventos.len(), 3);
    }

    #[test]
    fn resube_contenido_solo_si_cambia_pero_ignora_la_hora() {
        let m = mock();
        let _e = entorno("hash", &m.url);
        let mut c = contenido("Apuntes");
        sincronizar(&cfg("portatil"), Some(&c)).unwrap();
        c.generado = "2026-11-19T09:00".into(); // solo cambia la hora
        assert_eq!(sincronizar(&cfg("portatil"), Some(&c)).unwrap().subidos, 0);
        c.boveda = "Otra".into();
        assert_eq!(sincronizar(&cfg("portatil"), Some(&c)).unwrap().subidos, 1);
        assert!(m.e.lock().unwrap().archivos["contenido.json"].contains("Otra"));
    }

    #[test]
    fn baja_el_progreso_de_otros_dispositivos_y_no_repite() {
        let m = mock();
        let _e = entorno("baja", &m.url);
        let iphone = registro("iphone", 2);
        poner(&m, "progreso/iphone.json", &serde_json::to_string(&iphone).unwrap());
        let r = sincronizar(&cfg("portatil"), Some(&contenido("A"))).unwrap();
        assert_eq!(r.bajados, 1);
        let local: Registro = serde_json::from_str(&std::fs::read_to_string(rutas::progreso().join("iphone.json")).unwrap()).unwrap();
        assert_eq!(local, iphone);
        assert_eq!(sincronizar(&cfg("portatil"), Some(&contenido("A"))).unwrap().bajados, 0);
        // El iPhone avanza: se baja de nuevo la versión nueva.
        poner(&m, "progreso/iphone.json", &serde_json::to_string(&registro("iphone", 4)).unwrap());
        assert_eq!(sincronizar(&cfg("portatil"), Some(&contenido("A"))).unwrap().bajados, 1);
    }

    #[test]
    fn descarta_registros_ajenos_invalidos() {
        let m = mock();
        let _e = entorno("invalidos", &m.url);
        // dispositivo del contenido distinto del nombre del archivo, JSON roto y nombre peligroso
        poner(&m, "progreso/falso.json", &serde_json::to_string(&registro("otro", 1)).unwrap());
        poner(&m, "progreso/roto.json", "{no es json");
        poner(&m, "progreso/mal nombre.json", &serde_json::to_string(&registro("mal nombre", 1)).unwrap());
        let r = sincronizar(&cfg("portatil"), Some(&contenido("A"))).unwrap();
        assert_eq!(r.bajados, 0);
        let hay: Vec<_> = std::fs::read_dir(rutas::progreso()).map(|d| d.flatten().map(|e| e.file_name()).collect()).unwrap_or_default();
        assert!(hay.is_empty(), "{hay:?}");
    }

    #[test]
    fn sin_carpeta_de_apuntes_baja_el_contenido_del_repo() {
        let m = mock();
        let _e = entorno("movil", &m.url);
        let c = contenido("Desde el portátil");
        poner(&m, "contenido.json", &serde_json::to_string(&c).unwrap());
        let r = sincronizar(&cfg("movil"), None).unwrap();
        assert_eq!(r.contenido, Some(c));
        assert!(rutas::contenido().is_file(), "guarda copia local");
        // Sin contenido en el repo no falla.
        m.e.lock().unwrap().archivos.remove("contenido.json");
        assert_eq!(sincronizar(&cfg("movil"), None).unwrap().contenido, None);
    }

    #[test]
    fn reintenta_si_otro_dispositivo_cambia_el_archivo_entre_medias() {
        let m = mock();
        let _e = entorno("reintento", &m.url);
        guardar_local(&registro("portatil", 1));
        m.e.lock().unwrap().forzar_put = vec![409];
        let r = sincronizar(&cfg("portatil"), None).unwrap();
        assert_eq!(r.subidos, 1);
        assert!(m.e.lock().unwrap().archivos.contains_key("progreso/portatil.json"));
        // Si falla siempre, acaba con error y no se queda colgado.
        guardar_local(&registro("portatil", 2));
        m.e.lock().unwrap().forzar_put = vec![422, 422];
        assert!(sincronizar(&cfg("portatil"), None).unwrap_err().contains("reintentar"));
    }

    #[test]
    fn errores_claros() {
        let m = mock();
        let _e = entorno("errores", &m.url);
        let mut sin_repo = cfg("portatil");
        sin_repo.sync.repo.clear();
        assert!(sincronizar(&sin_repo, None).unwrap_err().contains("repo"));
        m.e.lock().unwrap().rechazar = true;
        assert!(sincronizar(&cfg("portatil"), None).unwrap_err().contains("rechaza el token"));
        m.e.lock().unwrap().rechazar = false;
        unsafe { std::env::set_var("ZEROSTUDY_API_URL", "http://127.0.0.1:9") };
        assert!(sincronizar(&cfg("portatil"), None).unwrap_err().contains("Sin conexión"));
        unsafe {
            std::env::set_var("ZEROSTUDY_API_URL", &m.url);
            std::env::remove_var("GH_TOKEN");
            std::env::remove_var("GITHUB_TOKEN");
        }
        assert!(sincronizar(&cfg("portatil"), None).unwrap_err().contains("token"));
    }

    #[test]
    fn dispositivo_invalido_no_sube_progreso() {
        let m = mock();
        let _e = entorno("dispositivo", &m.url);
        let r = sincronizar(&cfg("../malo"), None).unwrap();
        assert_eq!((r.subidos, puts(&m)), (0, 0));
    }
}
