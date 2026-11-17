//! Lector de apuntes en Markdown (bóveda de Obsidian o cualquier carpeta). Solo lee.
//!
//! - Preguntas: callouts `> [!question]-` (con opciones a–e antes de `**Respuesta: x)**`).
//! - Sesiones: notas cuyo nombre incluye «Sesión N», «Session N», «Tema N», «Lección N»…
//! - Plazos: tabla `| AAAA-MM-DD o sin fecha | asignatura | qué | estado | dónde |`.
//! - Conceptos: `<carpeta conceptos>/<id>.json` + `cruzadas.json` (formato de Frost Study).

use crate::config::{Asignatura, Config, COLORES};
use crate::modelo::*;
use regex_lite::Regex;
use sha1::{Digest, Sha1};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn re(patron: &'static str, celda: &'static OnceLock<Regex>) -> &'static Regex {
    celda.get_or_init(|| Regex::new(patron).unwrap())
}
fn re_callout() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    re(r"(?i)^>\s*\[!question\][-+]?\s*(.*)$", &R)
}
fn re_respuesta() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    re(r"(?i)^\s*(\*\*)?\s*respuesta\b", &R)
}
fn re_sesion() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    re(r"(?i)(?:sesi[oó]n|session|tema|lecci[oó]n|clase|semana|cap[ií]tulo)\s*(\d+)", &R)
}
fn re_sesion_sufijo() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    re(r"(?i)(?:sesi[oó]n|session)\s*\d+\s*-\s*(.+)$", &R)
}

/// Carpetas que no son asignaturas al detectar automáticamente.
const IGNORAR: &[&str] = &["atlas", "plantillas", "templates", "imagenes", "imágenes", "attachments", "adjuntos", "assets"];

fn id_pregunta(partes: &[&str]) -> String {
    let mut h = Sha1::new();
    h.update(partes.join("\u{1f}").as_bytes());
    h.finalize().iter().take(6).map(|b| format!("{b:02x}")).collect()
}

fn notas_md(carpeta: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = fs::read_dir(carpeta)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "md"))
        .filter(|p| p.file_name().is_some_and(|n| n != "CLAUDE.md" && n != "AGENTS.md"))
        .collect();
    v.sort();
    v
}

fn stem(p: &Path) -> String {
    p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
}

/// Subcarpetas de primer nivel con notas `.md`, como asignaturas «propio» o «clase».
pub fn detectar(raiz: &Path) -> Vec<Asignatura> {
    let mut v: Vec<PathBuf> = fs::read_dir(raiz)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .filter(|p| {
            let n = p.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
            !n.starts_with('.') && !IGNORAR.contains(&n.as_str())
        })
        .filter(|p| !notas_md(p).is_empty())
        .collect();
    v.sort();
    v.iter()
        .enumerate()
        .map(|(i, p)| {
            let carpeta = p.file_name().unwrap().to_string_lossy().into_owned();
            let titulo = carpeta.replace(['_', '-'], " ");
            let limpio: String = carpeta.chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>().to_lowercase();
            let id = limpio.strip_prefix("unit").filter(|r| !r.is_empty()).unwrap_or(&limpio).to_string();
            Asignatura {
                id: if id.is_empty() { format!("a{i}") } else { id },
                corto: titulo.split_whitespace().next().unwrap_or(&titulo).to_string(),
                titulo,
                carpeta,
                color: COLORES[i % COLORES.len()].into(),
                tipo: "clase".into(),
            }
        })
        .collect()
}

pub fn preguntas_de(raiz: &Path, nota: &Path, unidad: &str) -> io::Result<Vec<Pregunta>> {
    let texto = fs::read_to_string(nota)?;
    let lineas: Vec<&str> = texto.lines().collect();
    let nombre = stem(nota);
    let ruta = nota.strip_prefix(raiz).unwrap_or(nota).to_string_lossy().into_owned();
    let sesion = re_sesion().captures(&nombre).and_then(|c| c[1].parse().ok());
    let test = nombre.to_lowercase().contains("preparación del test") || nombre.to_lowercase().contains("test");
    let (mut salida, mut seccion, mut i) = (Vec::new(), String::new(), 0);
    while i < lineas.len() {
        let l = lineas[i];
        if l.starts_with('#') && l.trim_start_matches('#').starts_with(' ') {
            seccion = l.trim_start_matches('#').trim().to_string();
        }
        let Some(m) = re_callout().captures(l) else {
            i += 1;
            continue;
        };
        let mut titulo = m[1].trim().to_string();
        let mut cuerpo: Vec<String> = Vec::new();
        i += 1;
        while i < lineas.len() && lineas[i].starts_with('>') {
            let c = lineas[i].trim_start_matches('>');
            cuerpo.push(c.strip_prefix(' ').unwrap_or(c).to_string());
            i += 1;
        }
        let (mut opciones, mut respuesta): (Vec<String>, Vec<String>) = (Vec::new(), cuerpo.clone());
        if let Some(k) = cuerpo.iter().position(|c| re_respuesta().is_match(c)) {
            opciones = cuerpo[..k].to_vec();
            respuesta = cuerpo[k..].to_vec();
        }
        if titulo.is_empty() && !cuerpo.is_empty() {
            titulo = cuerpo[0].clone();
            respuesta = cuerpo[1..].to_vec();
        }
        salida.push(Pregunta {
            id: id_pregunta(&[unidad, &nombre, &titulo]),
            unidad: unidad.into(),
            nota: nombre.clone(),
            ruta: ruta.clone(),
            seccion: seccion.clone(),
            sesion,
            test,
            pregunta: titulo,
            opciones: opciones.join("\n").trim().to_string(),
            respuesta: respuesta.join("\n").trim().to_string(),
        });
    }
    Ok(salida)
}

