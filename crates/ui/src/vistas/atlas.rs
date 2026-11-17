//! Atlas de conceptos: grafo SVG con las posiciones ya calculadas en el núcleo.
//! Pan con puntero, zoom con la rueda alrededor del cursor, buscador, filtros y ficha lateral.

use leptos::html::Div;
use leptos::prelude::*;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use zerostudy_nucleo::{atlas as geometria, normalizar, verbo, Contenido};

use crate::app::ctx;
use crate::plataforma;
use crate::repaso;
use crate::util::{md_linea, param, uri_obsidian};
use crate::vistas::cabecera;

struct Nodo {
    id: String,
    nombre: String,
    x: f64,
    y: f64,
    r: f64,
    color: String,
    unidad: String,
    imp: u8,
    /// Rótulo siempre visible (top 7 de su asignatura).
    fijo: bool,
    busq: String,
    puente: bool,
}

struct Lado {
    de: usize,
    a: usize,
    cruzada: bool,
    color: String,
}

fn color_unidad(d: &Contenido, u: &str) -> String {
    d.unidad(u).map(|u| u.color.clone()).unwrap_or_else(|| "var(--acento)".into())
}

fn construir(d: &Contenido) -> (Vec<Nodo>, Vec<Lado>) {
    let grado = geometria::grados(&d.conceptos);
    let mut nodos: Vec<Nodo> = d
        .conceptos
        .nodos
        .iter()
        .map(|c| {
            let g = *grado.get(c.id.as_str()).unwrap_or(&0);
            Nodo {
                id: c.id.clone(),
                nombre: c.nombre.clone(),
                x: c.x,
                y: c.y,
                r: geometria::radio(c.importancia, g),
                color: color_unidad(d, &c.unidad),
                unidad: c.unidad.clone(),
                imp: c.importancia,
                fijo: false,
                busq: normalizar(&format!("{} {}", c.nombre, c.aliases.join(" "))),
                puente: false,
            }
        })
        .collect();
    for u in &d.unidades {
        let mut idx: Vec<usize> = (0..nodos.len()).filter(|&i| nodos[i].unidad == u.id).collect();
        let peso = |i: usize| nodos[i].imp as usize * 3 + grado.get(nodos[i].id.as_str()).copied().unwrap_or(0);
        idx.sort_by_key(|&i| std::cmp::Reverse(peso(i)));
        for i in idx.into_iter().take(7) {
            nodos[i].fijo = true;
        }
    }
    let pos = |id: &str| nodos.iter().position(|n| n.id == id);
    let lados: Vec<Lado> = d
        .conceptos
        .aristas
        .iter()
        .filter_map(|a| {
            let (i, j) = (pos(&a.de)?, pos(&a.a)?);
            Some(Lado { de: i, a: j, cruzada: a.cruzada, color: if a.cruzada { "#a8dcff".into() } else { nodos[i].color.clone() } })
        })
        .collect();
    for l in &lados {
        if l.cruzada {
            nodos[l.de].puente = true;
            nodos[l.a].puente = true;
        }
    }
    (nodos, lados)
}

