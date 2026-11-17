pub mod ajustes;
pub mod atlas;
pub mod inicio;
pub mod metodos;
pub mod panel;
pub mod pomodoro;
pub mod temas;
pub mod unidades;

use leptos::prelude::*;

/// Anillo de progreso (0–1).
pub fn anillo(pct: f64, color: String) -> impl IntoView {
    let r = 32.0;
    let l = 2.0 * std::f64::consts::PI * r;
    view! {
        <svg viewBox="0 0 78 78" class="anillo-svg">
            <circle cx="39" cy="39" r="32" fill="none" stroke="var(--linea)" stroke-width="6"/>
            <circle cx="39" cy="39" r="32" fill="none" stroke=color.clone() stroke-width="6"
                stroke-dasharray=format!("{} {l}", l * pct.clamp(0.0, 1.0)) style=format!("filter:drop-shadow(0 0 5px {color})")/>
        </svg>
    }
}

/// Cabecera de una vista.
pub fn cabecera(pre: String, titulo: String, lado: Vec<String>) -> impl IntoView {
    view! {
        <header class="cabecera">
            <div><div class="pre">{pre}</div><h1>{titulo}</h1></div>
            <div class="lado">{lado.into_iter().map(|l| view! { <div>{l}</div> }).collect_view()}</div>
        </header>
    }
}
