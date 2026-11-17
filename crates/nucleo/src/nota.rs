//! Nota «00 - Progreso de repaso» que se escribe en la bóveda tras cada guardado.

use crate::modelo::Contenido;
use crate::progreso::{Estado, CAJA_DOMINADA};

pub fn progreso(c: &Contenido, e: &Estado, hoy: &str) -> String {
    let mut s = String::new();
    s += "---\ntipo: progreso-repaso\n";
    s += &format!("actualizado: {hoy}\ngenerado: zerostudy\ntags:\n  - universidad\n  - repaso\n---\n\n");
    s += "# Progreso de repaso\n\n";
    s += "> [!info] Nota generada por ZeroStudy al repasar (en cualquier dispositivo). No la edites a mano: se sobrescribe.\n\n";
    s += "| Asignatura | Preguntas | Vistas | Dominadas (caja ≥ 3) | Dominio |\n|---|---:|---:|---:|---:|\n";
    for u in &c.unidades {
        let r = e.resumen(c, hoy, |p| p.unidad == u.id);
        s += &format!(
            "| {} · {} | {} | {} | {} | {} % |\n",
            u.id,
            u.titulo,
            r.total,
            r.vistas,
            r.dominadas,
            (r.dominio() * 100.0).round()
        );
    }
    let dias: Vec<&String> = e.por_dia.keys().collect();
    s += &format!("\nDías con repaso: **{}**", dias.len());
    if let Some(u) = dias.last() {
        s += &format!(" (último: {u}) · racha actual: **{}**", e.racha(hoy));
    }
    s += "\n\n## Más falladas\n\n";
    let mut falladas: Vec<(u32, &crate::Pregunta)> = c
        .preguntas
        .iter()
        .filter_map(|p| e.tarjeta(&p.id).filter(|t| t.fallos > 0).map(|t| (t.fallos, p)))
        .collect();
    falladas.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.pregunta.cmp(&b.1.pregunta)));
    if falladas.is_empty() {
        s += "- Todavía nada.\n";
    }
    for (n, p) in falladas.iter().take(15) {
        let enlace = if p.seccion.is_empty() { p.nota.clone() } else { format!("{}#{}", p.nota, p.seccion) };
        s += &format!("- **{n}×** {} → [[{enlace}]]\n", p.pregunta.replace('\n', " "));
    }
    if !e.simulacros.is_empty() {
        s += "\n## Últimos simulacros\n\n| Fecha | Asignaturas | Nota |\n|---|---|---:|\n";
        for x in e.simulacros.iter().rev().take(8) {
            let nota = if x.n == 0 { 0.0 } else { 10.0 * x.aciertos as f64 / x.n as f64 };
            s += &format!("| {} | {} | {:.1} |\n", &x.t[..x.t.len().min(10)], x.unidades.join(", "), nota);
        }
    }
    s += &format!(
        "\n> [!note] Qué significa «dominada»\n> Acertada varias veces seguidas en días distintos (caja {CAJA_DOMINADA} de 5 o más). Es evidencia de repaso, no de examen.\n"
    );
    s
}
