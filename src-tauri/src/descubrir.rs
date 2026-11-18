//! Descubrimiento de la carpeta de apuntes y plantilla opcional del primer uso.

use std::fs;
use std::path::{Path, PathBuf};
use zerostudy_nucleo::config::Config;

/// Lee `plantilla.json` si existe y es válida (misma forma que `config.json`).
pub fn leer_plantilla(ruta: &Path) -> Option<Config> {
    let mut c: Config = serde_json::from_str(&fs::read_to_string(ruta).ok()?).ok()?;
    c.configurado = false;
    Some(c)
}

const IGNORAR: [&str; 6] = ["node_modules", "target", ".git", ".venv", "venv", "dist"];

/// Busca `dir` (y subcarpetas hasta `prof`) con una carpeta `.obsidian`.
/// Con `requiere` solo cuenta si algún ancestro (dentro de la búsqueda) se llama así.
fn buscar(dir: &Path, prof: u8, requiere: Option<&str>, dentro: bool) -> Option<PathBuf> {
    let nombre = dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let dentro = dentro || requiere.is_none_or(|r| nombre == r);
    if dentro && dir.join(".obsidian").is_dir() {
        return Some(dir.to_path_buf());
    }
    if prof == 0 {
        return None;
    }
    let mut hijos: Vec<PathBuf> = fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .filter(|e| {
            let n = e.file_name().to_string_lossy().into_owned();
            !n.starts_with('.') && !IGNORAR.contains(&n.as_str())
        })
        .map(|e| e.path())
        .collect();
    hijos.sort();
    hijos.into_iter().find_map(|h| buscar(&h, prof - 1, requiere, dentro))
}

/// Primera bóveda de Obsidian en `~/Documents`, `~/Obsidian` o bajo una carpeta `NOTAS` de `~/Work`.
pub fn sugerir_boveda(home: &Path) -> Option<String> {
    let rutas: [(PathBuf, u8, Option<&str>); 3] = [(home.join("Documents"), 3, None), (home.join("Obsidian"), 3, None), (home.join("Work"), 5, Some("NOTAS"))];
    rutas.iter().filter(|(r, _, _)| r.is_dir()).find_map(|(r, p, req)| buscar(r, *p, *req, false)).map(|p| p.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(nombre: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("zs-test-{nombre}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn encuentra_boveda_en_documents() {
        let h = tmp("doc");
        fs::create_dir_all(h.join("Documents/Uni/.obsidian")).unwrap();
        assert_eq!(sugerir_boveda(&h), Some(h.join("Documents/Uni").to_string_lossy().into_owned()));
        let _ = fs::remove_dir_all(h);
    }

    #[test]
    fn en_work_solo_bajo_notas_y_con_limite_de_profundidad() {
        let h = tmp("work");
        fs::create_dir_all(h.join("Work/PROYECTOS/app/.obsidian")).unwrap();
        assert_eq!(sugerir_boveda(&h), None, "fuera de NOTAS no cuenta");
        fs::create_dir_all(h.join("Work/UNI/NOTAS/Mi boveda/.obsidian")).unwrap();
        assert_eq!(sugerir_boveda(&h), Some(h.join("Work/UNI/NOTAS/Mi boveda").to_string_lossy().into_owned()));
        let _ = fs::remove_dir_all(h);
    }

    #[test]
    fn ignora_carpetas_pesadas_y_sin_bovedas() {
        let h = tmp("vacio");
        fs::create_dir_all(h.join("Documents/node_modules/x/.obsidian")).unwrap();
        fs::create_dir_all(h.join("Documents/a/b/c/d/.obsidian")).unwrap();
        assert_eq!(sugerir_boveda(&h), None);
        let _ = fs::remove_dir_all(h);
    }

    #[test]
    fn plantilla_valida_invalida_y_ausente() {
        let h = tmp("plantilla");
        let p = h.join("plantilla.json");
        assert!(leer_plantilla(&p).is_none());
        fs::write(&p, "no es json").unwrap();
        assert!(leer_plantilla(&p).is_none());
        fs::write(&p, r#"{"boveda":"/x","configurado":true,"sync":{"repo":"u/r"}}"#).unwrap();
        let c = leer_plantilla(&p).unwrap();
        assert_eq!((c.boveda.as_str(), c.sync.repo.as_str(), c.configurado), ("/x", "u/r", false));
        let _ = fs::remove_dir_all(h);
    }
}
