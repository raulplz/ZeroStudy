//! Tema visual compartido con ZeroLayer (`~/.config/zerolayer/theme.json`).

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Tema {
    #[serde(default = "id_frost")]
    pub id: String,
    #[serde(default)]
    pub nombre: String,
    #[serde(default)]
    pub modo: String,
    #[serde(default)]
    pub colores: Colores,
    #[serde(default)]
    pub glifo: String,
    #[serde(default)]
    pub kanji: String,
    #[serde(default)]
    pub fuentes: Fuentes,
}

fn id_frost() -> String {
    "zero-frost".into()
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Colores {
    pub fondo: String,
    pub superficie: String,
    pub borde: String,
    pub texto: String,
    pub tenue: String,
    pub acento: String,
    pub acento2: String,
    pub ok: String,
    pub aviso: String,
    pub peligro: String,
}

impl Default for Colores {
    fn default() -> Self {
        Self {
            fondo: "#080b10".into(),
            superficie: "#111923".into(),
            borde: "#1c2a3a".into(),
            texto: "#d5dfeb".into(),
            tenue: "#8292a6".into(),
            acento: "#4da3ff".into(),
            acento2: "#75c4e8".into(),
            ok: "#8fbea0".into(),
            aviso: "#e6bf79".into(),
            peligro: "#ef7180".into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Fuentes {
    pub sans: String,
    pub mono: String,
}

impl Default for Fuentes {
    fn default() -> Self {
        Self { sans: "Inter Tight".into(), mono: "JetBrainsMono Nerd Font".into() }
    }
}

impl Default for Tema {
    fn default() -> Self {
        Self {
            id: id_frost(),
            nombre: "ZERØ Frost".into(),
            modo: "dark".into(),
            colores: Colores::default(),
            glifo: "Ø".into(),
            kanji: String::new(),
            fuentes: Fuentes::default(),
        }
    }
}

fn hex_valido(s: &str) -> bool {
    s.len() == 7 && s.starts_with('#') && s[1..].chars().all(|c| c.is_ascii_hexdigit())
}

fn nombre_fuente_valido(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_alphanumeric() || " -_".contains(c))
}

impl Tema {
    /// Variables CSS de la app. Los valores que no son colores hex válidos se ignoran
    /// (el archivo lo escribe otra herramienta: no se inyecta nada arbitrario en el estilo).
    pub fn variables_css(&self) -> Vec<(&'static str, String)> {
        let c = &self.colores;
        let mut v: Vec<(&'static str, String)> = [
            ("--fondo", &c.fondo),
            ("--panel-2", &c.superficie),
            ("--linea", &c.borde),
            ("--texto", &c.texto),
            ("--texto-2", &c.tenue),
            ("--acento", &c.acento),
            ("--cian", &c.acento2),
            ("--verde", &c.ok),
            ("--ambar", &c.aviso),
            ("--rojo", &c.peligro),
        ]
        .into_iter()
        .filter(|(_, x)| hex_valido(x))
        .map(|(k, x)| (k, x.clone()))
        .collect();
        if nombre_fuente_valido(&self.fuentes.sans) {
            v.push(("--sans", format!("\"{}\", \"Noto Sans\", system-ui, sans-serif", self.fuentes.sans)));
        }
        if nombre_fuente_valido(&self.fuentes.mono) {
            v.push(("--mono", format!("\"{}\", \"JetBrains Mono\", monospace", self.fuentes.mono)));
        }
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignora_valores_raros() {
        let mut t = Tema::default();
        t.colores.acento = "red; background:url(x)".into();
        t.fuentes.sans = "x\"; }".into();
        let v = t.variables_css();
        assert!(!v.iter().any(|(k, _)| *k == "--acento" || *k == "--sans"));
        assert!(v.iter().any(|(k, x)| *k == "--fondo" && x == "#080b10"));
        let parcial: Tema = serde_json::from_str(r##"{"id":"zero-corp","colores":{"acento":"#e8182c"}}"##).unwrap();
        assert_eq!(parcial.colores.acento, "#e8182c");
        assert_eq!(parcial.colores.fondo, "#080b10");
    }
}
