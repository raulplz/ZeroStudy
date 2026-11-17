//! Preguntas tipo test: opciones `a) …` y letra correcta en `**Respuesta: c)**`.

use regex_lite::Regex;
use std::sync::OnceLock;

#[derive(Clone, Debug, PartialEq)]
pub struct Opcion {
    pub letra: char,
    pub texto: String,
}

fn re_opcion() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"^\s*(?:[-*]\s*)?([a-eA-E])[\).]\s+(.*)$").unwrap())
}

fn re_respuesta() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"(?i)respuesta\s*:?\s*\**\s*:?\s*([a-e])\)").unwrap())
}

/// Opciones a–e; una línea que no empieza por letra continúa la opción anterior.
pub fn parsear(texto: &str) -> Vec<Opcion> {
    let mut ops: Vec<Opcion> = Vec::new();
    for linea in texto.lines() {
        if let Some(c) = re_opcion().captures(linea) {
            ops.push(Opcion {
                letra: c[1].chars().next().unwrap().to_ascii_lowercase(),
                texto: c[2].trim().to_string(),
            });
        } else if let (Some(ultima), false) = (ops.last_mut(), linea.trim().is_empty()) {
            ultima.texto.push(' ');
            ultima.texto.push_str(linea.trim());
        }
    }
    ops
}

pub fn letra_correcta(respuesta: &str) -> Option<char> {
    re_respuesta()
        .captures(respuesta)
        .and_then(|c| c[1].chars().next())
        .map(|c| c.to_ascii_lowercase())
}

/// Una pregunta se corrige sola si tiene al menos dos opciones y una letra correcta.
pub fn es_test(opciones: &str, respuesta: &str) -> bool {
    parsear(opciones).len() >= 2 && letra_correcta(respuesta).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opciones_y_respuesta() {
        let o = parsear("a) Uno\nb) Dos que\n   sigue\n- c. Tres");
        assert_eq!(o.len(), 3);
        assert_eq!(o[1].texto, "Dos que sigue");
        assert_eq!(o[2].letra, 'c');
        assert_eq!(letra_correcta("**Respuesta: b)** porque…"), Some('b'));
        assert_eq!(letra_correcta("Respuesta sin letra"), None);
        assert!(es_test("a) x\nb) y", "**Respuesta: a)**"));
    }
}
