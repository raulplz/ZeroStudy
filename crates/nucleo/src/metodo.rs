//! Metodologías de estudio: catálogo, recomendación según el material y el objetivo,
//! y qué cambia en la app al activar cada una.
//!
//! La evidencia se resume con prudencia. Fuente principal: Dunlosky et al. (2013),
//! «Improving Students' Learning With Effective Learning Techniques», *Psychological
//! Science in the Public Interest* 14(1), que clasifica la práctica de recuperación
//! (*practice testing*) y la práctica distribuida como de utilidad alta, y la
//! autoexplicación y la práctica intercalada como de utilidad moderada.

use crate::modelo::Contenido;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Evidencia {
    Alta,
    Moderada,
    Organizativa,
}

impl Evidencia {
    pub fn texto(self) -> &'static str {
        match self {
            Evidencia::Alta => "Evidencia alta",
            Evidencia::Moderada => "Evidencia moderada",
            Evidencia::Organizativa => "Gestión del tiempo, no técnica de memoria",
        }
    }
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Metodo {
    pub id: &'static str,
    pub nombre: &'static str,
    pub icono: &'static str,
    pub resumen: &'static str,
    /// Qué hace ZeroStudy cuando está activa.
    pub en_la_app: &'static str,
    pub ideal_para: &'static str,
    pub evidencia: Evidencia,
}

pub const CATALOGO: &[Metodo] = &[
    Metodo {
        id: "espaciada",
        nombre: "Repetición espaciada",
        icono: "◷",
        resumen: "Repasar justo antes de olvidar: cada acierto aleja la siguiente vez (1, 2, 4, 8 y 16 días).",
        en_la_app: "Cola diaria de tarjetas pendientes con cajas Leitner.",
        ideal_para: "Mucho contenido que hay que retener semanas o meses.",
        evidencia: Evidencia::Alta,
    },
    Metodo {
        id: "recuperacion",
        nombre: "Práctica de recuperación",
        icono: "⏱",
        resumen: "Ponerse a prueba sin mirar los apuntes en lugar de releerlos.",
        en_la_app: "Simulacros cronometrados con nota y aviso de simulacro cuando un plazo está cerca.",
        ideal_para: "Exámenes tipo test y asignaturas con preguntas cerradas.",
        evidencia: Evidencia::Alta,
    },
    Metodo {
        id: "intercalado",
        nombre: "Práctica intercalada",
        icono: "⇄",
        resumen: "Mezclar temas o asignaturas en la misma sesión en vez de bloque a bloque.",
        en_la_app: "Las sesiones mezclan asignaturas por defecto y evitan dos preguntas seguidas del mismo tema.",
        ideal_para: "Varias asignaturas a la vez o temas parecidos que se confunden.",
        evidencia: Evidencia::Moderada,
    },
    Metodo {
        id: "feynman",
        nombre: "Técnica Feynman",
        icono: "✎",
        resumen: "Explicar el concepto con tus palabras, como a alguien que no sabe nada, y ver dónde fallas.",
        en_la_app: "En las preguntas abiertas escribes tu explicación antes de ver la respuesta y la comparas.",
        ideal_para: "Conceptos que hay que entender y defender, aprendizaje propio.",
        evidencia: Evidencia::Moderada,
    },
    Metodo {
        id: "mapas",
        nombre: "Mapas conceptuales",
        icono: "⌬",
        resumen: "Relacionar conceptos entre sí y con otras asignaturas en lugar de memorizarlos sueltos.",
        en_la_app: "El atlas pasa al panel y cada sesión termina proponiendo un concepto relacionado.",
        ideal_para: "Temarios con muchas relaciones y trabajos que cruzan asignaturas.",
        evidencia: Evidencia::Moderada,
    },
    Metodo {
        id: "practica",
        nombre: "Aprender haciendo",
        icono: "⌘",
        resumen: "Aprender con un mini proyecto o ejercicios reales y repasar solo lo que hizo falta.",
        en_la_app: "Prioriza notas con código y abre la nota en Obsidian para practicar sobre ella.",
        ideal_para: "Programación, herramientas y aprendizaje personal.",
        evidencia: Evidencia::Moderada,
    },
    Metodo {
        id: "pomodoro",
        nombre: "Pomodoro",
        icono: "◔",
        resumen: "Bloques de 25 minutos de foco con descansos de 5.",
        en_la_app: "Temporizador en la barra lateral durante las sesiones.",
        ideal_para: "Arrancar cuando cuesta concentrarse o hay poco tiempo.",
        evidencia: Evidencia::Organizativa,
    },
];

pub fn metodo(id: &str) -> Option<&'static Metodo> {
    CATALOGO.iter().find(|m| m.id == id)
}

/// Para qué estudia la persona (lo elige en el primer uso).
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Objetivo {
    #[default]
    Examenes,
    Proyectos,
    Curiosidad,
}

