//! Progreso de repaso como **registro de eventos por dispositivo**.
//!
//! Cada dispositivo (portátil, PC de casa, iPhone) escribe solo su propio archivo
//! `progreso/<dispositivo>.json` en el repo privado. Así nunca hay conflictos al sincronizar:
//! el estado de cada tarjeta se reconstruye reproduciendo los eventos de todos los archivos
//! en orden de tiempo (sistema Leitner de 5 cajas).

use crate::modelo::Contenido;
use chrono::{Duration, NaiveDate};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

/// Días hasta el siguiente repaso según la caja (0 = nueva).
pub const INTERVALO: [i64; 6] = [0, 1, 2, 4, 8, 16];
/// Caja a partir de la cual una tarjeta cuenta como «dominada».
pub const CAJA_DOMINADA: u8 = 3;

/// Una respuesta: `r` = 1 fallé · 2 dudé · 3 lo sabía.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Evento {
    /// Hora local `AAAA-MM-DDTHH:MM:SS`.
    pub t: String,
    /// id de la pregunta.
    pub p: String,
    pub r: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Simulacro {
    pub t: String,
    pub unidades: Vec<String>,
    pub n: u32,
    pub aciertos: u32,
    pub segundos: u32,
}

/// Lo que guarda un dispositivo.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Registro {
    pub dispositivo: String,
    #[serde(default)]
    pub eventos: Vec<Evento>,
    #[serde(default)]
    pub simulacros: Vec<Simulacro>,
}

impl Registro {
    pub fn nuevo(dispositivo: &str) -> Self {
        Self { dispositivo: dispositivo.into(), ..Default::default() }
    }
    pub fn anotar(&mut self, t: &str, pregunta: &str, r: u8) {
        self.eventos.push(Evento { t: t.into(), p: pregunta.into(), r: r.clamp(1, 3) });
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Tarjeta {
    pub caja: u8,
    /// Próximo repaso (`AAAA-MM-DD`).
    pub proxima: String,
    pub aciertos: u32,
    pub fallos: u32,
    pub ultima: String,
}

/// Estado fusionado de todos los dispositivos.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Estado {
    pub tarjetas: HashMap<String, Tarjeta>,
    /// Respuestas por día (`AAAA-MM-DD` → n).
    pub por_dia: BTreeMap<String, u32>,
    pub simulacros: Vec<Simulacro>,
}

pub fn fecha(iso: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(iso.get(..10)?, "%Y-%m-%d").ok()
}

pub fn sumar_dias(iso: &str, n: i64) -> String {
    fecha(iso).map(|d| (d + Duration::days(n)).format("%Y-%m-%d").to_string()).unwrap_or_default()
}

/// Días de `hoy` a `iso` (negativo si ya pasó).
pub fn dias_hasta(hoy: &str, iso: &str) -> Option<i64> {
    Some((fecha(iso)? - fecha(hoy)?).num_days())
}

impl Estado {
    /// Reproduce los eventos de todos los registros en orden. Los eventos idénticos
    /// (mismo instante, pregunta y nota) se cuentan una vez: así da igual fusionar dos veces.
    pub fn desde(registros: &[Registro]) -> Self {
        let mut eventos: Vec<&Evento> = registros.iter().flat_map(|r| r.eventos.iter()).collect();
        eventos.sort();
        eventos.dedup();
        let mut e = Estado::default();
        for ev in eventos {
            e.aplicar(ev);
        }
        let mut sims: Vec<Simulacro> = registros.iter().flat_map(|r| r.simulacros.iter().cloned()).collect();
        sims.sort_by(|a, b| a.t.cmp(&b.t));
        sims.dedup();
        e.simulacros = sims;
        e
    }

    fn aplicar(&mut self, ev: &Evento) {
        let dia = ev.t.get(..10).unwrap_or_default().to_string();
        let t = self.tarjetas.entry(ev.p.clone()).or_default();
        match ev.r {
            1 => {
                t.caja = 1;
                t.proxima = dia.clone();
                t.fallos += 1;
            }
            2 => {
                t.caja = t.caja.max(1);
                t.proxima = sumar_dias(&dia, 1);
            }
            _ => {
                t.caja = (t.caja + 1).min(5);
                t.proxima = sumar_dias(&dia, INTERVALO[t.caja as usize]);
                t.aciertos += 1;
            }
        }
        t.ultima = ev.t.clone();
        *self.por_dia.entry(dia).or_default() += 1;
    }

    pub fn tarjeta(&self, id: &str) -> Option<&Tarjeta> {
        self.tarjetas.get(id)
    }
    pub fn pendiente(&self, id: &str, hoy: &str) -> bool {
        self.tarjetas.get(id).is_some_and(|t| t.proxima.as_str() <= hoy)
    }
    pub fn dominada(&self, id: &str) -> bool {
        self.tarjetas.get(id).is_some_and(|t| t.caja >= CAJA_DOMINADA)
    }

