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
        let a = Analisis { asignaturas: 4, de_clase: 4, preguntas: 369, preguntas_test: 150, conceptos: 180, relaciones: 308, plazo_mas_cercano: Some(7), ..Default::default() };
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
}
