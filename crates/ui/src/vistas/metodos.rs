//! Tarjetas del catálogo de metodologías con su recomendación. Se reutilizan en el asistente
//! de primer uso y en Ajustes.

use leptos::prelude::*;
use zerostudy_nucleo::metodo::{self, Analisis, Evidencia, Objetivo};

#[component]
pub fn Metodos(
    /// Rasgos del material (de `metodo::analizar`).
    #[prop(into)]
    analisis: Signal<Analisis>,
    #[prop(into)] objetivo: Signal<Objetivo>,
    /// Ids activos.
    #[prop(into)]
    activos: Signal<Vec<String>>,
    /// Se llama con el id al pulsar el conmutador.
    #[prop(into)]
    alternar: Callback<String>,
) -> impl IntoView {
    move || {
        let recs = metodo::recomendar(&analisis.get(), objetivo.get());
        view! {
            <div class="metodos">
                {recs.into_iter().enumerate().filter_map(|(i, r)| {
                    let m = metodo::metodo(r.id)?;
                    let id = r.id.to_string();
                    let id2 = id.clone();
                    let id4 = id.clone();
                    let evidencia = match m.evidencia {
                        Evidencia::Alta => "alta",
                        Evidencia::Moderada => "moderada",
                        Evidencia::Organizativa => "org",
                    };
                    Some(view! {
                        <article class="panel metodo" class:on=move || activos.with(|a| a.contains(&id))>
                            <div class="metodo-cab">
                                <span class="metodo-icono">{m.icono}</span>
                                <h3>{m.nombre}</h3>
                                {(i < 3).then(|| view! { <span class="etiqueta recomendada">"Recomendada"</span> })}
                                <button class="conmutador" role="switch"
                                    aria-label=format!("Activar {}", m.nombre)
                                    class:on=move || activos.with(|a| a.contains(&id2))
                                    aria-checked=move || if activos.with(|a| a.contains(&id4)) { "true" } else { "false" }
                                    on:click={let id3 = r.id.to_string(); move |_| alternar.run(id3.clone())}><i></i></button>
                            </div>
                            <p class="metodo-res">{m.resumen}</p>
                            <div class="metodo-meta">
                                <span class=format!("etiqueta ev-{evidencia}")>{m.evidencia.texto()}</span>
                                <span class="mono tenue">{format!("ENCAJE {}", r.puntos)}</span>
                            </div>
                            {(!r.motivos.is_empty()).then(|| view! {
                                <ul class="motivos">{r.motivos.iter().map(|t| view! { <li>{t.clone()}</li> }).collect_view()}</ul>
                            })}
                            <div class="metodo-app"><b class="mono">"EN LA APP"</b>" "{m.en_la_app}</div>
                            <div class="metodo-app tenue"><b class="mono">"IDEAL PARA"</b>" "{m.ideal_para}</div>
                        </article>
                    })
                }).collect_view()}
            </div>
        }
    }
}