    /// Días seguidos con repaso hasta hoy (si hoy aún no hay, cuenta desde ayer).
    pub fn racha(&self, hoy: &str) -> u32 {
        let mut d = hoy.to_string();
        if !self.por_dia.contains_key(&d) {
            d = sumar_dias(&d, -1);
        }
        let mut n = 0;
        while self.por_dia.contains_key(&d) {
            n += 1;
            d = sumar_dias(&d, -1);
        }
        n
    }

    pub fn resumen(&self, contenido: &Contenido, hoy: &str, filtro: impl Fn(&crate::Pregunta) -> bool) -> Resumen {
        let mut r = Resumen::default();
        for p in contenido.preguntas.iter().filter(|p| filtro(p)) {
            r.total += 1;
            match self.tarjetas.get(&p.id) {
                None => r.nuevas += 1,
                Some(t) => {
                    r.vistas += 1;
                    if t.caja >= CAJA_DOMINADA {
                        r.dominadas += 1;
                    }
                    if t.proxima.as_str() <= hoy {
                        r.pendientes += 1;
                    }
                    if t.fallos > 0 {
                        r.falladas += 1;
                    }
                }
            }
        }
        r
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Resumen {
    pub total: u32,
    pub vistas: u32,
    pub nuevas: u32,
    pub dominadas: u32,
    pub pendientes: u32,
    pub falladas: u32,
}

impl Resumen {
    pub fn dominio(&self) -> f64 {
        if self.total == 0 { 0.0 } else { self.dominadas as f64 / self.total as f64 }
    }
}

/// Lo que lee el widget del escritorio (`~/.local/share/zerostudy/widget.json`).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EstadoWidget {
    pub pendientes_hoy: u32,
    pub racha: u32,
    pub dominio: f64,
    pub proximo_plazo: Option<PlazoWidget>,
    pub actualizado: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PlazoWidget {
    pub titulo: String,
    pub unidad: String,
    pub dias: i64,
}

impl Estado {
    pub fn widget(&self, c: &Contenido, hoy: &str, ahora: &str) -> EstadoWidget {
        let r = self.resumen(c, hoy, |_| true);
        let proximo = c
            .plazos
            .iter()
            .filter(|p| p.estado != "entregado")
            .filter_map(|p| Some((dias_hasta(hoy, p.fecha.as_deref()?)?, p)))
            .filter(|(d, _)| *d >= 0)
            .min_by_key(|(d, _)| *d)
            .map(|(dias, p)| PlazoWidget { titulo: p.que.clone(), unidad: p.unidad.clone(), dias });
        EstadoWidget {
            pendientes_hoy: r.pendientes,
            racha: self.racha(hoy),
            dominio: (r.dominio() * 100.0).round() / 100.0,
            proximo_plazo: proximo,
            actualizado: ahora.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reg(d: &str, evs: &[(&str, &str, u8)]) -> Registro {
        let mut r = Registro::nuevo(d);
        for (t, p, n) in evs {
            r.anotar(t, p, *n);
        }
        r
    }

    #[test]
    fn leitner_y_fusion() {
        let portatil = reg("portatil", &[("2026-11-17T10:00:00", "q1", 3), ("2026-11-17T10:01:00", "q2", 1)]);
        let iphone = reg("iphone", &[("2026-11-18T08:00:00", "q1", 3), ("2026-11-18T08:01:00", "q2", 3)]);
        let e = Estado::desde(&[portatil.clone(), iphone.clone()]);
        let q1 = e.tarjeta("q1").unwrap();
        assert_eq!((q1.caja, q1.proxima.as_str()), (2, "2026-11-20"));
        let q2 = e.tarjeta("q2").unwrap();
        assert_eq!((q2.caja, q2.fallos, q2.proxima.as_str()), (2, 1, "2026-11-20"));
        // Fusionar dos veces lo mismo no cambia nada; el orden de los registros tampoco.
        assert_eq!(e, Estado::desde(&[iphone, portatil.clone(), portatil]));
        assert_eq!(e.racha("2026-11-18"), 2);
        assert_eq!(e.racha("2026-11-19"), 2);
        assert_eq!(e.racha("2026-11-20"), 0);
    }

    #[test]
    fn dudar_no_sube_de_caja() {
        let e = Estado::desde(&[reg("a", &[("2026-11-17T10:00:00", "q", 2)])]);
        let t = e.tarjeta("q").unwrap();
        assert_eq!((t.caja, t.proxima.as_str()), (1, "2026-11-18"));
    }
}