fn frontmatter(texto: &str, clave: &str) -> Option<String> {
    let resto = texto.strip_prefix("---\n")?;
    let fin = resto.find("\n---")?;
    resto[..fin].lines().find_map(|l| {
        let (k, v) = l.split_once(':')?;
        (k.trim() == clave).then(|| v.trim().trim_matches(['"', '\'']).to_string()).filter(|v| !v.is_empty())
    })
}

fn sesiones_de(raiz: &Path, carpeta: &Path) -> Vec<Sesion> {
    let mut v: Vec<Sesion> = notas_md(carpeta)
        .iter()
        .filter_map(|nota| {
            let nombre = stem(nota);
            let n: u32 = re_sesion().captures(&nombre).filter(|_| nombre.to_lowercase().contains("ses"))?[1].parse().ok()?;
            let texto = fs::read_to_string(nota).ok()?;
            let mut tema = frontmatter(&texto, "tema").unwrap_or_default();
            if tema.is_empty() {
                if let Some(h1) = texto.lines().find_map(|l| l.strip_prefix("# ")) {
                    let quitar = Regex::new(r"(?i)^Unit \d+\s*[·:-]\s*Sesi[oó]n \d+\s*[·:-]?\s*").unwrap();
                    tema = quitar.replace(h1, "").trim().to_string();
                }
            }
            if tema.is_empty() {
                if let Some(h2) = texto.lines().find_map(|l| l.strip_prefix("## ")) {
                    tema = Regex::new(r"^[A-Z]?\d+\.\s*").unwrap().replace(h2, "").trim().to_string();
                }
            }
            if let Some(extra) = re_sesion_sufijo().captures(&nombre) {
                let e = extra[1].to_lowercase();
                tema = if tema.is_empty() { e } else { format!("{tema} ({e})") };
            }
            Some(Sesion {
                n,
                nota: nombre,
                ruta: nota.strip_prefix(raiz).unwrap_or(nota).to_string_lossy().into_owned(),
                tema,
                bytes: texto.len() as u64,
                preguntas: texto.lines().filter(|l| re_callout().is_match(l)).count() as u32,
            })
        })
        .collect();
    v.sort_by(|a, b| (a.n, &a.nota).cmp(&(b.n, &b.nota)));
    v
}

pub fn plazos(raiz: &Path, nota: &str) -> Vec<Plazo> {
    let Ok(texto) = fs::read_to_string(raiz.join(nota)) else { return Vec::new() };
    let fecha = Regex::new(r"(?i)^(\d{4}-\d{2}-\d{2}|sin fecha)$").unwrap();
    let enlace = Regex::new(r"\[\[([^\]|#]+)").unwrap();
    texto
        .lines()
        .filter_map(|l| {
            let celdas: Vec<&str> = l.trim().trim_matches('|').split('|').map(str::trim).collect();
            if celdas.len() < 4 || !fecha.is_match(celdas[0]) {
                return None;
            }
            Some(Plazo {
                fecha: (!celdas[0].eq_ignore_ascii_case("sin fecha")).then(|| celdas[0].to_string()),
                unidad: celdas[1].into(),
                que: celdas[2].into(),
                estado: celdas[3].to_lowercase(),
                nota: celdas.get(4).and_then(|c| enlace.captures(c)).map(|m| m[1].to_string()),
            })
        })
        .collect()
}

#[derive(serde::Deserialize, Default)]
struct ArchivoConceptos {
    #[serde(default)]
    conceptos: Vec<Concepto>,
    #[serde(default)]
    relaciones: Vec<Arista>,
}

pub fn conceptos(dir: &Path, unidades: &[String]) -> Conceptos {
    let leer = |f: PathBuf| -> ArchivoConceptos {
        fs::read_to_string(f).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
    };
    let mut c = Conceptos::default();
    for u in unidades {
        let d = leer(dir.join(format!("U{u}.json")));
        let d = if d.conceptos.is_empty() { leer(dir.join(format!("{u}.json"))) } else { d };
        c.nodos.extend(d.conceptos.into_iter().map(|mut x| {
            x.unidad = u.clone();
            x
        }));
        c.aristas.extend(d.relaciones);
    }
    c.aristas.extend(leer(dir.join("cruzadas.json")).relaciones.into_iter().map(|mut a| {
        a.cruzada = true;
        a
    }));
    let ids: std::collections::HashSet<String> = c.nodos.iter().map(|x| x.id.clone()).collect();
    c.aristas.retain(|a| ids.contains(&a.de) && ids.contains(&a.a));
    c
}

