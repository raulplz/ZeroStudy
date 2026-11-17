//! Ajustes: metodologías, dispositivo, sincronización, carpeta de apuntes y tema.

use leptos::prelude::*;
use leptos::task::spawn_local;
use zerostudy_nucleo::metodo::{self, Objetivo};

use crate::app::{ctx, Ctx};
use crate::plataforma;
use crate::util::hoy;
use crate::vistas::cabecera;
use crate::vistas::inicio::{limpiar_dispositivo, limpiar_repo, selector_objetivo};
use crate::vistas::metodos::Metodos;

fn guardar(c: Ctx, aviso: &'static str) {
    let k = c.config.get_untracked();
    spawn_local(async move {
        match plataforma::guardar_config(&k).await {
            Ok(()) => c.aviso(aviso),
            Err(e) => c.aviso(format!("No se pudo guardar: {e}")),
        }
    });
}

#[component]
pub fn Ajustes() -> impl IntoView {
    let c = ctx();
    let web = !plataforma::escritorio();
    let token = RwSignal::new(plataforma::token());
    let analisis = Memo::new(move |_| c.contenido.with(|x| x.as_ref().map(|x| metodo::analizar(x, &hoy())).unwrap_or_default()));
    let objetivo = Signal::derive(move || c.config.with(|k| k.objetivo));
    let activos = Signal::derive(move || c.config.with(|k| k.metodos.clone()));

    view! {
        {cabecera("SYS://AJUSTES".into(), "Ajustes".into(), vec![format!("DISPOSITIVO · {}", c.config.with_untracked(|k| k.dispositivo.to_uppercase()))])}

        <h2 class="seccion"><b>"01"</b>"Cómo estudias"</h2>
        {selector_objetivo(objetivo, Callback::new(move |o: Objetivo| { c.config.update(|k| k.objetivo = o); guardar(c, "Objetivo guardado"); }))}
        <div class="mt"></div>
        <Metodos analisis=analisis objetivo=objetivo activos=activos
            alternar=Callback::new(move |id: String| {
                c.config.update(|k| if let Some(p) = k.metodos.iter().position(|m| *m == id) { k.metodos.remove(p); } else { k.metodos.push(id); });
                guardar(c, "Metodologías guardadas");
            })/>

        <h2 class="seccion"><b>"02"</b>"Dispositivo y sincronización"</h2>
        <section class="panel ajuste">
            <label class="campo"><span class="mono">"NOMBRE DE ESTE DISPOSITIVO"</span>
                <input prop:value=move || c.config.with(|k| k.dispositivo.clone())
                    on:change=move |e| {
                        let v = limpiar_dispositivo(&event_target_value(&e));
                        c.config.update(|k| k.dispositivo = v);
                        guardar(c, "Dispositivo guardado");
                    }/></label>
            <label class="campo"><span class="mono">"REPO PRIVADO DE DATOS"</span>
                <input placeholder="usuario/repo (vacío = sin sincronizar)" prop:value=move || c.config.with(|k| k.sync.repo.clone())
                    on:change=move |e| {
                        let v = limpiar_repo(&event_target_value(&e));
                        c.config.update(|k| k.sync.repo = v);
                        guardar(c, "Repo guardado");
                    }/></label>
            {web.then(|| view! {
                <label class="campo"><span class="mono">"TOKEN DE GITHUB (SOLO EN ESTE NAVEGADOR)"</span>
                    <input type="password" autocomplete="off" placeholder="github_pat_…" prop:value=move || token.get()
                        on:input=move |e| token.set(event_target_value(&e))
                        on:change=move |_| { plataforma::guardar_token(&token.get_untracked()); c.aviso("Token guardado en este navegador"); }/></label>
            })}
            <div class="acciones">
                <button class="boton" disabled=move || c.config.with(|k| k.sync.repo.is_empty()) on:click=move |_| c.sincronizar()>"Sincronizar ahora"</button>
                <span class="mono tenue">{move || c.guardado.get()}</span>
            </div>
        </section>

        {(!web).then(|| view! {
            <h2 class="seccion"><b>"03"</b>"Apuntes"</h2>
            <section class="panel ajuste">
                <label class="campo"><span class="mono">"CARPETA DE APUNTES"</span>
                    <input prop:value=move || c.config.with(|k| k.boveda.clone())
                        on:change=move |e| { c.config.update(|k| k.boveda = event_target_value(&e).trim().to_string()); }/></label>
                <div class="acciones">
                    <button class="boton fantasma" on:click=move |_| {
                        let k = c.config.get_untracked();
                        spawn_local(async move {
                            match plataforma::guardar_config(&k).await {
                                Ok(()) => { c.recargar(); c.aviso("Apuntes releídos"); }
                                Err(e) => c.aviso(format!("No se pudo guardar: {e}")),
                            }
                        });
                    }>"Guardar y releer"</button>
                </div>
            </section>
        })}

        <h2 class="seccion"><b>{if web { "03" } else { "04" }}</b>"Tema y asistente"</h2>
        <section class="panel ajuste">
            <div class="tema-activo">
                <div class="muestras">
                    {move || c.tema.with(|t| {
                        let k = &t.colores;
                        [&k.fondo, &k.superficie, &k.acento, &k.acento2, &k.ok, &k.aviso, &k.peligro].into_iter()
                            .map(|v| view! { <i style=format!("background:{v}")></i> }).collect_view()
                    })}
                </div>
                <div><b>{move || c.tema.with(|t| t.nombre.clone())}</b>
                    <div class="mono tenue">{if web { "EL TEMA SE ADAPTA AL DE TU ESCRITORIO CUANDO ABRES LA APP DE ESCRITORIO" } else { "LO ELIGE ZEROLAYER: «zerolayer theme»; ZEROSTUDY LO SIGUE EN VIVO" }}</div></div>
            </div>
            <div class="acciones">
                <button class="boton fantasma" on:click=move |_| {
                    c.config.update(|k| k.configurado = false);
                    guardar(c, "Asistente reiniciado");
                }>"Repetir asistente"</button>
            </div>
        </section>
    }
}
