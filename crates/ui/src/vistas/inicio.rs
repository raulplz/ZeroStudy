//! Asistente de primer uso: dónde están los apuntes, qué asignaturas son de clase y cuáles
//! propias, para qué se estudia y qué metodologías se activan.

use leptos::prelude::*;
use leptos::task::spawn_local;
use zerostudy_nucleo::config::{Asignatura, Config, COLORES};
use zerostudy_nucleo::metodo::{self, Objetivo};
use zerostudy_nucleo::Contenido;

use crate::app::ctx;
use crate::plataforma;
use crate::util::hoy;
use crate::vistas::metodos::Metodos;

/// Nombre de dispositivo apto para un nombre de archivo (`progreso/<nombre>.json`).
pub fn limpiar_dispositivo(s: &str) -> String {
    let v: String = s.trim().to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect();
    let v = v.trim_matches('-').to_string();
    if v.is_empty() {
        "portatil".into()
    } else {
        v
    }
}

/// `usuario/repo` a partir de lo que se pegue (admite la URL completa).
pub fn limpiar_repo(s: &str) -> String {
    let s = s.trim().trim_end_matches('/').trim_end_matches(".git");
    let s = s.strip_prefix("https://github.com/").or_else(|| s.strip_prefix("github.com/")).unwrap_or(s);
    let ok = s.split('/').count() == 2 && s.chars().all(|c| c.is_ascii_alphanumeric() || "-_./".contains(c));
    if ok {
        s.to_string()
    } else {
        String::new()
    }
}

pub fn objetivo_texto(o: Objetivo) -> (&'static str, &'static str) {
    match o {
        Objetivo::Examenes => ("Exámenes", "Retener lo de clase hasta las fechas de evaluación."),
        Objetivo::Proyectos => ("Proyectos", "Entender y poder construir y defender lo que haces."),
        Objetivo::Curiosidad => ("Curiosidad", "Aprender por gusto, sin fechas, a tu ritmo."),
    }
}

/// Selector de objetivo (reutilizado en Ajustes).
pub fn selector_objetivo(actual: Signal<Objetivo>, elegir: Callback<Objetivo>) -> impl IntoView {
    view! {
        <div class="objetivos">
            {[Objetivo::Examenes, Objetivo::Proyectos, Objetivo::Curiosidad].into_iter().map(|o| {
                let (t, d) = objetivo_texto(o);
                view! {
                    <button class="objetivo" class:on=move || actual.get() == o on:click=move |_| elegir.run(o)>
                        <b>{t}</b><span>{d}</span>
                    </button>
                }
            }).collect_view()}
        </div>
    }
}

