//! Estado global, carga, tema y armazón (barra lateral + vista según la ruta).

use leptos::ev;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::time::Duration;
use zerostudy_nucleo::config::Config;
use zerostudy_nucleo::tema::Tema;
use zerostudy_nucleo::{Contenido, Estado, Registro, Simulacro};

use crate::plataforma;
use crate::repaso;
use crate::util::{self, ahora, hoy};
use crate::vistas;

#[derive(Clone, Copy)]
pub struct Ctx {
    pub contenido: RwSignal<Option<Contenido>>,
    pub registros: RwSignal<Vec<Registro>>,
    pub estado: Memo<Estado>,
    pub config: RwSignal<Config>,
    pub tema: RwSignal<Tema>,
    pub guardado: RwSignal<String>,
    pub ruta: RwSignal<String>,
    pub repaso: RwSignal<Option<repaso::Sesion>>,
    pub cfg_repaso: RwSignal<repaso::Cfg>,
    pub toast: RwSignal<String>,
    pub cargado: RwSignal<bool>,
    pub error: RwSignal<Option<String>>,
    pub sugerencia: RwSignal<String>,
    pub cambios: StoredValue<u32>,
}

pub fn ctx() -> Ctx {
    expect_context::<Ctx>()
}

impl Ctx {
    pub fn metodo(&self, id: &str) -> bool {
        self.config.with(|c| c.metodos.iter().any(|m| m == id))
    }

    pub fn aviso(&self, txt: impl Into<String>) {
        let t = self.toast;
        t.set(txt.into());
        set_timeout(move || t.set(String::new()), Duration::from_millis(2600));
    }

    fn propio_mut(v: &mut Vec<Registro>, dispositivo: &str) -> usize {
        match v.iter().position(|r| r.dispositivo == dispositivo) {
            Some(i) => i,
            None => {
                v.push(Registro::nuevo(dispositivo));
                v.len() - 1
            }
        }
    }

    pub fn propio(&self) -> Option<Registro> {
        let d = self.config.with_untracked(|c| c.dispositivo.clone());
        self.registros.with_untracked(|v| v.iter().find(|r| r.dispositivo == d).cloned())
    }

    /// 1 fallé · 2 dudé · 3 lo sabía.
    pub fn calificar(&self, pregunta: &str, nota: u8) {
        let d = self.config.with_untracked(|c| c.dispositivo.clone());
        self.registros.update(|v| {
            let i = Self::propio_mut(v, &d);
            v[i].anotar(&ahora(), pregunta, nota);
        });
        self.persistir();
    }

    pub fn registrar_simulacro(&self, s: Simulacro) {
        let d = self.config.with_untracked(|c| c.dispositivo.clone());
        self.registros.update(|v| {
            let i = Self::propio_mut(v, &d);
            v[i].simulacros.push(s);
        });
        self.persistir();
    }

    /// Guarda unos segundos después del último cambio (en la web cada guardado es una subida a GitHub).
    pub fn persistir(&self) {
        let n = self.cambios.get_value() + 1;
        self.cambios.set_value(n);
        self.guardado.set("GUARDANDO…".into());
        let c = *self;
        let espera = if plataforma::escritorio() { 600 } else { 4000 };
        set_timeout(
            move || {
                if c.cambios.get_value() != n {
                    return;
                }
                let (Some(propio), cfg, todos) = (c.propio(), c.config.get_untracked(), c.registros.get_untracked()) else { return };
                spawn_local(async move {
                    let txt = match plataforma::guardar_registro(&cfg, &propio, &todos).await {
                        Ok(t) => format!("● {t} {}", &ahora()[11..16]),
                        Err(e) => format!("⚠ {e}"),
                    };
                    c.guardado.set(txt);
                });
            },
            Duration::from_millis(espera),
        );
    }

    /// Aplica el progreso de otros dispositivos sin pisar el propio, que puede tener respuestas
    /// todavía sin guardar.
    pub fn aplicar_ajenos(&self, nuevos: Vec<Registro>) {
        let d = self.config.with_untracked(|c| c.dispositivo.clone());
        self.registros.update(|v| {
            let propio = v.iter().find(|r| r.dispositivo == d).cloned();
            *v = nuevos.into_iter().filter(|r| r.dispositivo != d).collect();
            v.extend(propio);
        });
    }

