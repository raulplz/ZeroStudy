//! Configuración de ZeroStudy (`~/.config/zerostudy/config.json` en escritorio).
//! Vive en el núcleo para que la web y el asistente de primer uso usen el mismo formato.

use crate::metodo::Objetivo;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Config {
    /// Carpeta raíz de los apuntes (una bóveda de Obsidian o cualquier carpeta con `.md`).
    pub boveda: String,
    /// Nombre de la bóveda en Obsidian (para los enlaces `obsidian://`); por defecto, el de la carpeta.
    pub nombre_boveda: String,
    /// Asignaturas o temas. Vacío = se detectan las subcarpetas con notas.
    pub asignaturas: Vec<Asignatura>,
    /// Carpeta con los mapas de conceptos `<id>.json` + `cruzadas.json` (opcional).
    pub conceptos: String,
    /// Nota con la tabla de plazos, relativa a la bóveda (opcional).
    pub plazos: String,
    /// Nombre de este dispositivo en la sincronización (`portatil`, `pc-casa`, `iphone`…).
    pub dispositivo: String,
    pub sync: Sync,
    pub objetivo: Objetivo,
    /// Metodologías activas (ids de [`crate::metodo::CATALOGO`]).
    pub metodos: Vec<String>,
    /// Ya pasó el asistente de primer uso.
    pub configurado: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            boveda: String::new(),
            nombre_boveda: String::new(),
            asignaturas: Vec::new(),
            conceptos: String::new(),
            plazos: "00 - Plazos.md".into(),
            dispositivo: "portatil".into(),
            sync: Sync::default(),
            objetivo: Objetivo::Examenes,
            metodos: vec!["espaciada".into(), "recuperacion".into()],
            configurado: false,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Asignatura {
    pub id: String,
    /// Subcarpeta dentro de la bóveda.
    pub carpeta: String,
    pub titulo: String,
    pub corto: String,
    pub color: String,
    /// «clase» o «propio».
    pub tipo: String,
}

/// Sincronización con un repo privado de GitHub (`usuario/repo`). Vacío = sin sincronizar.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Sync {
    pub repo: String,
    /// Rama (por defecto `main`).
    pub rama: String,
}

/// Paleta para asignaturas detectadas automáticamente.
pub const COLORES: [&str; 8] = ["#75c4e8", "#4da3ff", "#8fbea0", "#e6bf79", "#c49bff", "#ef7180", "#dc9d70", "#9faee8"];
