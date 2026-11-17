//! ZeroStudy · interfaz (Leptos CSR). La misma app corre dentro de Tauri en el escritorio y
//! como PWA en el iPhone o cualquier navegador.

mod app;
mod plataforma;
mod repaso;
mod util;
mod vistas;

fn main() {
    leptos::mount::mount_to_body(app::App);
}
