//! Tema visual compartido con ZeroLayer (`~/.config/zerolayer/theme.json`).
//!
//! La lectura, la validación y los valores Frost viven en el crate `zerolayer-theme`; aquí solo se
//! fija cómo se llaman las variables CSS de ZeroStudy.

pub use zerolayer_theme::Theme as Tema;
use zerolayer_theme::{CssMap, Key};

/// Variables CSS de la app. Todo valor ya viene validado por `zerolayer-theme`
/// (el archivo lo escribe otra herramienta: no se inyecta nada arbitrario en el estilo).
pub fn variables_css(tema: &Tema) -> Vec<(&'static str, String)> {
    let mapa = CssMap::default()
        .rename(Key::Superficie, "--panel-2")
        .rename(Key::Borde, "--linea")
        .rename(Key::Tenue, "--texto-2")
        .rename(Key::Acento2, "--cian")
        .rename(Key::Ok, "--verde")
        .rename(Key::Aviso, "--ambar")
        .rename(Key::Peligro, "--rojo")
        .fallbacks("\"Noto Sans\", system-ui, sans-serif", "\"JetBrains Mono\", monospace");
    tema.css_variables(&mapa)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nombres_de_variables_de_zerostudy() {
        let t: Tema =
            serde_json::from_str(r##"{"id":"zero-corp","colores":{"acento":"#e8182c","peligro":"red; background:url(x)"},"fuentes":{"sans":"x\"; }"}}"##)
                .unwrap();
        let v = variables_css(&t);
        let get = |k: &str| v.iter().find(|(n, _)| *n == k).map(|(_, x)| x.as_str());
        assert_eq!(get("--acento"), Some("#e8182c"));
        assert_eq!(get("--rojo"), Some("#ef7180"), "valor inválido: Frost");
        assert_eq!(get("--fondo"), Some("#080b10"));
        assert_eq!(get("--sans"), Some("\"x\", \"Noto Sans\", system-ui, sans-serif"));
        assert!(get("--superficie").is_none() && get("--panel-2").is_some());
    }
}