#[component]
pub fn Inicio() -> impl IntoView {
    let c = ctx();
    let web = !plataforma::escritorio();
    let mut inicial = c.config.get_untracked();
    if web && inicial.dispositivo == "portatil" {
        inicial.dispositivo = "movil".into();
    }
    let sug = c.sugerencia.get_untracked();
    if !web && inicial.boveda.is_empty() {
        if let Some(p) = c.plantilla.get_untracked() {
            inicial = Config { dispositivo: inicial.dispositivo, ..p };
        } else if !sug.is_empty() {
            inicial = Config { boveda: sug, ..inicial };
        }
    }
    let cfg = RwSignal::new(inicial);
    let token = RwSignal::new(plataforma::token());
    let paso = RwSignal::new(1u8);
    let ocupado = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let leido = RwSignal::new(None::<Contenido>);

    // El análisis tiene en cuenta lo que se marque como «clase» o «propio».
    let analisis = Memo::new(move |_| {
        let Some(mut x) = leido.get() else { return metodo::Analisis::default() };
        cfg.with(|k| {
            for u in x.unidades.iter_mut() {
                if let Some(a) = k.asignaturas.iter().find(|a| a.id == u.id) {
                    u.tipo = a.tipo.clone();
                }
            }
        });
        metodo::analizar(&x, &hoy())
    });
    let objetivo = Signal::derive(move || cfg.with(|k| k.objetivo));
    let activos = Signal::derive(move || cfg.with(|k| k.metodos.clone()));

    let analizar = move |_| {
        error.set(None);
        let mut k = cfg.get_untracked();
        k.dispositivo = limpiar_dispositivo(&k.dispositivo);
        k.sync.repo = limpiar_repo(&k.sync.repo);
        if web {
            if k.sync.repo.is_empty() || token.get_untracked().trim().is_empty() {
                error.set(Some("Escribe el repo como usuario/repo y pega el token.".into()));
                return;
            }
            plataforma::guardar_token(&token.get_untracked());
        } else if k.boveda.trim().is_empty() {
            error.set(Some("Indica la carpeta de apuntes.".into()));
            return;
        }
        ocupado.set(true);
        spawn_local(async move {
            match plataforma::analizar(&k).await {
                Ok(x) => {
                    if k.asignaturas.is_empty() {
                        k.asignaturas = x
                            .unidades
                            .iter()
                            .enumerate()
                            .map(|(i, u)| Asignatura {
                                id: u.id.clone(),
                                carpeta: u.carpeta.clone(),
                                titulo: u.titulo.clone(),
                                corto: u.corto.clone(),
                                color: if u.color.is_empty() { COLORES[i % COLORES.len()].into() } else { u.color.clone() },
                                tipo: u.tipo.clone(),
                            })
                            .collect();
                    }
                    cfg.set(k);
                    leido.set(Some(x));
                    paso.set(2);
                }
                Err(e) => error.set(Some(e)),
            }
            ocupado.set(false);
        });
    };

    let a_metodos = move |_| {
        let recs = metodo::recomendar(&analisis.get_untracked(), cfg.with_untracked(|k| k.objetivo));
        cfg.update(|k| k.metodos = metodo::por_defecto(&recs));
        paso.set(3);
    };

    let empezar = move |_| {
        let mut k = cfg.get_untracked();
        k.configurado = true;
        ocupado.set(true);
        spawn_local(async move {
            match plataforma::guardar_config(&k).await {
                Ok(()) => c.recargar(),
                Err(e) => {
                    error.set(Some(e));
                    ocupado.set(false);
                }
            }
        });
    };

    let paso1 = move || {
        view! {
            <h2>"Dónde están tus apuntes"</h2>
            {if web {
                view! {
                    <p class="tenue">"Esta versión web lee y guarda todo en un repo privado de GitHub. Crea un token fino con permiso de contenido solo sobre ese repo."</p>
                    <label class="campo"><span class="mono">"REPO DE DATOS"</span>
                        <input placeholder="usuario/repo" prop:value=move || cfg.with(|k| k.sync.repo.clone())
                            on:input=move |e| cfg.update(|k| k.sync.repo = event_target_value(&e))/></label>
                    <label class="campo"><span class="mono">"TOKEN DE GITHUB"</span>
                        <input type="password" autocomplete="off" placeholder="github_pat_…" prop:value=move || token.get()
                            on:input=move |e| token.set(event_target_value(&e))/></label>
                }.into_any()
            } else {
                view! {
                    <p class="tenue">"Una carpeta con tus notas en Markdown (una bóveda de Obsidian sirve). Cada subcarpeta será una asignatura o tema. ZeroStudy solo escribe en ella la nota de progreso."</p>
                    <label class="campo"><span class="mono">"CARPETA DE APUNTES"</span>
                        <input placeholder="/ruta/a/tus/apuntes" prop:value=move || cfg.with(|k| k.boveda.clone())
                            on:input=move |e| cfg.update(|k| k.boveda = event_target_value(&e))/></label>
                    <label class="campo"><span class="mono">"REPO DE SINCRONIZACIÓN (OPCIONAL)"</span>
                        <input placeholder="usuario/repo-privado" prop:value=move || cfg.with(|k| k.sync.repo.clone())
                            on:input=move |e| cfg.update(|k| k.sync.repo = event_target_value(&e))/></label>
                }.into_any()
            }}
            <label class="campo"><span class="mono">"NOMBRE DE ESTE DISPOSITIVO"</span>
                <input placeholder="portatil" prop:value=move || cfg.with(|k| k.dispositivo.clone())
                    on:input=move |e| cfg.update(|k| k.dispositivo = event_target_value(&e))/></label>
            <div class="acciones">
                <button class="boton" disabled=move || ocupado.get() on:click=analizar>{move || if ocupado.get() { "Analizando…" } else { "Analizar apuntes →" }}</button>
            </div>
        }
    };

    let paso2 = move || {
        let a = analisis.get();
        let filas = cfg.with(|k| k.asignaturas.clone()).into_iter().enumerate().map(|(i, s)| {
            let n_preg = leido.with(|x| x.as_ref().map(|x| x.preguntas.iter().filter(|p| p.unidad == s.id).count()).unwrap_or(0));
            let ajustar = move |tipo: &'static str| cfg.update(|k| if let Some(x) = k.asignaturas.get_mut(i) { x.tipo = tipo.into() });
            view! {
                <div class="fila-asig" style=format!("--c:{}", s.color)>
                    <i></i>
                    <div><b>{s.titulo.clone()}</b><div class="mono tenue">{format!("{} · {n_preg} PREGUNTAS", s.carpeta)}</div></div>
                    <div class="chips">
                        <span class="chip" role="button" class:on=move || cfg.with(|k| k.asignaturas.get(i).is_some_and(|x| x.tipo != "propio")) on:click=move |_| ajustar("clase")>"Clase"</span>
                        <span class="chip" role="button" class:on=move || cfg.with(|k| k.asignaturas.get(i).is_some_and(|x| x.tipo == "propio")) on:click=move |_| ajustar("propio")>"Propio"</span>
                    </div>
                </div>
            }
        }).collect_view();
        view! {
            <h2>"Lo que hemos encontrado"</h2>
            <div class="rejilla-4 resumen-analisis">
                <div><b>{a.asignaturas}</b>"asignaturas"</div>
                <div><b>{a.notas}</b>"notas"</div>
                <div><b>{a.preguntas}</b>{format!("preguntas ({} de test)", a.preguntas_test)}</div>
                <div><b>{a.conceptos}</b>{format!("conceptos · {} relaciones", a.relaciones)}</div>
            </div>
            <h3 class="sub">"Asignaturas"</h3>
            <p class="tenue">"«Clase» tiene profesor y plazos; «propio» es aprendizaje por tu cuenta."</p>
            {filas}
            <h3 class="sub">"¿Para qué estudias?"</h3>
            {selector_objetivo(objetivo, Callback::new(move |o| cfg.update(|k| k.objetivo = o)))}
            <div class="acciones">
                <button class="boton fantasma" on:click=move |_| paso.set(1)>"← Atrás"</button>
                <button class="boton" on:click=a_metodos>"Ver metodologías →"</button>
            </div>
        }
    };

    let paso3 = move || {
        view! {
            <h2>"Cómo quieres estudiar"</h2>
            <p class="tenue">"Ordenadas según tu material y tu objetivo; las tres primeras vienen activadas. Puedes cambiarlas cuando quieras en Ajustes."</p>
            <Metodos analisis=analisis objetivo=objetivo activos=activos
                alternar=Callback::new(move |id: String| cfg.update(|k| {
                    if let Some(p) = k.metodos.iter().position(|m| *m == id) { k.metodos.remove(p); } else { k.metodos.push(id); }
                }))/>
            <div class="acciones">
                <button class="boton fantasma" on:click=move |_| paso.set(2)>"← Atrás"</button>
                <button class="boton" disabled=move || ocupado.get() on:click=empezar>"Empezar ▶"</button>
            </div>
        }
    };

    view! {
        <main class="vista inicio">
            <header class="cabecera">
                <div><div class="pre">"SYS://PRIMER USO"</div><h1>"Bienvenido a ZeroStudy"</h1></div>
                <div class="lado">{move || format!("PASO {} DE 3", paso.get())}</div>
            </header>
            <div class="progreso-barra"><i style=move || format!("width:{}%", paso.get() as u32 * 100 / 3)></i></div>
            <section class="panel asistente">
                {move || match paso.get() { 1 => paso1().into_any(), 2 => paso2().into_any(), _ => paso3().into_any() }}
                {move || error.get().map(|e| view! { <div class="error-caja">"⚠ " {e}</div> })}
            </section>
        </main>
    }
}
