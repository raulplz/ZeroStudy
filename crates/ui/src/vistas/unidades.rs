//! Asignaturas: sesiones, temas y dominio por grupo de preguntas.

use leptos::prelude::*;
use std::collections::BTreeMap;

use crate::app::ctx;
use crate::plataforma;
use crate::util::{hoy, param, uri_obsidian};
use crate::vistas::cabecera;

#[component]
pub fn Unidades() -> impl IntoView {
    let c = ctx();
    move || {
        let solo = param(&c.ruta.get(), "u");
        let h = hoy();
        c.contenido.with(|x| {
            let x = x.as_ref()?;
            let e = c.estado.get();
            let boveda = x.boveda.clone();
            let paneles = x
                .unidades
                .iter()
                .filter(|u| solo.as_ref().is_none_or(|s| *s == u.id))
                .map(|u| {
                    let r = e.resumen(x, &h, |p| p.unidad == u.id);
                    let mut grupos: BTreeMap<String, Vec<&zerostudy_nucleo::Pregunta>> = BTreeMap::new();
                    for p in x.preguntas.iter().filter(|p| p.unidad == u.id) {
                        let k = if p.test { "TEST".to_string() } else if let Some(s) = p.sesion { format!("S{s:02}") } else { "GUÍA".to_string() };
                        grupos.entry(k).or_default().push(p);
                    }
                    let n_ses = u.sesiones.iter().map(|s| s.n).max().unwrap_or(0).max(13);
                    let casillas = (1..=n_ses)
                        .map(|n| {
                            match u.sesiones.iter().find(|s| s.n == n).filter(|s| s.bytes > 0) {
                                Some(s) => {
                                    let (b, ruta) = (boveda.clone(), s.ruta.clone());
                                    view! {
                                        <button class="sesion con" title=format!("S{n} · {}", if s.tema.is_empty() { &s.nota } else { &s.tema })
                                            on:click=move |_| plataforma::abrir(&uri_obsidian(&b, &ruta))>{n}</button>
                                    }
                                    .into_any()
                                }
                                None => view! { <span class="sesion" title=format!("S{n}: sin contenido")>{n}</span> }.into_any(),
                            }
                        })
                        .collect_view();
                    let temas = u
                        .sesiones
                        .iter()
                        .filter(|s| s.bytes > 0)
                        .map(|s| {
                            let t = if s.tema.is_empty() { s.nota.split_once(" - ").map(|x| x.1.to_string()).unwrap_or(s.nota.clone()) } else { s.tema.clone() };
                            view! { <div class="tema"><span class="mono sesion-n">{format!("S{:02}", s.n)}</span>" "{t}</div> }
                        })
                        .collect_view();
                    let barras = grupos
                        .iter()
                        .map(|(k, g)| {
                            let d = g.iter().filter(|p| e.dominada(&p.id)).count();
                            let v = g.iter().filter(|p| e.tarjeta(&p.id).is_some()).count();
                            let n = g.len() as f64;
                            view! {
                                <div class="barra-sesion"><span>{k.clone()}</span>
                                    <span class="pista"><u style=format!("width:{}%", 100.0 * v as f64 / n)></u><i style=format!("width:{}%", 100.0 * d as f64 / n)></i></span>
                                    <span>{format!("{d}/{}", g.len())}</span></div>
                            }
                        })
                        .collect_view();
                    let (b2, resumen) = (boveda.clone(), u.resumen.clone());
                    let tiene_resumen = !u.resumen.is_empty();
                    view! {
                        <section class="panel unidad mb" style=format!("--c:{}", u.color)>
                            <div class="unidad-cab">
                                <div><div class="num">{format!("{} · {}", u.tipo.to_uppercase(), u.id)}</div><h3>{u.titulo.clone()}</h3></div>
                                <div class="mono tenue mini-txt">
                                    {format!("{}/{} DOMINADAS · {} PARA HOY", r.dominadas, r.total, r.pendientes)}
                                    {tiene_resumen.then(|| view! { " · " <button class="enlace-mono" on:click=move |_| plataforma::abrir(&uri_obsidian(&b2, &resumen))>"↗ RESUMEN GENERAL"</button> })}
                                </div>
                            </div>
                            <div class="sesiones">{casillas}</div>
                            <div class="rejilla-2 mt">
                                <div>{temas}</div>
                                <div>{barras}
                                    <div class="mono tenue mini-txt mt">"BARRA CLARA = VISTAS · BARRA BRILLANTE = DOMINADAS"</div>
                                    <div class="acciones mt">
                                        <a class="boton fantasma" href=format!("#/repaso?u={}", u.id)>{format!("Repasar {}", u.corto)}</a>
                                        <a class="boton fantasma" href=format!("#/temas?u={}", u.id)>"Elegir sesión"</a>
                                    </div>
                                </div>
                            </div>
                        </section>
                    }
                })
                .collect_view();
            Some(view! {
                {cabecera(
                    "SYS://ASIGNATURAS · SESIONES Y DOMINIO".into(),
                    "Asignaturas".into(),
                    vec!["CUADRO RELLENO = NOTA CON CONTENIDO".into(), "PULSA UNA SESIÓN PARA ABRIRLA EN OBSIDIAN".into()],
                )}
                {paneles}
            })
        })
    }
}
