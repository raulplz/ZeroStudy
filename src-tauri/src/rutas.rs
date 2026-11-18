//! Rutas de ZeroStudy en el disco y escritura atómica.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

fn home() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."))
}

fn xdg(var: &str, relativo: &str) -> PathBuf {
    std::env::var_os(var).map(PathBuf::from).filter(|p| p.is_absolute()).unwrap_or_else(|| home().join(relativo))
}

pub fn config_dir() -> PathBuf {
    xdg("XDG_CONFIG_HOME", ".config")
}
pub fn config() -> PathBuf {
    config_dir().join("zerostudy/config.json")
}
/// Plantilla opcional (fuera del repo) que rellena el asistente de primer uso.
pub fn plantilla() -> PathBuf {
    config_dir().join("zerostudy/plantilla.json")
}
pub fn datos() -> PathBuf {
    xdg("XDG_DATA_HOME", ".local/share").join("zerostudy")
}
pub fn progreso() -> PathBuf {
    datos().join("progreso")
}
pub fn widget() -> PathBuf {
    datos().join("widget.json")
}
/// Última copia del contenido (la lee la app y se sube al repo de datos).
pub fn contenido() -> PathBuf {
    datos().join("contenido.json")
}
/// Hash del último contenido subido (para no subirlo si no cambió).
pub fn hash_subido() -> PathBuf {
    datos().join("contenido.sha1")
}

/// Un nombre de dispositivo seguro para usarlo como nombre de archivo.
pub fn dispositivo_valido(d: &str) -> bool {
    !d.is_empty() && d.len() <= 40 && d.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Escribe en un archivo temporal y lo renombra, para no dejar nunca un archivo a medias.
pub fn escribir(ruta: &Path, datos: &[u8]) -> io::Result<()> {
    if let Some(dir) = ruta.parent() {
        fs::create_dir_all(dir)?;
    }
    let nombre = ruta.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let tmp = ruta.with_file_name(format!(".{nombre}.zerostudy-tmp"));
    fs::write(&tmp, datos)?;
    fs::rename(&tmp, ruta).inspect_err(|_| {
        let _ = fs::remove_file(&tmp);
    })
}

/// Como [`escribir`], pero solo si el contenido es distinto del que ya hay.
pub fn escribir_si_cambia(ruta: &Path, datos: &[u8]) -> io::Result<bool> {
    if fs::read(ruta).is_ok_and(|actual| actual == datos) {
        return Ok(false);
    }
    escribir(ruta, datos).map(|_| true)
}
