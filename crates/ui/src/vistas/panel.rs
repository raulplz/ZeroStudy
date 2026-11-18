use leptos::prelude::*;
use zerostudy_nucleo::progreso::{dias_hasta, sumar_dias};

use crate::app::ctx;
use crate::util::{coma, fecha_es, hora, hoy, pct};
use crate::vistas::{anillo, cabecera};

#[component]
pub fn Panel() -> impl IntoView {
    let c = ctx();
    move || {
        let h = hoy();
        let cfg = c.config.get();
        c.contenido.with(|x| {
            let x = x.as_ref()?;
            let e = c.estado.get();
            let r = e.resumen(x, &h, |_| true);
            let saludo = match hora() {
                0..=6 => "Buenas noches",
                7..=13 => "Buenos días",
                14..=20 => "Buenas tardes",
                _ => "Buenas noches",
            };
            let mut proximos: Vec<(i64, zerostudy_nucleo::Plazo)> = x
                .plazos
                .iter()
                .filter(|p| p.estado != "entregado")
                .filter_map(|p| Some((dias_hasta(&h, p.fecha.as_deref()?)?, p.clone())))
                .filter(|(d, _)| *d >= 0)
                .collect();
            proximos.sort_by_key(|(d, _)| *d);
            let sin_fecha: Vec<_> = x.plazos.iter().filter(|p| p.fecha.is_none()).cloned().collect();
            let color_u = |u: &str| x.unidad(u).map(|u| u.color.clone()).unwrap_or_else(|| "var(--acento)".into());

            // Avisos según las metodologías activas
            let aviso_simulacro = (cfg.metodos.iter().any(|m| m == "recuperacion"))
                .then(|| proximos.first().filter(|(d, _)| *d <= 14).cloned())
                .flatten()
                .map(|(d, p)| {
                    let u = p.unidad.clone();
                    view! {
                        <div class="panel banner" style="--c:var(--ambar)">
                            <span class="icono-banner">"⏱"</span>
                            <div><b>{format!("{} en {}", p.que, crate::util::dias(d))}</b><div class="tenue">"Práctica de recuperación: un simulacro cronometrado ahora vale más que releer."</div></div>
                            <button class="boton" on:click=move |_| {
                                c.cfg_repaso.set(crate::repaso::Cfg { unidades: vec![u.clone()], modo: "simulacro", tipo: "todas", n: 20 });
                                crate::repaso::empezar(c, None);
                            }>"Simulacro"</button>
                        </div>
                    }
                });
            let concepto_dia = (cfg.metodos.iter().any(|m| m == "mapas") && !x.conceptos.nodos.is_empty()).then(|| {
                let dia: u32 = h.bytes().map(u32::from).sum();
                let nucleo: Vec<_> = x.conceptos.nodos.iter().filter(|k| k.importancia >= 3).collect();
                let k = nucleo.get(dia as usize % nucleo.len().max(1)).copied().unwrap_or(&x.conceptos.nodos[0]);
                view! {
                    <div class="panel banner" style=format!("--c:{}", color_u(&k.unidad))>
                        <span class="icono-banner">"⌬"</span>
                        <div><b>{format!("Concepto del día · {}", k.nombre)}</b><div class="tenue recorte" inner_html=crate::util::md_linea(&k.definicion)></div></div>
                        <a class="boton fantasma" href=format!("#/atlas?c={}", k.id)>"Ver en el atlas"</a>
                    </div>
                }
            });

            // Mapa de calor: 17 semanas que acaban en la actual
            let dow = (js_sys::Date::new_0().get_day() + 6) % 7;
            let lunes = sumar_dias(&h, -(dow as i64));
            let mut celdas = Vec::new();
            for w in (0..17).rev() {
                for k in 0..7 {
                    let d = sumar_dias(&lunes, -7 * w + k);
                    let n = *e.por_dia.get(&d).unwrap_or(&0);
                    let nivel = if d > h { -1 } else if n == 0 { 0 } else if n < 10 { 1 } else if n < 25 { 2 } else if n < 50 { 3 } else { 4 };
                    celdas.push(view! { <span data-n=nivel.to_string() title=format!("{d}: {n} repasos")></span> });
                }
            }
            let sims: Vec<_> = e.simulacros.iter().rev().take(4).cloned().collect();
            let prox = proximos.first().cloned();
            Some(view! {
                {cabecera(
                    format!("SYS://PANEL · {}", fecha_es(&h, &[("weekday", "long"), ("day", "numeric"), ("month", "long")]).to_uppercase()),
                    format!("{saludo}."),
                    vec![format!("{} PREGUNTAS · {} CONCEPTOS", x.preguntas.len(), x.conceptos.nodos.len()), format!("LEÍDO DE «{}»", x.boveda.to_uppercase())],
                )}
                {aviso_simulacro}
                <section class="rejilla-4">
                    <div class="panel dato principal"><div class="k">"Para repasar hoy"</div><div class="v">{r.pendientes}<small>"tarjetas"</small></div>
                        <div class="s">{format!("{} nuevas sin estrenar", r.nuevas)}</div>
                        <div class="mt"><a class="boton" href="#/repaso?auto=hoy">"▶ Empezar"</a></div></div>
                    <div class="panel dato"><div class="k">"Racha"</div><div class="v">{e.racha(&h)}<small>"días"</small></div><div class="s">{format!("{} con repaso en total", crate::util::dias(e.por_dia.len() as i64))}</div></div>
                    <div class="panel dato"><div class="k">"Dominadas"</div><div class="v">{r.dominadas}<small>{format!("/ {}", r.total)}</small></div><div class="s">{format!("{} % del banco de preguntas", pct(r.dominio()))}</div></div>
                    <div class="panel dato"><div class="k">"Próximo plazo"</div>{match prox {
                        Some((d, p)) => view! { <div class="v" class:rojo={d <= 7}>{d}<small>"días"</small></div><div class="s">{format!("{} · {}", p.unidad, p.que)}</div> }.into_any(),
                        None => view! { <div class="v">"—"</div><div class="s">"Sin plazos con fecha"</div> }.into_any(),
                    }}</div>
                </section>
                {concepto_dia}
                <h2 class="seccion"><b>"01"</b>"Asignaturas"</h2>
                <section class="rejilla-4">{x.unidades.iter().map(|u| {
                    let ru = e.resumen(x, &h, |p| p.unidad == u.id);
                    let nc = x.conceptos.nodos.iter().filter(|k| k.unidad == u.id).count();
                    let con = u.sesiones.iter().filter(|s| s.bytes > 0).count();
                    view! {
                        <article class="panel unidad" style=format!("--c:{}", u.color)>
                            <div class="num">{format!("{} · {}", u.tipo.to_uppercase(), u.id)}</div><h3>{u.titulo.clone()}</h3>
                            <div class="anillo">{anillo(ru.dominio(), u.color.clone())}<div><div class="pct">{format!("{} %", pct(ru.dominio()))}</div><div class="tenue mono mini-k">"DOMINIO"</div></div></div>
                            <div class="mini"><div><b>{ru.pendientes}</b>"HOY"</div><div><b>{ru.total}</b>"PREG."</div><div><b>{nc}</b>"CONC."</div></div>
                            <div class="acciones"><a class="boton fantasma" href=format!("#/repaso?u={}", u.id)>"Repasar"</a><a class="boton fantasma" href=format!("#/temas?u={}", u.id)>{format!("{con} ses.")}</a></div>
                        </article>
                    }
                }).collect_view()}</section>
                <section class="rejilla-2 mt">
                    <div><h2 class="seccion"><b>"02"</b>"Plazos"</h2><div class="panel">
                        {proximos.iter().map(|(d, p)| view! {
                            <div class="plazo" class:urgente={*d <= 7} class:cerca={*d > 7 && *d <= 21}>
                                <div class="dias">{*d}<small>{if *d == 1 { "DÍA" } else { "DÍAS" }}</small></div>
                                <div><div class="que">{p.que.clone()}</div><div class="meta">{fecha_es(p.fecha.as_deref().unwrap_or(""), &[("weekday", "short"), ("day", "numeric"), ("month", "short"), ("year", "numeric")]).to_uppercase()}</div></div>
                                <span class="etiqueta" style=format!("--c:{}", color_u(&p.unidad))>{format!("{} · {}", p.unidad, p.estado)}</span>
                            </div>
                        }).collect_view()}
                        {proximos.is_empty().then(|| view! { <div class="vacio">"Nada con fecha por delante."</div> })}
                        {sin_fecha.into_iter().map(|p| view! {
                            <div class="plazo"><div class="dias">"?"<small>"SIN FECHA"</small></div><div><div class="que">{p.que.clone()}</div></div>
                                <span class="etiqueta" style=format!("--c:{}", color_u(&p.unidad))>{p.unidad.clone()}</span></div>
                        }).collect_view()}
                        <div class="mono tenue nota-pie">"FUENTE: «00 - PLAZOS» EN TUS APUNTES"</div>
                    </div></div>
                    <div><h2 class="seccion"><b>"03"</b>"Actividad"</h2><div class="panel">
                        <div class="calor">{celdas}</div>
                        <div class="mono tenue nota-pie">"ÚLTIMAS 17 SEMANAS · CADA CUADRO ES UN DÍA · TODOS TUS DISPOSITIVOS"</div>
                        {(!sims.is_empty()).then(|| view! {
                            <div class="mt">{sims.into_iter().map(|s| { let nota = if s.n > 0 { 10.0 * s.aciertos as f64 / s.n as f64 } else { 0.0 };
                                let col = if nota >= 7.0 { "var(--verde)" } else if nota >= 5.0 { "var(--ambar)" } else { "var(--rojo)" };
                                view! { <div class="barra-sesion simulacro"><span>{format!("{} · {}", &s.t[5..10], if s.unidades.is_empty() { "todas".into() } else { s.unidades.join("+") })}</span>
                                    <span class="pista" style=format!("--c:{col}")><i style=format!("width:{}%", nota * 10.0)></i></span><span class="brillo">{coma(nota)}</span></div> }
                            }).collect_view()}<div class="mono tenue nota-pie">"ÚLTIMOS SIMULACROS (NOTA /10)"</div></div>
                        })}
                        <div class="mt fila-botones"><a class="boton fantasma" href="#/repaso?auto=falladas">"Repasar falladas"</a><a class="boton fantasma" href="#/temas">"Elegir tema"</a><a class="boton fantasma" href="#/atlas">"Abrir el atlas"</a></div>
                    </div></div>
                </section>
            })
        })
    }
}