#[component]
pub fn Atlas() -> impl IntoView {
    let c = ctx();
    let Some(datos) = c.contenido.get_untracked() else { return ().into_any() };
    let datos = Arc::new(datos);
    if datos.conceptos.nodos.is_empty() {
        return view! {
            {cabecera("SYS://ATLAS · GRAFO DE CONCEPTOS".into(), "Atlas".into(), vec![])}
            <div class="panel vacio">"Todavía no hay conceptos. Indica la carpeta de mapas en la configuración (o sincroniza el contenido) y recarga."</div>
        }
        .into_any();
    }
    let (nodos, lados) = construir(&datos);
    let nodos = Arc::new(nodos);
    let n_puentes = lados.iter().filter(|l| l.cruzada).count();
    let lado_txt = vec![
        format!("{} CONCEPTOS · {} RELACIONES", nodos.len(), lados.len()),
        format!("{n_puentes} PUENTES ENTRE ASIGNATURAS"),
    ];

    let caja = NodeRef::<Div>::new();
    // (tx, ty, k)
    let vista = RwSignal::new((0.0f64, 0.0f64, 1.0f64));
    let nivel = Memo::new(move |_| vista.with(|v| if v.2 > 1.6 { 2u8 } else if v.2 > 1.05 { 1 } else { 0 }));
    let buscar = RwSignal::new(String::new());
    let q = Memo::new(move |_| normalizar(buscar.get().trim()));
    let ocultas = RwSignal::new(Vec::<String>::new());
    let solo = RwSignal::new(false);
    let foco = RwSignal::new(None::<String>);
    let arrastre = StoredValue::new(None::<(f64, f64, f64, f64)>);
    let movido = StoredValue::new(false);

    let vecinos = {
        let d = datos.clone();
        Memo::new(move |_| {
            let mut s = HashSet::new();
            if let Some(f) = foco.get() {
                for a in &d.conceptos.aristas {
                    if a.de == f {
                        s.insert(a.a.clone());
                    }
                    if a.a == f {
                        s.insert(a.de.clone());
                    }
                }
                s.insert(f);
            }
            s
        })
    };

    let tamano = move || {
        caja.get_untracked().map(|e| {
            let r = e.get_bounding_client_rect();
            (r.width(), r.height())
        })
    };
    let encuadrar = {
        let nodos = nodos.clone();
        Callback::new(move |_: ()| {
            let Some((w, h)) = tamano() else { return };
            let (mut x0, mut x1, mut y0, mut y1) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
            for n in nodos.iter() {
                x0 = x0.min(n.x);
                x1 = x1.max(n.x);
                y0 = y0.min(n.y);
                y1 = y1.max(n.y);
            }
            let (x0, x1, y0, y1) = (x0 - 40.0, x1 + 40.0, y0 - 50.0, y1 + 30.0);
            let k = (w / (x1 - x0)).min((h - 70.0) / (y1 - y0)).clamp(0.2, 1.6);
            vista.set((w / 2.0 - k * (x0 + x1) / 2.0, (h + 50.0) / 2.0 - k * (y0 + y1) / 2.0, k));
        })
    };
    let zoom_en = move |mx: f64, my: f64, f: f64| {
        vista.update(|v| {
            let k2 = (v.2 * f).clamp(0.2, 5.0);
            v.0 = mx - (mx - v.0) * k2 / v.2;
            v.1 = my - (my - v.1) * k2 / v.2;
            v.2 = k2;
        });
    };
    let centrar = {
        let nodos = nodos.clone();
        move |id: &str| {
            let (Some(n), Some((w, h))) = (nodos.iter().find(|n| n.id == id), tamano()) else { return };
            let (sx, sy) = (if w > 760.0 { (w - 380.0) / 2.0 } else { w / 2.0 }, if w > 760.0 { h / 2.0 } else { h * 0.28 });
            vista.update(|v| {
                v.2 = v.2.max(1.0);
                v.0 = sx - v.2 * n.x;
                v.1 = sy - v.2 * n.y;
            });
        }
    };
    let abrir = Callback::new(move |id: String| {
        centrar(&id);
        foco.set(Some(id));
    });

    // encuadre inicial y ficha de la ruta `atlas?c=<id>`
    {
        let inicial = c.ruta.get_untracked();
        set_timeout(
            move || {
                encuadrar.run(());
                if let Some(id) = param(&inicial, "c") {
                    abrir.run(id);
                }
            },
            Duration::from_millis(40),
        );
    }

    let ve = {
        let nodos = nodos.clone();
        move |i: usize| {
            let n = &nodos[i];
            !ocultas.with(|o| o.contains(&n.unidad)) && (!solo.get() || n.puente)
        }
    };

    // ── dibujo ──
    let n_unidades = datos.unidades.len();
    let rotulos_u: Vec<_> = datos
        .unidades
        .iter()
        .enumerate()
        .map(|(i, u)| {
            let (x, y) = geometria::centro(i, n_unidades);
            view! {
                <text x=x y=y text-anchor="middle" fill=u.color.clone() fill-opacity="0.16" class="atlas-grupo">
                    {format!("{} · {}", u.id, u.corto.to_uppercase())}
                </text>
            }
        })
        .collect();
    let svg_lados: Vec<_> = lados
        .iter()
        .map(|l| {
            let (a, b) = (&nodos[l.de], &nodos[l.a]);
            let (ida, idb) = (a.id.clone(), b.id.clone());
            let (i, j, cruzada) = (l.de, l.a, l.cruzada);
            let ve = ve.clone();
            view! {
                <line x1=a.x y1=a.y x2=b.x y2=b.y stroke=l.color.clone()
                    stroke-width=if cruzada { "1.4" } else { "1" }
                    stroke-dasharray=if cruzada { Some("5 4") } else { None }
                    display=move || (!(ve(i) && ve(j) && (!solo.get() || cruzada))).then_some("none")
                    stroke-opacity=move || match foco.get() {
                        Some(f) => if f == ida || f == idb { "0.9" } else { "0.05" },
                        None => if cruzada { "0.55" } else { "0.22" },
                    }/>
            }
        })
        .collect();
    let svg_nodos: Vec<_> = nodos
        .iter()
        .enumerate()
        .map(|(i, n)| {
            let id = n.id.clone();
            let id2 = id.clone();
            let id3 = id.clone();
            let (ve1, ve2) = (ve.clone(), ve.clone());
            let busq = n.busq.clone();
            let busq2 = n.busq.clone();
            let (vec1, vec2) = (vecinos, vecinos);
            let resalta = move |busq: &str, id: &str| {
                if foco.with(Option::is_some) {
                    vec1.with(|v| v.contains(id))
                } else {
                    let q = q.get();
                    q.is_empty() || busq.contains(&q)
                }
            };
            let resalta2 = move |busq: &str, id: &str| {
                if foco.with(Option::is_some) {
                    vec2.with(|v| v.contains(id))
                } else {
                    let q = q.get();
                    q.is_empty() || busq.contains(&q)
                }
            };
            let (fijo, imp) = (n.fijo, n.imp);
            let brillo = (n.imp >= 3).then(|| format!("filter:drop-shadow(0 0 4px {})", n.color));
            view! {
                <circle cx=n.x cy=n.y r=n.r fill=n.color.clone() stroke="var(--fondo)" stroke-width="1.5" class="nodo"
                    style=brillo
                    display=move || (!ve1(i)).then_some("none")
                    fill-opacity=move || if resalta(&busq, &id) { "0.95" } else { "0.12" }
                    on:click=move |e| {
                        e.stop_propagation();
                        if !movido.get_value() {
                            abrir.run(id3.clone());
                        }
                    }/>
                <text x=n.x y=n.y - n.r - 5.0 text-anchor="middle" class="nodo-etiqueta"
                    display=move || {
                        let busca = foco.with(Option::is_some) || !q.with(String::is_empty);
                        let rotula = if busca { resalta2(&busq2, &id2) } else { nivel.get() == 2 || (nivel.get() == 1 && imp >= 3) || fijo };
                        (!(ve2(i) && rotula)).then_some("none")
                    }>
                    {n.nombre.clone()}
                </text>
            }
        })
        .collect();

    // ── herramientas ──
    let chips_u: Vec<_> = datos
        .unidades
        .iter()
        .map(|u| {
            let (id, id2) = (u.id.clone(), u.id.clone());
            view! {
                <span class="chip" style=format!("--c:{}", u.color) role="button"
                    class:on=move || ocultas.with(|o| !o.contains(&id))
                    on:click=move |_| ocultas.update(|o| if let Some(p) = o.iter().position(|x| *x == id2) { o.remove(p); } else { o.push(id2.clone()) })>
                    <i></i>{u.corto.clone()}
                </span>
            }
        })
        .collect();

    // ── ficha ──
    let ficha = {
        let d = datos.clone();
        move || {
            let id = foco.get()?;
            let k = d.concepto(&id)?.clone();
            let color = color_unidad(&d, &k.unidad);
            let nombre_de = |i: &str| d.concepto(i).map(|x| x.nombre.clone()).unwrap_or_default();
            let unidad_de = |i: &str| d.concepto(i).map(|x| x.unidad.clone()).unwrap_or_default();
            let rel = |otro: String, saliente: bool, tipo: &str, etiqueta: &str, cruzada: bool| {
                let verbo_txt = if etiqueta.is_empty() { verbo(tipo).to_string() } else { etiqueta.to_string() };
                let (nombre, u2) = (nombre_de(&otro), unidad_de(&otro));
                let flecha = if cruzada { "⇄ " } else if saliente { "→ " } else { "← " };
                let c2 = color_unidad(&d, &u2);
                view! {
                    <span class="rel" role="button" on:click=move |_| abrir.run(otro.clone())>
                        {flecha}
                        {saliente.then(|| format!("{verbo_txt} "))}
                        <b>{nombre}</b>
                        {(!saliente).then(|| format!(" {verbo_txt}"))}
                        {cruzada.then(|| view! { <span class="etiqueta" style=format!("--c:{c2}")>{u2.clone()}</span> })}
                    </span>
                }
            };
            let mut rels = Vec::new();
            for a in d.conceptos.aristas.iter().filter(|a| a.de == id) {
                rels.push(rel(a.a.clone(), true, &a.tipo, &a.etiqueta, a.cruzada).into_any());
            }
            for a in d.conceptos.aristas.iter().filter(|a| a.a == id) {
                rels.push(rel(a.de.clone(), false, &a.tipo, &a.etiqueta, a.cruzada).into_any());
            }
            let preguntas = d.preguntas_de_concepto(&k);
            let ids: Vec<String> = preguntas.iter().map(|p| p.id.clone()).collect();
            let n = ids.len();
            let dominadas = c.estado.with(|e| ids.iter().filter(|i| e.dominada(i)).count());
            let lista: Vec<_> = preguntas
                .iter()
                .take(6)
                .map(|p| view! { <div class="rel fijo" inner_html=format!("· {}", md_linea(p.pregunta.trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == ' ')))></div> })
                .collect();
            let ruta = d
                .unidad(&k.unidad)
                .and_then(|u| u.sesiones.iter().find(|s| s.nota == k.nota).map(|s| s.ruta.clone()).or_else(|| Some(format!("{}/{}.md", u.carpeta, k.nota))))
                .unwrap_or_else(|| format!("{}.md", k.nota));
            let boveda = d.boveda.clone();
            let titulo = format!("Atlas · {}", k.nombre);
            let etiqueta = format!(
                "{} · {}{}",
                k.unidad,
                if k.tipo.is_empty() { "concepto" } else { &k.tipo },
                k.sesion.map(|s| format!(" · S{s:02}")).unwrap_or_default()
            );
            Some(view! {
                <div style=format!("--c:{color}")>
                    <span class="etiqueta" style=format!("--c:{color}")>{etiqueta}</span>
                    <h3>{k.nombre.clone()}</h3>
                    {(!k.aliases.is_empty()).then(|| view! { <div class="alias">{k.aliases.join(" · ")}</div> })}
                    <p class="def" inner_html=md_linea(&k.definicion)></p>
                    <button class="boton fantasma pequeno" on:click=move |_| plataforma::abrir(&uri_obsidian(&boveda, &ruta))>{format!("↗ {}", k.nota)}</button>
                    <div class="mono tenue pie-ficha">{format!("§ {}", k.seccion)}</div>
                    {(!rels.is_empty()).then(|| view! { <h4>"Relaciones"</h4> {rels} })}
                    <h4>{format!("Preguntas relacionadas · {n}")}</h4>
                    {if n > 0 {
                        view! {
                            <div class="mono tenue pie-ficha">{format!("{dominadas} DOMINADAS")}</div>
                            <button class="boton mt" on:click=move |_| repaso::empezar(c, Some((titulo.clone(), ids.clone())))>{format!("▶ Repasar estas {n}")}</button>
                            {lista}
                        }
                        .into_any()
                    } else {
                        view! { <div class="tenue">"Ninguna pregunta la menciona todavía."</div> }.into_any()
                    }}
                </div>
            })
        }
    };

    let buscar_enter = {
        let nodos = nodos.clone();
        move |e: web_sys::KeyboardEvent| {
            if e.key() == "Enter" {
                let q = q.get_untracked();
                if let Some(n) = nodos.iter().find(|n| !q.is_empty() && n.busq.contains(&q)) {
                    abrir.run(n.id.clone());
                }
            }
        }
    };

    view! {
        {cabecera("SYS://ATLAS · GRAFO DE CONCEPTOS".into(), "Atlas".into(), lado_txt)}
        <div class="panel atlas" node_ref=caja>
            <div class="herramientas">
                <input class="buscar" placeholder="Buscar concepto…" autocomplete="off" prop:value=move || buscar.get()
                    on:input=move |e| buscar.set(event_target_value(&e)) on:keydown=buscar_enter/>
                <div class="chips">{chips_u}</div>
                <span class="chip" role="button" class:on=move || solo.get() on:click=move |_| solo.update(|s| *s = !*s)>"Solo puentes"</span>
                <span class="zoom-botones">
                    <button class="chip" aria-label="Acercar" on:click=move |_| { if let Some((w, h)) = tamano() { zoom_en(w / 2.0, h / 2.0, 1.4) } }>"+"</button>
                    <button class="chip" aria-label="Alejar" on:click=move |_| { if let Some((w, h)) = tamano() { zoom_en(w / 2.0, h / 2.0, 1.0 / 1.4) } }>"−"</button>
                    <button class="chip" aria-label="Encuadrar todo" on:click=move |_| encuadrar.run(())>"⤢"</button>
                </span>
            </div>
            <svg
                on:pointerdown=move |e: web_sys::PointerEvent| {
                    if e.button() != 0 { return; }
                    movido.set_value(false);
                    let v = vista.get_untracked();
                    arrastre.set_value(Some((e.client_x() as f64, e.client_y() as f64, v.0, v.1)));
                }
                on:pointermove=move |e: web_sys::PointerEvent| {
                    let Some((x0, y0, tx, ty)) = arrastre.get_value() else { return };
                    if e.buttons() & 1 == 0 { arrastre.set_value(None); return; }
                    let (dx, dy) = (e.client_x() as f64 - x0, e.client_y() as f64 - y0);
                    if dx.hypot(dy) > 4.0 { movido.set_value(true); }
                    if movido.get_value() { vista.update(|v| { v.0 = tx + dx; v.1 = ty + dy; }); }
                }
                on:pointerup=move |_| arrastre.set_value(None)
                on:pointerleave=move |_| arrastre.set_value(None)
                on:click=move |_| { if !movido.get_value() { foco.set(None); } }
                on:wheel=move |e: web_sys::WheelEvent| {
                    e.prevent_default();
                    let Some(el) = caja.get_untracked() else { return };
                    let r = el.get_bounding_client_rect();
                    zoom_en(e.client_x() as f64 - r.left(), e.client_y() as f64 - r.top(), (-e.delta_y() * 0.0015).exp());
                }>
                <g transform=move || vista.with(|v| format!("translate({} {}) scale({})", v.0, v.1, v.2))>
                    <g>{rotulos_u}</g>
                    <g>{svg_lados}</g>
                    <g>{svg_nodos}</g>
                </g>
            </svg>
            <div class="leyenda">"RUEDA: ZOOM · ARRASTRA PARA MOVER · CLIC EN UN NODO PARA SU FICHA · LÍNEA DISCONTINUA = PUENTE ENTRE ASIGNATURAS"</div>
            <aside class="ficha-concepto" class:abierta=move || foco.with(Option::is_some)>
                <button class="cerrar" aria-label="Cerrar" on:click=move |_| foco.set(None)>"✕"</button>
                {ficha}
            </aside>
        </div>
    }
    .into_any()
}
