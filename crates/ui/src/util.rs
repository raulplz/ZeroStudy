//! Utilidades de la interfaz: fechas locales, Markdown, enlaces y almacenamiento del navegador.

use js_sys::Date;
use pulldown_cmark::{html, Event, Options, Parser};

fn dos(n: f64) -> String {
    format!("{:02}", n as u32)
}

/// Fecha local `AAAA-MM-DD`.
pub fn hoy() -> String {
    let d = Date::new_0();
    format!("{}-{}-{}", d.get_full_year(), dos((d.get_month() + 1) as f64), dos(d.get_date() as f64))
}

/// Hora local `AAAA-MM-DDTHH:MM:SS`.
pub fn ahora() -> String {
    let d = Date::new_0();
    format!("{}T{}:{}:{}", hoy(), dos(d.get_hours() as f64), dos(d.get_minutes() as f64), dos(d.get_seconds() as f64))
}

pub fn hora() -> u32 {
    Date::new_0().get_hours()
}

pub fn ms() -> f64 {
    Date::now()
}

/// «martes, 17 de noviembre» (o el formato que se pida) en español.
pub fn fecha_es(iso: &str, opciones: &[(&str, &str)]) -> String {
    let d = Date::new(&format!("{}T12:00:00", &iso[..iso.len().min(10)]).into());
    let o = js_sys::Object::new();
    for (k, v) in opciones {
        let _ = js_sys::Reflect::set(&o, &(*k).into(), &(*v).into());
    }
    d.to_locale_date_string("es-ES", &o).into()
}

/// `[[nota|alias]]` → alias y `[[nota]]` → nota.
pub fn sin_wikilinks(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut resto = s;
    while let Some(i) = resto.find("[[") {
        out.push_str(&resto[..i]);
        match resto[i + 2..].find("]]") {
            Some(j) => {
                let dentro = &resto[i + 2..i + 2 + j];
                out.push_str(dentro.rsplit('|').next().unwrap_or(dentro));
                resto = &resto[i + 4 + j..];
            }
            None => {
                out.push_str(&resto[i..]);
                resto = "";
            }
        }
    }
    out.push_str(resto);
    out
}

/// Markdown → HTML. El HTML crudo de las notas se muestra como texto (nada se ejecuta).
pub fn md(s: &str) -> String {
    let texto = sin_wikilinks(s);
    let mut op = Options::empty();
    op.insert(Options::ENABLE_TABLES);
    op.insert(Options::ENABLE_STRIKETHROUGH);
    let eventos = Parser::new_ext(&texto, op).map(|e| match e {
        Event::Html(h) | Event::InlineHtml(h) => Event::Text(h),
        otro => otro,
    });
    let mut out = String::new();
    html::push_html(&mut out, eventos);
    out
}

/// Markdown de una línea (sin el `<p>` que lo envuelve).
pub fn md_linea(s: &str) -> String {
    let h = md(s);
    let h = h.trim();
    h.strip_prefix("<p>").and_then(|x| x.strip_suffix("</p>")).unwrap_or(h).to_string()
}

pub fn uri_obsidian(boveda: &str, ruta: &str) -> String {
    let ruta = ruta.strip_suffix(".md").unwrap_or(ruta);
    format!(
        "obsidian://open?vault={}&file={}",
        js_sys::encode_uri_component(boveda),
        js_sys::encode_uri_component(ruta)
    )
}

pub fn barajar<T>(v: &mut [T]) {
    for i in (1..v.len()).rev() {
        let j = (js_sys::Math::random() * (i as f64 + 1.0)) as usize;
        v.swap(i, j.min(i));
    }
}

fn almacen() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}
pub fn local_leer(k: &str) -> Option<String> {
    almacen()?.get_item(k).ok()?
}
pub fn local_guardar(k: &str, v: &str) {
    if let Some(a) = almacen() {
        let _ = a.set_item(k, v);
    }
}

pub fn ir(ruta: &str) {
    if let Some(w) = web_sys::window() {
        let _ = w.location().set_hash(&format!("/{ruta}"));
    }
}

pub fn ruta_actual() -> String {
    web_sys::window()
        .and_then(|w| w.location().hash().ok())
        .map(|h| h.trim_start_matches('#').trim_start_matches('/').to_string())
        .filter(|h| !h.is_empty())
        .unwrap_or_else(|| "panel".into())
}

/// Parámetro `clave` de la ruta `vista?clave=valor`.
pub fn param(ruta: &str, clave: &str) -> Option<String> {
    let (_, q) = ruta.split_once('?')?;
    q.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        (k == clave).then(|| js_sys::decode_uri_component(v).ok().map(String::from).unwrap_or_else(|| v.to_string()))
    })
}

pub fn pct(x: f64) -> u32 {
    (x * 100.0).round() as u32
}

pub fn coma(x: f64) -> String {
    format!("{x:.1}").replace('.', ",")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wikilinks() {
        assert_eq!(sin_wikilinks("ver [[Unit 16 - Sesión 1#A|la sesión]] y [[Otra]]"), "ver la sesión y Otra");
    }
}