    pub fn sincronizar(&self) {
        let c = *self;
        c.guardado.set("SINCRONIZANDO…".into());
        spawn_local(async move {
            let cfg = c.config.get_untracked();
            match plataforma::sincronizar(&cfg, c.propio().as_ref()).await {
                Ok(s) => {
                    if !s.registros.is_empty() || plataforma::escritorio() {
                        c.aplicar_ajenos(s.registros);
                    }
                    if let Some(x) = s.contenido {
                        c.contenido.set(Some(x));
                    }
                    c.guardado.set(format!("● {}", s.estado));
                }
                Err(e) => c.guardado.set(format!("⚠ {e}")),
            }
        });
    }

    pub fn recargar(&self) {
        let c = *self;
        spawn_local(async move {
            match plataforma::cargar().await {
                Ok(carga) => {
                    c.config.set(carga.config);
                    c.contenido.set(carga.contenido);
                    c.registros.set(carga.registros);
                    if let Some(t) = carga.tema {
                        c.tema.set(t);
                    }
                    c.sugerencia.set(carga.sugerencia);
                    c.error.set(carga.aviso);
                }
                Err(e) => c.error.set(Some(e)),
            }
            c.cargado.set(true);
            if c.config.with_untracked(|x| x.configurado && !x.sync.repo.is_empty()) {
                c.sincronizar();
            }
        });
    }
}

#[component]
pub fn App() -> impl IntoView {
    let registros = RwSignal::new(Vec::<Registro>::new());
    let c = Ctx {
        contenido: RwSignal::new(None),
        registros,
        estado: Memo::new(move |_| registros.with(|r| Estado::desde(r))),
        config: RwSignal::new(Config::default()),
        tema: RwSignal::new(Tema::default()),
        guardado: RwSignal::new("—".into()),
        ruta: RwSignal::new(util::ruta_actual()),
        repaso: RwSignal::new(None),
        cfg_repaso: RwSignal::new(repaso::Cfg::default()),
        toast: RwSignal::new(String::new()),
        cargado: RwSignal::new(false),
        error: RwSignal::new(None),
        sugerencia: RwSignal::new(String::new()),
        cambios: StoredValue::new(0),
    };
    provide_context(c);

    // ruta
    let _ = window_event_listener(ev::hashchange, move |_| {
        let r = util::ruta_actual();
        if !r.starts_with("repaso") {
            c.repaso.set(None);
        }
        c.ruta.set(r);
    });
    // teclado del repaso
    let _ = window_event_listener(ev::keydown, move |e| repaso::tecla(c, e));
    // tema: variables CSS en vivo
    Effect::new(move |_| {
        let t = c.tema.get();
        if let Some(raiz) = document().document_element() {
            let raiz: web_sys::HtmlElement = wasm_bindgen::JsCast::unchecked_into(raiz);
            let estilo = raiz.style();
            for (k, v) in t.variables_css() {
                let _ = estilo.set_property(k, &v);
            }
        }
    });
    plataforma::escuchar::<Tema>("tema", move |t| c.tema.set(t));
    plataforma::escuchar::<Vec<Registro>>("registros", move |r| c.aplicar_ajenos(r));
    plataforma::escuchar::<String>("ruta", move |r| util::ir(&r));
    // sincroniza cada 5 minutos si hay repo
    set_interval(
        move || {
            if c.config.with_untracked(|x| x.configurado && !x.sync.repo.is_empty()) {
                c.sincronizar();
            }
        },
        Duration::from_secs(300),
    );
    c.recargar();

    view! {
        <div class="fondo-rejilla" aria-hidden="true"></div>
        <div class="kanji" aria-hidden="true">{move || c.tema.with(|t| t.kanji.clone())}</div>
        {move || {
            if !c.cargado.get() {
                view! { <main class="vista"><div class="cargando mono">"CONECTANDO CON TUS APUNTES…"</div></main> }.into_any()
            } else if !c.config.with(|x| x.configurado) {
                view! { <vistas::inicio::Inicio/> }.into_any()
            } else {
                view! { <Armazon/> }.into_any()
            }
        }}
        <div class="toast" class:ver=move || !c.toast.with(String::is_empty)>{move || c.toast.get()}</div>
    }
}

