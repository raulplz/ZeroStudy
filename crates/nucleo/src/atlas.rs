//! Disposición del atlas de conceptos: simulación de fuerzas determinista (sin azar),
//! calculada una vez al generar el contenido para que la web solo tenga que dibujar.
//! Imita lo que hacía d3-force en Frost Study: grupos por asignatura en una rejilla,
//! enlaces que atraen, nodos que se repelen y colisiones.

use crate::modelo::Conceptos;
use std::collections::HashMap;

pub const SEP_X: f64 = 330.0;
pub const SEP_Y: f64 = 250.0;

/// Centro del grupo `i` de `n` (rejilla casi cuadrada centrada en el origen).
pub fn centro(i: usize, n: usize) -> (f64, f64) {
    let cols = (n as f64).sqrt().ceil().max(1.0) as usize;
    let filas = n.div_ceil(cols).max(1);
    let (c, f) = (i % cols, i / cols);
    let x = (c as f64 - (cols as f64 - 1.0) / 2.0) * 2.0 * SEP_X;
    let y = (f as f64 - (filas as f64 - 1.0) / 2.0) * 2.0 * SEP_Y;
    (x, y)
}

/// Grado (número de relaciones) de cada concepto.
pub fn grados(c: &Conceptos) -> HashMap<&str, usize> {
    let mut g: HashMap<&str, usize> = HashMap::new();
    for a in &c.aristas {
        *g.entry(a.de.as_str()).or_default() += 1;
        *g.entry(a.a.as_str()).or_default() += 1;
    }
    g
}

pub fn radio(importancia: u8, grado: usize) -> f64 {
    4.0 + importancia as f64 * 2.6 + (grado as f64 * 0.5).min(6.0)
}

/// Rellena `x`/`y` de cada concepto. `unidades` fija el orden de los grupos.
pub fn disponer(c: &mut Conceptos, unidades: &[String]) {
    let n = c.nodos.len();
    if n == 0 {
        return;
    }
    let idx: HashMap<String, usize> = c.nodos.iter().enumerate().map(|(i, x)| (x.id.clone(), i)).collect();
    let grado = grados(c);
    let r: Vec<f64> = c.nodos.iter().map(|x| radio(x.importancia, *grado.get(x.id.as_str()).unwrap_or(&0))).collect();
    let centros: Vec<(f64, f64)> = c
        .nodos
        .iter()
        .map(|x| centro(unidades.iter().position(|u| *u == x.unidad).unwrap_or(0), unidades.len().max(1)))
        .collect();
    let mut pos: Vec<(f64, f64)> =
        (0..n).map(|i| (centros[i].0 + (i as f64).cos() * 60.0, centros[i].1 + (i as f64).sin() * 60.0)).collect();
    let mut vel = vec![(0.0f64, 0.0f64); n];
    let enlaces: Vec<(usize, usize, f64, f64)> = c
        .aristas
        .iter()
        .filter_map(|a| {
            let (s, t) = (*idx.get(&a.de)?, *idx.get(&a.a)?);
            Some(if a.cruzada { (s, t, 260.0, 0.015) } else { (s, t, 46.0, 0.45) })
        })
        .collect();

    let mut alfa = 1.0f64;
    let decaimiento = 1.0 - 0.001f64.powf(1.0 / 300.0);
    for _ in 0..320 {
        alfa += (0.0 - alfa) * decaimiento;
        // enlaces
        for &(s, t, dist, k) in &enlaces {
            let (dx, dy) = (pos[t].0 + vel[t].0 - pos[s].0 - vel[s].0, pos[t].1 + vel[t].1 - pos[s].1 - vel[s].1);
            let l = (dx * dx + dy * dy).sqrt().max(1e-6);
            let f = (l - dist) / l * alfa * k;
            vel[t].0 -= dx * f * 0.5;
            vel[t].1 -= dy * f * 0.5;
            vel[s].0 += dx * f * 0.5;
            vel[s].1 += dy * f * 0.5;
        }
        // repulsión (carga -120, alcance 320)
        for i in 0..n {
            for j in (i + 1)..n {
                let (dx, dy) = (pos[j].0 - pos[i].0, pos[j].1 - pos[i].1);
                let d2 = (dx * dx + dy * dy).max(1.0);
                if d2 > 320.0 * 320.0 {
                    continue;
                }
                let f = -120.0 * alfa / d2;
                vel[i].0 += dx * f;
                vel[i].1 += dy * f;
                vel[j].0 -= dx * f;
                vel[j].1 -= dy * f;
            }
        }
        // atracción a su grupo
        for i in 0..n {
            vel[i].0 += (centros[i].0 - pos[i].0) * 0.12 * alfa;
            vel[i].1 += (centros[i].1 - pos[i].1) * 0.12 * alfa;
        }
        for i in 0..n {
            vel[i].0 *= 0.6;
            vel[i].1 *= 0.6;
            pos[i].0 += vel[i].0;
            pos[i].1 += vel[i].1;
        }
        // colisiones
        for i in 0..n {
            for j in (i + 1)..n {
                let (dx, dy) = (pos[j].0 - pos[i].0, pos[j].1 - pos[i].1);
                let d = (dx * dx + dy * dy).sqrt().max(1e-6);
                let min = r[i] + r[j] + 8.0;
                if d < min {
                    let m = (min - d) / d * 0.5;
                    pos[i].0 -= dx * m;
                    pos[i].1 -= dy * m;
                    pos[j].0 += dx * m;
                    pos[j].1 += dy * m;
                }
            }
        }
    }
    for (i, x) in c.nodos.iter_mut().enumerate() {
        x.x = (pos[i].0 * 10.0).round() / 10.0;
        x.y = (pos[i].1 * 10.0).round() / 10.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modelo::{Arista, Concepto};

    #[test]
    fn grupos_separados_y_determinista() {
        let mut c = Conceptos::default();
        for u in ["a", "b"] {
            for k in 0..10 {
                c.nodos.push(Concepto { id: format!("{u}{k}"), unidad: u.into(), importancia: 1, ..Default::default() });
            }
        }
        c.aristas.push(Arista { de: "a0".into(), a: "a1".into(), tipo: "usa".into(), ..Default::default() });
        let unidades = vec!["a".to_string(), "b".to_string()];
        let mut otra = c.clone();
        disponer(&mut c, &unidades);
        disponer(&mut otra, &unidades);
        assert_eq!(c, otra);
        let media = |u: &str| c.nodos.iter().filter(|x| x.unidad == u).map(|x| x.x).sum::<f64>() / 10.0;
        assert!(media("a") < 0.0 && media("b") > 0.0);
    }
}
