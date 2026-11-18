//! Repaso espaciado, simulacros de test y técnica Feynman.

use leptos::prelude::*;
use std::time::Duration;
use web_sys::KeyboardEvent;
use zerostudy_nucleo::opciones::{self, Opcion};
use zerostudy_nucleo::progreso::INTERVALO;
use zerostudy_nucleo::{Pregunta, Simulacro};

use crate::app::{ctx, Ctx};
use crate::plataforma;
use crate::util::{self, barajar, coma, hoy, md, md_linea, ms};

#[derive(Clone, Debug, PartialEq)]
pub struct Cfg {
    /// Vacío = todas.
    pub unidades: Vec<String>,
    pub modo: &'static str,
    pub tipo: &'static str,
    pub n: usize,
}

impl Default for Cfg {
    fn default() -> Self {
        Self { unidades: Vec::new(), modo: "mezcla", tipo: "todas", n: 20 }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Sesion {
    pub titulo: String,
    pub cola: Vec<String>,
    pub i: usize,
    pub total: usize,
    pub resultados: Vec<(String, u8)>,
    pub revelada: bool,
    pub elegida: Option<char>,
    pub simulacro: bool,
    pub inicio: f64,
    pub fin: Option<f64>,
    pub respuestas: Vec<(String, char, bool)>,
    pub explicacion: String,
}

fn incluye(cfg: &Cfg, p: &Pregunta) -> bool {
    (cfg.unidades.is_empty() || cfg.unidades.contains(&p.unidad))
        && match cfg.tipo {
            "test" => p.test,
            "abiertas" => !p.test,
            _ => true,
        }
}

/// Reparte la cola alternando asignaturas (práctica intercalada).
fn intercalar(ids: Vec<String>, c: &Ctx) -> Vec<String> {
    let unidad = |id: &String| c.contenido.with_untracked(|x| x.as_ref().and_then(|x| x.pregunta(id)).map(|p| p.unidad.clone()).unwrap_or_default());
    let mut grupos: Vec<(String, std::collections::VecDeque<String>)> = Vec::new();
    for id in ids {
        let u = unidad(&id);
        match grupos.iter_mut().find(|(k, _)| *k == u) {
            Some((_, g)) => g.push_back(id),
            None => grupos.push((u, std::collections::VecDeque::from([id]))),
        }
    }
    let mut out = Vec::new();
    while grupos.iter().any(|(_, g)| !g.is_empty()) {
        for (_, g) in grupos.iter_mut() {
            if let Some(x) = g.pop_front() {
                out.push(x);
            }
        }
    }
    out
}

pub fn construir_cola(c: &Ctx, cfg: &Cfg) -> Vec<String> {
    let h = hoy();
    let ids = c.contenido.with_untracked(|x| {
        let Some(x) = x else { return Vec::new() };
        c.estado.with_untracked(|e| {
            let ps: Vec<&Pregunta> = x.preguntas.iter().filter(|p| incluye(cfg, p)).collect();
            let nuevas = || {
                let mut v: Vec<String> = ps.iter().filter(|p| e.tarjeta(&p.id).is_none()).map(|p| p.id.clone()).collect();
                barajar(&mut v);
                v
            };
            let mut pend: Vec<&&Pregunta> = ps.iter().filter(|p| e.pendiente(&p.id, &h)).collect();
            pend.sort_by_key(|p| e.tarjeta(&p.id).map(|t| t.caja).unwrap_or(0));
            let pend: Vec<String> = pend.iter().map(|p| p.id.clone()).collect();
            match cfg.modo {
                "hoy" => pend,
                "nuevas" => nuevas(),
                "mezcla" => {
                    let mut v = pend;
                    v.extend(nuevas().into_iter().take(cfg.n));
                    v
                }
                "falladas" => {
                    let mut v: Vec<&&Pregunta> = ps.iter().filter(|p| e.tarjeta(&p.id).is_some_and(|t| t.fallos > 0)).collect();
                    v.sort_by_key(|p| std::cmp::Reverse(e.tarjeta(&p.id).map(|t| t.fallos).unwrap_or(0)));
                    v.iter().map(|p| p.id.clone()).collect()
                }
                "simulacro" => {
                    let mut v: Vec<String> = x
                        .preguntas
                        .iter()
                        .filter(|p| (cfg.unidades.is_empty() || cfg.unidades.contains(&p.unidad)) && p.test && opciones::es_test(&p.opciones, &p.respuesta))
                        .map(|p| p.id.clone())
                        .collect();
                    barajar(&mut v);
                    v
                }
                _ => {
                    let mut v: Vec<String> = ps.iter().map(|p| p.id.clone()).collect();
                    barajar(&mut v);
                    v
                }
            }
        })
    });
    let tope = if cfg.modo == "simulacro" { cfg.n.min(40) } else { cfg.n };
    let mut ids: Vec<String> = ids.into_iter().take(tope).collect();
    if c.metodo("intercalado") && cfg.modo != "falladas" {
        ids = intercalar(ids, c);
    }
    ids
}

/// Empieza una sesión con la configuración actual o con una lista concreta de preguntas.
pub fn empezar(c: Ctx, extra: Option<(String, Vec<String>)>) {
    let cfg = c.cfg_repaso.get_untracked();
    let (titulo, cola, simulacro) = match extra {
        Some((t, ids)) => {
            let mut ids = ids;
            if c.metodo("intercalado") {
                ids = intercalar(ids, &c);
            }
            (t, ids, false)
        }
        None => (String::new(), construir_cola(&c, &cfg), cfg.modo == "simulacro"),
    };
    if cola.is_empty() {
        c.aviso(if cfg.modo == "hoy" { "No hay pendientes para hoy: prueba «Hoy + nuevas»" } else { "No hay preguntas con ese filtro" });
        return;
    }
    c.repaso.set(Some(Sesion {
        titulo,
        total: cola.len(),
        cola,
        i: 0,
        resultados: Vec::new(),
        revelada: false,
        elegida: None,
        simulacro,
        inicio: ms(),
        fin: None,
        respuestas: Vec::new(),
        explicacion: String::new(),
    }));
    util::ir("repaso");
}

fn pregunta(c: &Ctx, id: &str) -> Option<Pregunta> {
    c.contenido.with_untracked(|x| x.as_ref().and_then(|x| x.pregunta(id).cloned()))
}

fn responder(c: Ctx, nota: u8) {
    let Some(id) = c.repaso.with_untracked(|s| s.as_ref().map(|s| s.cola[s.i].clone())) else { return };
    c.calificar(&id, nota);
    c.repaso.update(|s| {
        let Some(s) = s else { return };
        s.resultados.push((id.clone(), nota));
        if nota == 1 && s.cola.iter().filter(|x| **x == id).count() < 3 {
            s.cola.push(id);
        }
        s.i += 1;
        s.revelada = false;
        s.elegida = None;
        s.explicacion.clear();
    });
}

fn responder_simulacro(c: Ctx, letra: char) {
    let Some(s) = c.repaso.get_untracked() else { return };
    let Some(p) = pregunta(&c, &s.cola[s.i]) else { return };
    let ok = opciones::letra_correcta(&p.respuesta) == Some(letra);
    if !ok {
        c.calificar(&p.id, 1);
    }
    c.repaso.update(|s| {
        let Some(s) = s else { return };
        s.respuestas.push((p.id.clone(), letra, ok));
        s.i += 1;
        if s.i >= s.cola.len() {
            s.fin = Some(ms());
        }
    });
    if let Some(s) = c.repaso.get_untracked().filter(|s| s.fin.is_some()) {
        let n = s.respuestas.len() as u32;
        c.registrar_simulacro(Simulacro {
            t: util::ahora(),
            unidades: c.cfg_repaso.with_untracked(|x| x.unidades.clone()),
            n,
            aciertos: s.respuestas.iter().filter(|r| r.2).count() as u32,
            segundos: ((s.fin.unwrap() - s.inicio) / 1000.0) as u32,
        });
    }
}

fn elegir(c: Ctx, letra: char) {
    c.repaso.update(|s| {
        if let Some(s) = s {
            s.elegida = Some(letra);
            s.revelada = true;
        }
    });
}

fn revelar(c: Ctx) {
    c.repaso.update(|s| {
        if let Some(s) = s {
            s.revelada = true;
        }
    });
}

pub fn tecla(c: Ctx, e: KeyboardEvent) {
    let Some(s) = c.repaso.get_untracked() else { return };
    if !c.ruta.with_untracked(|r| r.starts_with("repaso")) {
        return;
    }
    let k = e.key().to_lowercase();
    let en_texto = e
        .target()
        .and_then(|t| wasm_bindgen::JsCast::dyn_into::<web_sys::Element>(t).ok())
        .is_some_and(|el| matches!(el.tag_name().as_str(), "INPUT" | "TEXTAREA"));
    if en_texto {
        if k == "enter" && (e.ctrl_key() || e.meta_key()) {
            e.prevent_default();
            revelar(c);
        }
        if k == "escape" {
            if let Some(el) = document().active_element() {
                let _ = wasm_bindgen::JsCast::unchecked_into::<web_sys::HtmlElement>(el).blur();
            }
        }
        return;
    }
    if s.i >= s.cola.len() {
        if k == "enter" {
            c.repaso.set(None);
        }
        return;
    }
    if k == "escape" {
        c.repaso.set(None);
        return;
    }
    let Some(p) = pregunta(&c, &s.cola[s.i]) else { return };
    let ops = opciones::parsear(&p.opciones);
    let correcta = opciones::letra_correcta(&p.respuesta);
    let es_test = ops.len() >= 2 && correcta.is_some();
    let letra = (k.len() == 1).then(|| k.chars().next().unwrap()).filter(|l| ops.iter().any(|o| o.letra == *l));
    if s.simulacro {
        if let Some(l) = letra {
            responder_simulacro(c, l);
        }
        return;
    }
    if !s.revelada {
        if let (true, Some(l)) = (es_test, letra) {
            elegir(c, l);
        } else if !es_test && (k == " " || k == "enter") {
            e.prevent_default();
            revelar(c);
        }
        return;
    }
    let fallo = es_test && s.elegida != correcta;
    match k.as_str() {
        "enter" | " " | "1" if fallo => {
            e.prevent_default();
            responder(c, 1);
        }
        "1" if !es_test => responder(c, 1),
        "2" if !fallo => responder(c, 2),
        "3" | "enter" if !fallo => responder(c, 3),
        _ => {}
    }
}

#[component]
pub fn VistaRepaso() -> impl IntoView {
    let c = ctx();
    // ?auto=hoy|falladas desde el panel
    let auto = c.ruta.with_untracked(|r| util::param(r, "auto"));
    if let (Some(a), true) = (auto, c.repaso.with_untracked(Option::is_none)) {
        c.cfg_repaso.set(Cfg { modo: if a == "falladas" { "falladas" } else { "mezcla" }, ..Cfg::default() });
        util::ir("repaso");
        empezar(c, None);
    }
    if let Some(u) = c.ruta.with_untracked(|r| util::param(r, "u")) {
        c.cfg_repaso.update(|x| x.unidades = vec![u]);
    }
    view! {
        {move || match c.repaso.get() {
            None => view! { <Configurar/> }.into_any(),
            Some(s) if s.i >= s.cola.len() => if s.simulacro { view! { <FinSimulacro s=s/> }.into_any() } else { view! { <Fin s=s/> }.into_any() },
            Some(s) => view! { <TarjetaVista s=s/> }.into_any(),
        }}
    }
}

#[component]
fn Configurar() -> impl IntoView {
    let c = ctx();
    let unidades = c.contenido.with_untracked(|x| x.as_ref().map(|x| x.unidades.clone()).unwrap_or_default());
    let chips_u = unidades
        .into_iter()
        .map(|u| {
            let id = u.id.clone();
            let id2 = id.clone();
            view! {
                <span class="chip" style=format!("--c:{}", u.color)
                    class:on=move || c.cfg_repaso.with(|x| x.unidades.is_empty() || x.unidades.contains(&id))
                    on:click=move |_| c.cfg_repaso.update(|x| {
                        let todas: Vec<String> = c.contenido.with_untracked(|k| k.as_ref().map(|k| k.unidades.iter().map(|u| u.id.clone()).collect()).unwrap_or_default());
                        if x.unidades.is_empty() { x.unidades = todas.into_iter().filter(|u| *u != id2).collect(); }
                        else if x.unidades.contains(&id2) { if x.unidades.len() > 1 { x.unidades.retain(|u| *u != id2); } }
                        else { x.unidades.push(id2.clone()); }
                    })>
                    <i></i>{format!("{} · {}", u.id, u.corto)}
                </span>
            }
        })
        .collect_view();
    let resumen = move || {
        let cfg = c.cfg_repaso.get();
        c.contenido.with(|x| x.as_ref().map(|x| c.estado.with(|e| e.resumen(x, &hoy(), |p| incluye(&cfg, p)))).unwrap_or_default())
    };
    let modo = move |id: &'static str, texto: Box<dyn Fn() -> String + Send + Sync>| {
        view! { <span class="chip" class:on=move || c.cfg_repaso.with(|x| x.modo == id) on:click=move |_| c.cfg_repaso.update(|x| x.modo = id)>{texto}</span> }
    };
    let tipo = move |id: &'static str, texto: &'static str| {
        view! { <span class="chip" class:on=move || c.cfg_repaso.with(|x| x.tipo == id) on:click=move |_| c.cfg_repaso.update(|x| x.tipo = id)>{texto}</span> }
    };
    let tam = move |n: usize| {
        view! { <span class="chip" class:on=move || c.cfg_repaso.with(|x| x.n == n) on:click=move |_| c.cfg_repaso.update(|x| x.n = n)>{if n >= 999 { "Sin límite".to_string() } else { n.to_string() }}</span> }
    };
    let feynman = c.metodo("feynman");
    let intercalado = c.metodo("intercalado");
    view! {
        <header class="cabecera"><div><div class="pre">"SYS://REPASO · REPETICIÓN ESPACIADA"</div><h1>"Repaso"</h1></div>
            <div class="lado">{format!("CAJAS 1–5 · INTERVALOS {} DÍAS", INTERVALO[1..].iter().map(|d| d.to_string()).collect::<Vec<_>>().join(" / "))}</div></header>
        <div class="panel config">
            <div class="fila"><div class="k">"ASIGNATURAS"</div><div class="chips">{chips_u}</div></div>
            <div class="fila"><div class="k">"MODO"</div><div class="chips">
                {modo("hoy", Box::new(move || format!("Pendientes hoy · {}", resumen().pendientes)))}
                {modo("mezcla", Box::new(move || { let r = resumen(); format!("Hoy + nuevas · {}", r.pendientes as usize + (r.nuevas as usize).min(c.cfg_repaso.with(|x| x.n))) }))}
                {modo("nuevas", Box::new(move || format!("Nuevas · {}", resumen().nuevas)))}
                {modo("falladas", Box::new(move || format!("Falladas · {}", resumen().falladas)))}
                {modo("todo", Box::new(move || format!("Todo aleatorio · {}", resumen().total)))}
                {modo("simulacro", Box::new(|| "⏱ Simulacro de test".to_string()))}
            </div></div>
            <div class="fila"><div class="k">"TIPO"</div><div class="chips">{tipo("todas", "Todas")}{tipo("test", "Tipo test (a–d)")}{tipo("abiertas", "Abiertas")}</div></div>
            <div class="fila"><div class="k">"TAMAÑO"</div><div class="chips">{tam(10)}{tam(20)}{tam(40)}{tam(999)}</div></div>
            <div class="fila"><div class="k"></div>
                <button class="boton" on:click=move |_| empezar(c, None)>"▶ Empezar sesión"</button>
                <span class="mono tenue atajos">"ESPACIO MOSTRAR · A–D ELEGIR · 1 FALLÉ · 2 DUDÉ · 3 LO SABÍA · ESC SALIR"</span></div>
        </div>
        <h2 class="seccion"><b>"i"</b>"Cómo funciona"</h2>
        <div class="panel md texto-2">
            <p>"Las preguntas salen de tus callouts " <code>"> [!question]-"</code> ": si añades una en una nota, aparece aquí al recargar. Cada acierto sube la tarjeta una caja y la aleja en el tiempo (1, 2, 4, 8 y 16 días); un fallo la devuelve a la caja 1 y vuelve a salir al final de la sesión. «Dominada» = caja 3 o más."</p>
            {feynman.then(|| view! { <p>"✎ " <strong>"Feynman activo:"</strong> " en las preguntas abiertas escribe primero tu explicación y compárala con la respuesta."</p> })}
            {intercalado.then(|| view! { <p>"⇄ " <strong>"Intercalado activo:"</strong> " las preguntas alternan asignaturas."</p> })}
        </div>
    }
}

fn opcion_vista(o: Opcion, clase: &'static str, deshabilitada: bool, al_pulsar: impl Fn() + 'static) -> impl IntoView {
    view! {
        <button class=format!("opcion {clase}") disabled=deshabilitada on:click=move |_| al_pulsar()>
            <span class="letra">{o.letra.to_ascii_uppercase().to_string()}</span>
            <span inner_html=md_linea(&o.texto)></span>
        </button>
    }
}

#[component]
fn TarjetaVista(s: Sesion) -> impl IntoView {
    let c = ctx();
    let Some(p) = pregunta(&c, &s.cola[s.i]) else { return view! { <div class="vacio">"Pregunta no encontrada"</div> }.into_any() };
    let (color, corto) =
        c.contenido.with_untracked(|x| x.as_ref().and_then(|x| x.unidad(&p.unidad)).map(|u| (u.color.clone(), u.corto.clone())).unwrap_or_default());
    let boveda = c.contenido.with_untracked(|x| x.as_ref().map(|x| x.boveda.clone()).unwrap_or_default());
    let uri = util::uri_obsidian(&boveda, &p.ruta);
    let ops = opciones::parsear(&p.opciones);
    let correcta = opciones::letra_correcta(&p.respuesta);
    let es_test = ops.len() >= 2 && correcta.is_some();
    let enunciado = md_linea(p.pregunta.trim_start_matches(|ch: char| ch.is_ascii_digit()).trim_start_matches(". "));
    let salir = move |_| c.repaso.set(None);

    if s.simulacro {
        let inicio = s.inicio;
        let tic = RwSignal::new(0u32);
        let h = set_interval_with_handle(move || tic.update(|x| *x += 1), Duration::from_secs(1)).ok();
        on_cleanup(move || {
            if let Some(h) = h {
                h.clear()
            }
        });
        let reloj = move || {
            tic.get();
            let t = ((ms() - inicio) / 1000.0) as u32;
            format!("{:02}:{:02}", t / 60, t % 60)
        };
        let n = s.cola.len();
        return view! {
            <div class="tarjeta-zona">
                <div class="barra-sup mono tenue">
                    <span style="color:var(--ambar)">{format!("⏱ SIMULACRO · PREGUNTA {} / {n}", s.i + 1)}</span>
                    <span class="reloj">{reloj}</span>
                    <button class="boton fantasma pequeno" on:click=salir>"✕ Abandonar " <kbd>"Esc"</kbd></button>
                </div>
                <div class="progreso-barra ambar"><i style=format!("width:{}%", 100 * s.i / n)></i></div>
                <article class="panel brillo tarjeta" style="--c:var(--ambar)">
                    <div class="meta"><span class="etiqueta" style=format!("--c:{color}")>{format!("{} · {corto}", p.unidad)}</span><span>{p.seccion.clone()}</span>
                        <span class="derecha">"SIN CORRECCIÓN HASTA EL FINAL"</span></div>
                    <div class="pregunta" inner_html=enunciado></div>
                    <div class="opciones">{ops.into_iter().map(|o| { let l = o.letra; opcion_vista(o, "", false, move || responder_simulacro(c, l)) }).collect_view()}</div>
                    <div class="notas-tarjeta"><span class="atajos">"PULSA A–D · NO HAY VUELTA ATRÁS, COMO EN EL EXAMEN"</span></div>
                </article>
            </div>
        }
        .into_any();
    }

    let caja = c.estado.with_untracked(|e| e.tarjeta(&p.id).map(|t| format!("CAJA {}/5", t.caja))).unwrap_or_else(|| "NUEVA".into());
    let hechas = s.resultados.len();
    let feynman = c.metodo("feynman") && !es_test;
    let sesion_txt = match (p.sesion, p.test) {
        (Some(n), _) => format!("S{n:02}"),
        (None, true) => "TEST".into(),
        _ => "GUÍA".into(),
    };
    let opciones_v = es_test.then(|| {
        ops.into_iter()
            .map(|o| {
                let l = o.letra;
                let clase = match s.elegida {
                    Some(_) if Some(l) == correcta => "correcta",
                    Some(e) if e == l => "incorrecta",
                    _ => "",
                };
                opcion_vista(o, clase, s.elegida.is_some(), move || elegir(c, l))
            })
            .collect_view()
    });
    let respuesta = s.revelada.then(|| {
        let cab = if es_test {
            if s.elegida == correcta {
                "✓ CORRECTO".to_string()
            } else {
                format!("✕ ERA LA {}", correcta.unwrap_or('?').to_ascii_uppercase())
            }
        } else {
            "RESPUESTA".into()
        };
        let mia = (!s.explicacion.trim().is_empty())
            .then(|| view! { <div class="mi-explicacion"><div class="k">"TU EXPLICACIÓN"</div><div>{s.explicacion.clone()}</div></div> });
        view! { {mia}<div class="respuesta"><div class="k">{cab}</div><div class="md" inner_html=md(&p.respuesta)></div></div> }
    });
    let feynman_v = (feynman && !s.revelada).then(|| {
        view! {
            <textarea class="explicar" placeholder="✎ Feynman: explícalo con tus palabras, como a alguien que no sabe nada. Ctrl+Enter para comparar."
                prop:value=s.explicacion.clone()
                on:input=move |e| { let v = event_target_value(&e); c.repaso.update_untracked(|s| if let Some(s) = s { s.explicacion = v }); }></textarea>
        }
    });
    let fallo = es_test && s.revelada && s.elegida != correcta;
    let acciones = if !s.revelada {
        if es_test {
            view! { <span class="atajos">"PULSA A–D O HAZ CLIC"</span> }.into_any()
        } else {
            view! { <button class="boton" on:click=move |_| revelar(c)>{if feynman { "Comparar " } else { "Mostrar respuesta " }}<kbd>"Espacio"</kbd></button> }
                .into_any()
        }
    } else if fallo {
        view! { <button class="boton" on:click=move |_| responder(c, 1)>"Siguiente " <kbd>"Enter"</kbd></button> }.into_any()
    } else {
        view! {
            <div class="calificar">
                {(!es_test).then(|| view! { <button class="boton f" on:click=move |_| responder(c, 1)>"Fallé " <kbd>"1"</kbd></button> })}
                <button class="boton d" on:click=move |_| responder(c, 2)>"Dudé " <kbd>"2"</kbd></button>
                <button class="boton b" on:click=move |_| responder(c, 3)>"Lo sabía " <kbd>"3"</kbd></button>
            </div>
        }
        .into_any()
    };
    let extra = s.cola.len().saturating_sub(s.total);
    view! {
        <div class="tarjeta-zona">
            <div class="barra-sup mono tenue">
                <span>{format!("{}{} / {}{}", if s.titulo.is_empty() { String::new() } else { format!("{} · ", s.titulo.to_uppercase()) }, hechas + 1, s.total, if extra > 0 { format!(" · +{extra} REPETICIONES") } else { String::new() })}</span>
                <button class="boton fantasma pequeno" on:click=salir>"✕ Salir " <kbd>"Esc"</kbd></button>
            </div>
            <div class="progreso-barra"><i style=format!("width:{}%", 100 * hechas / s.cola.len().max(1))></i></div>
            <article class="panel brillo tarjeta" style=format!("--c:{color}")>
                <div class="meta"><span class="etiqueta" style=format!("--c:{color}")>{format!("{} · {corto}", p.unidad)}</span>
                    <span>{sesion_txt}</span><span>"·"</span><span>{if p.seccion.is_empty() { p.nota.clone() } else { p.seccion.clone() }}</span>
                    <span class="derecha">{caja}</span></div>
                <div class="pregunta" inner_html=enunciado></div>
                {opciones_v}
                {feynman_v}
                {respuesta}
                <div class="notas-tarjeta">
                    <a class="mono enlace-nota" href=uri.clone() on:click=move |e| { e.prevent_default(); plataforma::abrir(&uri); }>{format!("↗ ABRIR «{}» EN OBSIDIAN", p.nota.to_uppercase())}</a>
                    {acciones}
                </div>
            </article>
        </div>
    }
    .into_any()
}

#[component]
fn Fin(s: Sesion) -> impl IntoView {
    let c = ctx();
    let mut primeras: Vec<(String, u8)> = Vec::new();
    for (id, n) in &s.resultados {
        if !primeras.iter().any(|(x, _)| x == id) {
            primeras.push((id.clone(), *n));
        }
    }
    let bien = primeras.iter().filter(|(_, n)| *n == 3).count();
    let total = primeras.len();
    let falladas: Vec<Pregunta> = primeras.iter().filter(|(_, n)| *n == 1).filter_map(|(id, _)| pregunta(&c, id)).collect();
    let racha = c.estado.with_untracked(|e| e.racha(&hoy()));
    // Mapas conceptuales: propone un concepto relacionado con lo que más ha fallado.
    let concepto = c
        .metodo("mapas")
        .then(|| {
            c.contenido.with_untracked(|x| {
                let x = x.as_ref()?;
                let p = falladas.first().or_else(|| primeras.first().and_then(|(id, _)| x.pregunta(id)))?;
                x.conceptos
                    .nodos
                    .iter()
                    .find(|k| k.unidad == p.unidad && (k.nota == p.nota || p.pregunta.to_lowercase().contains(&k.nombre.to_lowercase())))
                    .cloned()
            })
        })
        .flatten();
    view! {
        <div class="tarjeta-zona"><div class="panel brillo resumen-final">
            <div class="mono pre-acento">"SESIÓN COMPLETADA"</div>
            <div class="grande">{format!("{}%", (100 * bien).checked_div(total).unwrap_or(0))}</div>
            <div class="tenue">{format!("{bien} de {total} a la primera · {} respuestas en total · racha de {}", s.resultados.len(), crate::util::dias(racha as i64))}</div>
            {(!falladas.is_empty()).then(|| view! {
                <ul class="lista-falladas">{falladas.into_iter().map(|p| view! { <li inner_html=md_linea(&p.pregunta)></li> }).collect_view()}</ul>
            })}
            {concepto.map(|k| { let id = k.id.clone(); view! {
                <div class="panel sugerencia"><span>"⌬ Para cerrar, mira cómo encaja " <a href=format!("#/atlas?c={id}")>{k.nombre.clone()}</a> " en el atlas."</span></div>
            }})}
            <div class="acciones-centro">
                <button class="boton" on:click=move |_| c.repaso.set(None)>"▶ Otra sesión"</button>
                <a class="boton fantasma" href="#/panel">"Panel"</a>
            </div>
        </div></div>
    }
}

#[component]
fn FinSimulacro(s: Sesion) -> impl IntoView {
    let c = ctx();
    let n = s.respuestas.len();
    let ok = s.respuestas.iter().filter(|r| r.2).count();
    let nota = if n > 0 { 10.0 * ok as f64 / n as f64 } else { 0.0 };
    let color = if nota >= 7.0 {
        "var(--verde)"
    } else if nota >= 5.0 {
        "var(--ambar)"
    } else {
        "var(--rojo)"
    };
    let t = ((s.fin.unwrap_or_else(ms) - s.inicio) / 1000.0) as u32;
    let boveda = c.contenido.with_untracked(|x| x.as_ref().map(|x| x.boveda.clone()).unwrap_or_default());
    let mal: Vec<(Pregunta, char)> = s.respuestas.iter().filter(|r| !r.2).filter_map(|r| Some((pregunta(&c, &r.0)?, r.1))).collect();
    let ids_mal: Vec<String> = mal.iter().map(|(p, _)| p.id.clone()).collect();
    let n_mal = mal.len();
    view! {
        <div class="tarjeta-zona"><div class="panel brillo resumen-final">
            <div class="mono pre-ambar">{format!("SIMULACRO TERMINADO · {:02}:{:02}", t / 60, t % 60)}</div>
            <div class="grande" style=format!("color:{color}")>{coma(nota)}</div>
            <div class="tenue">{format!("{ok} de {n} correctas · {}", if nota >= 7.0 { "por encima del 70 de la beca" } else if nota >= 5.0 { "aprobado, pero por debajo de 7" } else { "toca repasar" })}</div>
            <div class="fallos">{mal.into_iter().map(|(p, elegida)| {
                let ops = opciones::parsear(&p.opciones);
                let txt = ops.iter().find(|o| o.letra == elegida).map(|o| o.texto.clone()).unwrap_or_default();
                let uri = util::uri_obsidian(&boveda, &p.ruta);
                view! {
                    <div class="panel fallo">
                        <div class="enunciado" inner_html=md_linea(&p.pregunta)></div>
                        <div class="tu-respuesta" inner_html=format!("✕ Tu respuesta: {}) {}", elegida.to_ascii_uppercase(), md_linea(&txt))></div>
                        <div class="md" inner_html=md(&p.respuesta)></div>
                        <a class="mono enlace-nota" href=uri.clone() on:click=move |e| { e.prevent_default(); plataforma::abrir(&uri); }>{format!("↗ {} › {}", p.nota, p.seccion)}</a>
                    </div>
                }
            }).collect_view()}</div>
            <div class="acciones-centro">
                <button class="boton" on:click=move |_| { c.repaso.set(None); empezar(c, None); }>"▶ Otro simulacro"</button>
                {(n_mal > 0).then(|| view! { <button class="boton fantasma" on:click=move |_| empezar(c, Some(("Falladas del simulacro".into(), ids_mal.clone())))>{format!("Repasar las {n_mal} falladas")}</button> })}
                <a class="boton fantasma" href="#/panel">"Panel"</a>
            </div>
        </div></div>
    }
}
