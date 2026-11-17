//! «Elegir tema»: encuentra qué estudiar ahora, por asignatura, sesión o concepto.

use leptos::prelude::*;
use zerostudy_nucleo::normalizar;

use crate::app::ctx;
use crate::repaso;
use crate::util::{md_linea, param, pct};
use crate::vistas::cabecera;

#[component]
pub fn Temas() -> impl IntoView {
    let c = ctx();
    let buscar = RwSignal::new(String::new());
    let solo0 = move || param(&c.ruta.get(), "u");
    let resultados = move || {
        let solo = solo0();
        let q = normalizar(buscar.get().trim());
        c.contenido.with(|x| {
            let x = x.as_ref()?;
            let e = c.estado.get();
            // (unidad, sesión n, nº preguntas, dominio) de toda sesión con preguntas; para la sugerencia
            let mut flojas: Vec<(String, u32, String, f64, u32)> = Vec::new();
            let bloques = x
                .unidades
                .iter()
                .filter(|u| solo.as_ref().is_none_or(|s| *s == u.id))
                .map(|u| {
                    let ids_sesion = |nota: &str| -> Vec<String> {
                        x.preguntas.iter().filter(|p| p.unidad == u.id && p.nota == nota).map(|p| p.id.clone()).collect()
                    };
                    let mut filas = Vec::new();
                    for s in u.sesiones.iter().filter(|s| s.bytes > 0) {
                        let ids = ids_sesion(&s.nota);
                        let tema = if s.tema.is_empty() { s.nota.clone() } else { s.tema.clone() };
                        if !q.is_empty() {
                            let en_enunciado = x.preguntas.iter().any(|p| p.unidad == u.id && p.nota == s.nota && normalizar(&p.pregunta).contains(&q));
                            let en_conceptos = x.conceptos.nodos.iter().any(|k| k.unidad == u.id && k.nota == s.nota && normalizar(&k.nombre).contains(&q));
                            if !(normalizar(&tema).contains(&q) || normalizar(&s.nota).contains(&q) || en_enunciado || en_conceptos) {
                                continue;
                            }
                        }
                        let dom = ids.iter().filter(|i| e.dominada(i)).count();
                        let dominio = if ids.is_empty() { 1.0 } else { dom as f64 / ids.len() as f64 };
                        if !ids.is_empty() {
                            flojas.push((u.id.clone(), s.n, tema.clone(), dominio, ids.len() as u32));
                        }
                        filas.push((s.n, tema, ids, dominio));
                    }
                    // conceptos clave de la asignatura que casan con la búsqueda
                    let conceptos: Vec<_> = x
                        .conceptos
                        .nodos
                        .iter()
                        .filter(|k| k.unidad == u.id && k.importancia >= 3)
                        .filter(|k| q.is_empty() || normalizar(&format!("{} {}", k.nombre, k.aliases.join(" "))).contains(&q))
                        .cloned()
                        .collect();
                    let enunciados: Vec<_> = if q.len() >= 3 {
                        x.preguntas.iter().filter(|p| p.unidad == u.id && normalizar(&p.pregunta).contains(&q)).take(5).cloned().collect()
                    } else {
                        Vec::new()
                    };
                    (u, filas, conceptos, enunciados)
                })
                .collect::<Vec<_>>();

            let sugerida = flojas
                .iter()
                .filter(|(_, _, _, _, n)| *n >= 3)
                .min_by(|a, b| a.3.total_cmp(&b.3).then(b.4.cmp(&a.4)))
                .cloned();
            let sugerencia = sugerida.map(|(uid, n, tema, dom, np)| {
                let (color, corto) = x.unidad(&uid).map(|u| (u.color.clone(), u.corto.clone())).unwrap_or_default();
                let nota = x.unidad(&uid).and_then(|u| u.sesiones.iter().find(|s| s.n == n)).map(|s| s.nota.clone()).unwrap_or_default();
                let ids: Vec<String> = x.preguntas.iter().filter(|p| p.unidad == uid && p.nota == nota).map(|p| p.id.clone()).collect();
                let titulo = format!("{corto} · S{n:02}");
                view! {
                    <div class="panel sugerencia" style=format!("--c:{color}")>
                        <div><div class="mono pre-acento">"SUGERENCIA · LA SESIÓN MÁS FLOJA"</div>
                            <b>{format!("{corto} · S{n:02} · {tema}")}</b>
                            <div class="tenue">{format!("{np} preguntas · {} % de dominio. Es donde más rinde repasar ahora.", pct(dom))}</div></div>
                        <button class="boton" on:click=move |_| repaso::empezar(c, Some((titulo.clone(), ids.clone())))>"▶ Estudiar"</button>
                    </div>
                }
            });

            let paneles_vacios = bloques.iter().all(|(_, f, k, en)| f.is_empty() && k.is_empty() && en.is_empty());
            let paneles = bloques
                .into_iter()
                .map(|(u, filas, conceptos, enunciados)| {
                    if filas.is_empty() && conceptos.is_empty() && enunciados.is_empty() {
                        return ().into_any();
                    }
                    let filas_v = filas
                        .into_iter()
                        .map(|(n, tema, ids, dominio)| {
                            let np = ids.len();
                            let titulo = format!("{} · S{n:02}", u.corto);
                            view! {
                                <div class="fila-tema">
                                    <span class="mono sesion-n">{format!("S{n:02}")}</span>
                                    <div class="tema-nombre">{tema}</div>
                                    <span class="mono tenue">{format!("{np} PREG.")}</span>
                                    <span class="pista" style=format!("--c:{}", u.color)><i style=format!("width:{}%", if np == 0 { 0.0 } else { dominio * 100.0 })></i></span>
                                    <span class="mono brillo">{if np == 0 { "—".to_string() } else { format!("{} %", pct(dominio)) }}</span>
                                    <button class="boton fantasma pequeno" disabled=np == 0
                                        on:click=move |_| repaso::empezar(c, Some((titulo.clone(), ids.clone())))>"Estudiar"</button>
                                </div>
                            }
                        })
                        .collect_view();
                    let chips = conceptos
                        .into_iter()
                        .map(|k| {
                            let ps: Vec<String> = x.preguntas_de_concepto(&k).iter().map(|p| p.id.clone()).collect();
                            let titulo = format!("Atlas · {}", k.nombre);
                            let (id, n) = (k.id.clone(), ps.len());
                            view! {
                                <span class="chip on concepto" style=format!("--c:{}", u.color)>
                                    <a href=format!("#/atlas?c={id}")>{format!("★★★ {}", k.nombre)}</a>
                                    {(n > 0).then(|| view! { <button class="mini-boton" title="Repasar sus preguntas" on:click=move |_| repaso::empezar(c, Some((titulo.clone(), ps.clone())))>{format!("▶ {n}")}</button> })}
                                </span>
                            }
                        })
                        .collect_view();
                    let enun = enunciados
                        .into_iter()
                        .map(|p| view! { <div class="coincidencia" inner_html=md_linea(&p.pregunta)></div> })
                        .collect_view();
                    view! {
                        <section class="panel unidad mb" style=format!("--c:{}", u.color)>
                            <div class="num">{format!("{} · {}", u.tipo.to_uppercase(), u.id)}</div>
                            <h3>{u.titulo.clone()}</h3>
                            {filas_v}
                            <div class="chips mt">{chips}</div>
                            {enun}
                        </section>
                    }
                    .into_any()
                })
                .collect_view();
            let vacio = paneles_vacios.then(|| view! { <div class="panel vacio">"Nada coincide con esa búsqueda."</div> });
            Some(view! { {sugerencia} {paneles} {vacio} })
        })
    };
    view! {
        {move || cabecera("SYS://TEMAS · QUÉ ESTUDIO AHORA".into(), "Elegir tema".into(), vec!["BUSCA EN SESIONES, CONCEPTOS Y ENUNCIADOS".into(), solo0().map(|s| format!("FILTRO: {s}")).unwrap_or_default()])}
        <div class="barra-busqueda">
            <input class="buscar ancho" placeholder="Buscar un tema, un concepto o parte de una pregunta…" autocomplete="off"
                prop:value=move || buscar.get_untracked()
                on:input=move |e| buscar.set(event_target_value(&e))/>
            {move || solo0().map(|_| view! { <a class="boton fantasma" href="#/temas">"Ver todas"</a> })}
        </div>
        {resultados}
    }
}
