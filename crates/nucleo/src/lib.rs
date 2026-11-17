//! Núcleo de ZeroStudy.
//!
//! Compila igual para el escritorio (Tauri) y para la web (WebAssembly), así que aquí no hay
//! relojes ni sistema de archivos: quien llama pasa la fecha de hoy. La única excepción es el
//! módulo [`boveda`], que lee las notas de Obsidian y solo existe con la feature `boveda`.

pub mod atlas;
pub mod config;
pub mod metodo;
pub mod modelo;
pub mod nota;
pub mod opciones;
pub mod progreso;
pub mod tema;

#[cfg(feature = "boveda")]
pub mod boveda;


pub use modelo::*;
pub use progreso::{Estado, Evento, Registro, Simulacro, Tarjeta};
