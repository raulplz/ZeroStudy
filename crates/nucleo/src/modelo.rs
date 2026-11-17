//! Lo que se lee de la bóveda: asignaturas, sesiones, preguntas, plazos y conceptos.
//! Se serializa en `contenido.json`, que es lo que viaja al repo privado de datos.

use serde::{Deserialize, Serialize};

/// Todo el contenido de estudio, ya procesado.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Contenido {
    /// Nombre de la bóveda de Obsidian (para los enlaces `obsidian://`).
    pub boveda: String,
    /// Cuándo se generó (hora local, `AAAA-MM-DDTHH:MM`).
    #[serde(default)]
    pub generado: String,
    pub unidades: Vec<Unidad>,
    pub preguntas: Vec<Pregunta>,
    #[serde(default)]
    pub plazos: Vec<Plazo>,
    #[serde(default)]
    pub conceptos: Conceptos,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Unidad {
    pub id: String,
    pub titulo: String,
    pub corto: String,
    /// Color de la asignatura (hex).
    pub color: String,
    #[serde(default)]
    pub sesiones: Vec<Sesion>,
    /// Ruta de su resumen general dentro de la bóveda.
    #[serde(default)]
    pub resumen: String,
    /// Subcarpeta de la bóveda.
    #[serde(default)]
    pub carpeta: String,
    /// «clase» (asignatura con profesor y plazos) o «propio» (aprendizaje personal).
    #[serde(default = "clase")]
    pub tipo: String,
    /// Notas `.md` de la carpeta (para recomendar metodología).
    #[serde(default)]
    pub notas: u32,
    /// Bloques de código en sus notas.
    #[serde(default)]
    pub bloques_codigo: u32,
    /// Otros archivos (PDF, presentaciones…) junto a las notas.
    #[serde(default)]
    pub adjuntos: u32,
}

fn clase() -> String {
    "clase".into()
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Sesion {
    pub n: u32,
    pub nota: String,
    pub ruta: String,
    #[serde(default)]
    pub tema: String,
    #[serde(default)]
    pub bytes: u64,
    #[serde(default)]
    pub preguntas: u32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Pregunta {
    /// sha1(unidad␟nota␟pregunta)[..12]: estable mientras no cambie el enunciado.
    pub id: String,
    pub unidad: String,
    pub nota: String,
    pub ruta: String,
    #[serde(default)]
    pub seccion: String,
    #[serde(default)]
    pub sesion: Option<u32>,
    /// Viene de una nota «Preparación del test».
    #[serde(default)]
    pub test: bool,
    pub pregunta: String,
    /// Líneas antes de `**Respuesta: x)**` (opciones a–e), si las hay.
    #[serde(default)]
    pub opciones: String,
    pub respuesta: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Plazo {
    /// `AAAA-MM-DD`, o `None` si es «sin fecha».
    pub fecha: Option<String>,
    pub unidad: String,
    pub que: String,
    pub estado: String,
    #[serde(default)]
    pub nota: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Conceptos {
    pub nodos: Vec<Concepto>,
    pub aristas: Vec<Arista>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Concepto {
    pub id: String,
    pub nombre: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub definicion: String,
    #[serde(default)]
    pub nota: String,
    #[serde(default)]
    pub seccion: String,
    #[serde(default)]
    pub sesion: Option<u32>,
    #[serde(default)]
    pub tipo: String,
    #[serde(default = "uno")]
    pub importancia: u8,
    #[serde(default)]
    pub unidad: String,
    /// Posición en el atlas (la calcula [`crate::atlas::disponer`]).
    #[serde(default)]
    pub x: f64,
    #[serde(default)]
    pub y: f64,
}

fn uno() -> u8 {
    1
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Arista {
    pub de: String,
    pub a: String,
    pub tipo: String,
    #[serde(default)]
    pub etiqueta: String,
    /// Puente entre asignaturas (propuesto por Claude, no contenido de clase).
    #[serde(default)]
    pub cruzada: bool,
}

/// Verbo por defecto de cada tipo de relación.
pub fn verbo(tipo: &str) -> &str {
    match tipo {
        "es-un" => "es un tipo de",
        "parte-de" => "forma parte de",
        "usa" => "usa",
        "contrasta-con" => "contrasta con",
        "precede" => "va antes de",
        "ejemplo-de" => "es ejemplo de",
        "se-aplica-en" => "se aplica en",
        otro => otro,
    }
}

impl Contenido {
    pub fn pregunta(&self, id: &str) -> Option<&Pregunta> {
        self.preguntas.iter().find(|p| p.id == id)
    }
    pub fn concepto(&self, id: &str) -> Option<&Concepto> {
        self.conceptos.nodos.iter().find(|c| c.id == id)
    }
    pub fn unidad(&self, id: &str) -> Option<&Unidad> {
        self.unidades.iter().find(|u| u.id == id)
    }

    /// Preguntas que tratan un concepto: su nombre o un alias aparece en el enunciado
    /// (no en las opciones, para no enlazar por un distractor), o están en su misma sección.
    pub fn preguntas_de_concepto(&self, c: &Concepto) -> Vec<&Pregunta> {
        let terminos: Vec<String> = std::iter::once(&c.nombre)
            .chain(c.aliases.iter())
            .map(|t| normalizar(t))
            .filter(|t| t.chars().count() >= 3)
            .collect();
        self.preguntas
            .iter()
            .filter(|p| p.unidad == c.unidad)
            .filter(|p| {
                let enunciado = normalizar(&p.pregunta);
                terminos.iter().any(|t| contiene_palabra(&enunciado, t))
                    || (p.nota == c.nota && p.seccion == c.seccion)
            })
            .collect()
    }
}

/// Minúsculas y sin tildes (para buscar).
pub fn normalizar(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'á' | 'à' | 'ä' | 'â' | 'Á' | 'À' | 'Ä' | 'Â' => 'a',
            'é' | 'è' | 'ë' | 'ê' | 'É' | 'È' | 'Ë' | 'Ê' => 'e',
            'í' | 'ì' | 'ï' | 'î' | 'Í' | 'Ì' | 'Ï' | 'Î' => 'i',
            'ó' | 'ò' | 'ö' | 'ô' | 'Ó' | 'Ò' | 'Ö' | 'Ô' => 'o',
            'ú' | 'ù' | 'ü' | 'û' | 'Ú' | 'Ù' | 'Ü' | 'Û' => 'u',
            'ñ' | 'Ñ' => 'n',
            c => c.to_lowercase().next().unwrap_or(c),
        })
        .collect()
}

/// `aguja` aparece en `pajar` sin letras ni dígitos pegados a los lados.
fn contiene_palabra(pajar: &str, aguja: &str) -> bool {
    let alnum = |c: Option<char>| c.is_some_and(|c| c.is_ascii_alphanumeric());
    let mut desde = 0;
    while let Some(pos) = pajar[desde..].find(aguja) {
        let ini = desde + pos;
        let fin = ini + aguja.len();
        if !alnum(pajar[..ini].chars().next_back()) && !alnum(pajar[fin..].chars().next()) {
            return true;
        }
        desde = ini + aguja.chars().next().map_or(1, char::len_utf8);
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palabra_completa() {
        assert!(contiene_palabra("que es el edge computing?", "edge"));
        assert!(!contiene_palabra("knowledge", "edge"));
        assert_eq!(normalizar("Ingeniería"), "ingenieria");
    }
}
