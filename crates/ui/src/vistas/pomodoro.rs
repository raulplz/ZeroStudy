use leptos::prelude::*;
use std::time::Duration;

/// Temporizador Pomodoro (25 min de foco, 5 de descanso) en la barra lateral.
#[component]
pub fn Pomodoro() -> impl IntoView {
    let foco = RwSignal::new(true);
    let restante = RwSignal::new(25 * 60u32);
    let corriendo = RwSignal::new(false);
    let h = set_interval_with_handle(
        move || {
            if !corriendo.get_untracked() {
                return;
            }
            if restante.get_untracked() <= 1 {
                let f = !foco.get_untracked();
                foco.set(f);
                restante.set(if f { 25 * 60 } else { 5 * 60 });
            } else {
                restante.update(|r| *r -= 1);
            }
        },
        Duration::from_secs(1),
    )
    .ok();
    on_cleanup(move || {
        if let Some(h) = h {
            h.clear()
        }
    });
    view! {
        <div class="pomodoro" class:descanso=move || !foco.get()>
            <div class="k">{move || if foco.get() { "◔ FOCO" } else { "◔ DESCANSO" }}</div>
            <div class="t">{move || { let r = restante.get(); format!("{:02}:{:02}", r / 60, r % 60) }}</div>
            <div class="b">
                <button on:click=move |_| corriendo.update(|c| *c = !*c)>{move || if corriendo.get() { "Pausa" } else { "Iniciar" }}</button>
                <button on:click=move |_| { corriendo.set(false); foco.set(true); restante.set(25 * 60); }>"Reiniciar"</button>
            </div>
        </div>
    }
}