#[component]
fn Armazon() -> impl IntoView {
    let c = ctx();
    let vista = move || c.ruta.with(|r| r.split('?').next().unwrap_or("panel").to_string());
    let enlace = move |id: &'static str, nombre: &'static str, icono: &'static str| {
        view! {
            <a href=format!("#/{id}") class:activa=move || vista() == id>
                <span class="icono" inner_html=icono></span>
                <span>{nombre}</span>
                {(id == "repaso").then(|| view! {
                    <b class="insignia">{move || {
                        let n = c.contenido.with(|x| x.as_ref().map(|x| c.estado.with(|e| e.resumen(x, &hoy(), |_| true).pendientes)).unwrap_or(0));
                        if n > 0 { n.to_string() } else { String::new() }
                    }}</b>
                })}
            </a>
        }
    };
    view! {
        <aside class="lateral">
            <a class="marca" href="#/panel" title="ZeroStudy">
                <span class="marca-logo">"ZER"<span>"Ø"</span>"STUDY"</span>
                <span class="marca-sub">{move || format!("SYS://{} · {}", c.tema.with(|t| t.nombre.to_uppercase()), c.config.with(|x| x.dispositivo.to_uppercase()))}</span>
            </a>
            <nav>
                {enlace("panel", "Panel", ICONO_PANEL)}
                {enlace("temas", "Elegir tema", ICONO_TEMAS)}
                {enlace("repaso", "Repaso", ICONO_REPASO)}
                {enlace("atlas", "Atlas", ICONO_ATLAS)}
                {enlace("unidades", "Asignaturas", ICONO_UNIDADES)}
                {enlace("ajustes", "Ajustes", ICONO_AJUSTES)}
            </nav>
            {move || c.metodo("pomodoro").then(|| view! { <vistas::pomodoro::Pomodoro/> })}
            <div class="lateral-pie">
                <div class="mono tenue estado-guardado">{move || c.guardado.get()}</div>
                {move || (!c.config.with(|x| x.sync.repo.is_empty())).then(|| view! {
                    <button class="enlace-mono" on:click=move |_| c.sincronizar()>"SINCRONIZAR"</button>
                })}
                <div class="mono tenue">"◢ ZERØ / " {move || c.config.with(|x| x.dispositivo.to_uppercase())}</div>
            </div>
        </aside>
        <main class="vista" tabindex="-1">
            {move || c.error.get().map(|e| view! { <div class="panel aviso">"⚠ " {e}</div> })}
            {move || {
                if c.contenido.with(Option::is_none) {
                    return view! { <div class="vacio">"Todavía no hay apuntes cargados. Revisa la carpeta o la sincronización en Ajustes."</div> }.into_any();
                }
                match vista().as_str() {
                    "repaso" => view! { <repaso::VistaRepaso/> }.into_any(),
                    "atlas" => view! { <vistas::atlas::Atlas/> }.into_any(),
                    "unidades" => view! { <vistas::unidades::Unidades/> }.into_any(),
                    "temas" => view! { <vistas::temas::Temas/> }.into_any(),
                    "ajustes" => view! { <vistas::ajustes::Ajustes/> }.into_any(),
                    _ => view! { <vistas::panel::Panel/> }.into_any(),
                }
            }}
        </main>
    }
}

const ICONO_PANEL: &str = r#"<svg viewBox="0 0 24 24"><path d="M3 3h8v8H3zM13 3h8v5h-8zM13 10h8v11h-8zM3 13h8v8H3z"/></svg>"#;
const ICONO_TEMAS: &str = r#"<svg viewBox="0 0 24 24"><circle cx="11" cy="11" r="6.5"/><path d="M16 16l5 5M8 11h6M11 8v6"/></svg>"#;
const ICONO_REPASO: &str = r#"<svg viewBox="0 0 24 24"><path d="M4 5h12l4 4v10H4z"/><path d="M8 12h8M8 16h5"/></svg>"#;
const ICONO_ATLAS: &str = r#"<svg viewBox="0 0 24 24"><circle cx="6" cy="6" r="2.5"/><circle cx="18" cy="8" r="2.5"/><circle cx="9" cy="18" r="2.5"/><circle cx="19" cy="19" r="1.8"/><path d="M8 7l8 1M7.5 8l1 7.5M11 17l6-7.5M11.4 18.3l5.8.6"/></svg>"#;
const ICONO_UNIDADES: &str = r#"<svg viewBox="0 0 24 24"><path d="M4 4h7v7H4zM13 4h7v7h-7zM4 13h7v7H4zM13 13h7v7h-7z"/></svg>"#;
const ICONO_AJUSTES: &str = r#"<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="3"/><path d="M12 2v3M12 19v3M2 12h3M19 12h3M4.9 4.9l2.1 2.1M17 17l2.1 2.1M4.9 19.1L7 17M17 7l2.1-2.1"/></svg>"#;