/// Lee todo el contenido según la configuración. `ahora` = hora local `AAAA-MM-DDTHH:MM`.
pub fn leer(cfg: &Config, ahora: &str) -> io::Result<Contenido> {
    let raiz = PathBuf::from(&cfg.boveda);
    if !raiz.is_dir() {
        return Err(io::Error::new(io::ErrorKind::NotFound, format!("No existe la carpeta de apuntes: {}", cfg.boveda)));
    }
    let asignaturas = if cfg.asignaturas.is_empty() { detectar(&raiz) } else { cfg.asignaturas.clone() };
    let (mut unidades, mut preguntas) = (Vec::new(), Vec::new());
    for a in &asignaturas {
        let carpeta = raiz.join(&a.carpeta);
        if !carpeta.is_dir() {
            continue;
        }
        let notas = notas_md(&carpeta);
        let (mut codigo, mut adjuntos) = (0u32, 0u32);
        for n in &notas {
            preguntas.extend(preguntas_de(&raiz, n, &a.id)?);
            codigo += fs::read_to_string(n).map(|t| t.matches("\n```").count() as u32 / 2).unwrap_or(0);
        }
        for e in fs::read_dir(&carpeta).into_iter().flatten().flatten() {
            let ext = e.path().extension().map(|x| x.to_string_lossy().to_lowercase()).unwrap_or_default();
            if ["pdf", "pptx", "docx", "ipynb", "py", "rs", "js"].contains(&ext.as_str()) {
                adjuntos += 1;
            }
        }
        let resumen = notas
            .iter()
            .find(|n| stem(n).to_lowercase().contains("resumen general"))
            .map(|n| n.strip_prefix(&raiz).unwrap_or(n).to_string_lossy().into_owned())
            .unwrap_or_default();
        unidades.push(Unidad {
            id: a.id.clone(),
            titulo: a.titulo.clone(),
            corto: a.corto.clone(),
            color: a.color.clone(),
            sesiones: sesiones_de(&raiz, &carpeta),
            resumen,
            carpeta: a.carpeta.clone(),
            tipo: if a.tipo.is_empty() { "clase".into() } else { a.tipo.clone() },
            notas: notas.len() as u32,
            bloques_codigo: codigo,
            adjuntos,
        });
    }
    let ids: Vec<String> = unidades.iter().map(|u| u.id.clone()).collect();
    let mut conc = if cfg.conceptos.is_empty() { Conceptos::default() } else { conceptos(Path::new(&cfg.conceptos), &ids) };
    crate::atlas::disponer(&mut conc, &ids);
    let nombre = if cfg.nombre_boveda.is_empty() {
        raiz.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
    } else {
        cfg.nombre_boveda.clone()
    };
    Ok(Contenido {
        boveda: nombre,
        generado: ahora.into(),
        unidades,
        preguntas,
        plazos: if cfg.plazos.is_empty() { Vec::new() } else { plazos(&raiz, &cfg.plazos) },
        conceptos: conc,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lee_carpeta_de_ejemplo() {
        let dir = std::env::temp_dir().join(format!("zerostudy-prueba-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("Rust")).unwrap();
        fs::write(
            dir.join("Rust/Tema 1 - Ownership.md"),
            "# Ownership\n\n## Préstamos\n\n> [!question]- ¿Qué es un préstamo?\n> Una referencia.\n\n```rust\nlet a = 1;\n```\n",
        )
        .unwrap();
        fs::write(dir.join("Rust/Test.md"), "> [!question]- ¿Cuál?\n> a) Uno\n> b) Dos\n> **Respuesta: b)** porque sí\n").unwrap();
        let cfg = Config { boveda: dir.to_string_lossy().into(), plazos: String::new(), ..Config::default() };
        let c = leer(&cfg, "2026-11-17T12:00").unwrap();
        assert_eq!(c.unidades.len(), 1);
        assert_eq!(c.unidades[0].bloques_codigo, 1);
        assert_eq!(c.preguntas.len(), 2);
        let p = c.preguntas.iter().find(|p| p.pregunta.contains("préstamo")).unwrap();
        assert_eq!((p.seccion.as_str(), p.sesion, p.respuesta.as_str()), ("Préstamos", Some(1), "Una referencia."));
        let t = c.preguntas.iter().find(|p| p.test).unwrap();
        assert!(crate::opciones::es_test(&t.opciones, &t.respuesta));
        fs::remove_dir_all(&dir).unwrap();
    }
}