/// Rasgos del material importado que pesan en la recomendación.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Analisis {
    pub asignaturas: u32,
    pub de_clase: u32,
    pub propias: u32,
    pub notas: u32,
    pub preguntas: u32,
    pub preguntas_test: u32,
    pub conceptos: u32,
    pub relaciones: u32,
    pub bloques_codigo: u32,
    /// Días hasta el plazo pendiente más cercano.
    pub plazo_mas_cercano: Option<i64>,
}

pub fn analizar(c: &Contenido, hoy: &str) -> Analisis {
    let es_test = |p: &&crate::Pregunta| crate::opciones::es_test(&p.opciones, &p.respuesta);
    Analisis {
        asignaturas: c.unidades.len() as u32,
        de_clase: c.unidades.iter().filter(|u| u.tipo != "propio").count() as u32,
        propias: c.unidades.iter().filter(|u| u.tipo == "propio").count() as u32,
        notas: c.unidades.iter().map(|u| u.notas).sum(),
        preguntas: c.preguntas.len() as u32,
        preguntas_test: c.preguntas.iter().filter(es_test).count() as u32,
        conceptos: c.conceptos.nodos.len() as u32,
        relaciones: c.conceptos.aristas.len() as u32,
        bloques_codigo: c.unidades.iter().map(|u| u.bloques_codigo).sum(),
        plazo_mas_cercano: c
            .plazos
            .iter()
            .filter(|p| p.estado != "entregado")
            .filter_map(|p| crate::progreso::dias_hasta(hoy, p.fecha.as_deref()?))
            .filter(|d| *d >= 0)
            .min(),
    }
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Recomendacion {
    pub id: &'static str,
    pub puntos: u32,
    pub motivos: Vec<String>,
}

/// Ordena el catálogo por encaje con el material y el objetivo, con los motivos a la vista.
pub fn recomendar(a: &Analisis, objetivo: Objetivo) -> Vec<Recomendacion> {
    let mut v: Vec<Recomendacion> = CATALOGO
        .iter()
        .map(|m| {
            let mut puntos = 10u32;
            let mut motivos = Vec::new();
            let mut sumar = |p: u32, motivo: String| {
                puntos += p;
                motivos.push(motivo);
            };
            match m.id {
                "espaciada" => {
                    if a.preguntas >= 30 {
                        sumar(30, format!("Tienes {} preguntas: sin espaciar se olvidan antes de volver a verlas.", a.preguntas));
                    }
                    if objetivo == Objetivo::Examenes {
                        sumar(15, "Preparas exámenes: retener a varias semanas es justo lo que mide.".into());
                    }
                }
                "recuperacion" => {
                    if a.preguntas_test >= 10 {
                        sumar(25, format!("{} preguntas son de tipo test: se pueden hacer simulacros reales.", a.preguntas_test));
                    }
                    if let Some(d) = a.plazo_mas_cercano.filter(|d| *d <= 21) {
                        sumar(20, format!("Tu próximo plazo es en {d} días."));
                    }
                    if objetivo == Objetivo::Examenes {
                        sumar(10, "Ponerse a prueba es la técnica con más evidencia para exámenes.".into());
                    }
                }
                "intercalado" => {
                    if a.asignaturas >= 3 {
                        sumar(25, format!("Llevas {} asignaturas a la vez: mezclarlas ayuda a distinguirlas.", a.asignaturas));
                    } else if a.asignaturas == 2 {
                        sumar(10, "Dos materias: mezclarlas de vez en cuando ayuda a distinguirlas.".into());
                    }
                }
                "feynman" => {
                    if a.conceptos >= 20 {
                        sumar(15, format!("Hay {} conceptos que entender, no solo memorizar.", a.conceptos));
                    }
                    if a.propias > 0 || objetivo == Objetivo::Curiosidad {
                        sumar(20, "Aprendes por tu cuenta: explicarlo destapa lo que aún no entiendes.".into());
                    }
                    if a.preguntas > a.preguntas_test * 2 {
                        sumar(10, "La mayoría de tus preguntas son abiertas.".into());
                    }
                }
                "mapas" => {
                    if a.relaciones >= 40 {
                        sumar(20, format!("Tu atlas tiene {} relaciones entre conceptos.", a.relaciones));
                    }
                    if objetivo == Objetivo::Proyectos {
                        sumar(15, "Los proyectos cruzan temas: ver las conexiones ayuda a defenderlos.".into());
                    }
                }
                "practica" => {
                    if a.bloques_codigo >= 15 {
                        sumar(25, format!("Tus notas tienen {} bloques de código.", a.bloques_codigo));
                    }
                    if objetivo == Objetivo::Proyectos {
                        sumar(20, "Tu objetivo son proyectos.".into());
                    }
                    if a.propias > 0 {
                        sumar(10, "Tienes temas de aprendizaje propio.".into());
                    }
                }
                "pomodoro" => {
                    sumar(5, "Combina con cualquier otra para organizar el tiempo.".into());
                }
                _ => {}
            }
            if m.evidencia == Evidencia::Alta {
                puntos += 5;
            }
            Recomendacion { id: m.id, puntos, motivos }
        })
        .collect();
    v.sort_by(|x, y| y.puntos.cmp(&x.puntos).then(x.id.cmp(y.id)));
    v
}

/// Las metodologías que se activan por defecto: las tres primeras de la recomendación.
pub fn por_defecto(recs: &[Recomendacion]) -> Vec<String> {
    recs.iter().take(3).map(|r| r.id.to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn examenes_con_test_prioriza_recuperacion_y_espaciada() {
        let a = Analisis {
            asignaturas: 4,
            de_clase: 4,
            preguntas: 369,
            preguntas_test: 150,
            conceptos: 180,
            relaciones: 308,
            plazo_mas_cercano: Some(7),
            ..Default::default()
        };
        let r = recomendar(&a, Objetivo::Examenes);
        let top: Vec<&str> = r.iter().take(2).map(|x| x.id).collect();
        assert!(top.contains(&"recuperacion") && top.contains(&"espaciada"), "{top:?}");
        assert!(r.iter().all(|x| metodo(x.id).is_some()));
    }

    #[test]
    fn aprendizaje_propio_con_codigo_prioriza_practica() {
        let a = Analisis { asignaturas: 1, propias: 1, notas: 20, bloques_codigo: 60, ..Default::default() };
        assert_eq!(recomendar(&a, Objetivo::Proyectos)[0].id, "practica");
    }

    #[test]
    fn recomendar_es_determinista_ordenado_y_cubre_el_catalogo() {
        let a = Analisis { asignaturas: 3, preguntas: 50, ..Default::default() };
        let r = recomendar(&a, Objetivo::Curiosidad);
        assert_eq!(r, recomendar(&a, Objetivo::Curiosidad));
        assert_eq!(r.len(), CATALOGO.len());
        assert!(r.windows(2).all(|w| w[0].puntos >= w[1].puntos));
        let mut ids: Vec<_> = r.iter().map(|x| x.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), CATALOGO.len());
    }

    #[test]
    fn plazo_cercano_y_objetivo_cambian_la_puntuacion() {
        let base = Analisis { asignaturas: 1, ..Default::default() };
        let puntos = |a: &Analisis, o, id: &str| recomendar(a, o).into_iter().find(|r| r.id == id).unwrap().puntos;
        let cerca = Analisis { plazo_mas_cercano: Some(5), ..base.clone() };
        let lejos = Analisis { plazo_mas_cercano: Some(60), ..base.clone() };
        assert_eq!(puntos(&cerca, Objetivo::Curiosidad, "recuperacion"), puntos(&lejos, Objetivo::Curiosidad, "recuperacion") + 20);
        assert!(puntos(&base, Objetivo::Examenes, "espaciada") > puntos(&base, Objetivo::Curiosidad, "espaciada"));
        assert!(puntos(&base, Objetivo::Proyectos, "practica") > puntos(&base, Objetivo::Examenes, "practica"));
    }

    #[test]
    fn por_defecto_son_las_tres_primeras_y_hay_motivos() {
        let a = Analisis { asignaturas: 4, preguntas: 300, preguntas_test: 100, ..Default::default() };
        let r = recomendar(&a, Objetivo::Examenes);
        let d = por_defecto(&r);
        assert_eq!(d, r.iter().take(3).map(|x| x.id.to_string()).collect::<Vec<_>>());
        assert!(r[0].motivos.iter().all(|m| !m.is_empty()) && !r[0].motivos.is_empty());
        assert!(por_defecto(&[]).is_empty());
    }

    #[test]
    fn analizar_cuenta_tipos_tests_y_plazos() {
        use crate::modelo::{Plazo, Pregunta, Unidad};
        let u = |id: &str, tipo: &str, notas, codigo| Unidad { id: id.into(), tipo: tipo.into(), notas, bloques_codigo: codigo, ..Default::default() };
        let test = Pregunta { opciones: "a) x\nb) y".into(), respuesta: "**Respuesta: a)**".into(), ..Default::default() };
        let abierta = Pregunta { respuesta: "porque sí".into(), ..Default::default() };
        let plazo = |f: &str, e: &str| Plazo { fecha: Some(f.into()), estado: e.into(), ..Default::default() };
        let c = Contenido {
            unidades: vec![u("1", "clase", 5, 2), u("2", "propio", 7, 10)],
            preguntas: vec![test, abierta],
            plazos: vec![
                plazo("2026-11-30", "pendiente"),
                plazo("2026-11-20", "pendiente"),
                plazo("2026-11-19", "entregado"),
                plazo("2026-11-01", "pendiente"),
            ],
            ..Default::default()
        };
        let a = analizar(&c, "2026-11-18");
        assert_eq!((a.asignaturas, a.de_clase, a.propias, a.notas, a.bloques_codigo), (2, 1, 1, 12, 12));
        assert_eq!((a.preguntas, a.preguntas_test), (2, 1));
        assert_eq!(a.plazo_mas_cercano, Some(2));
        assert_eq!(analizar(&Contenido::default(), "2026-11-18"), Analisis::default());
    }
}
