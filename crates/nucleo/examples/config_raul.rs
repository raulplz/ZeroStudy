//! Imprime la configuración de Raúl como JSON (`--configurado` la marca como ya configurada).
fn main() {
    let mut c = zerostudy_nucleo::config::Config::raul();
    c.configurado = std::env::args().any(|a| a == "--configurado");
    println!("{}", serde_json::to_string_pretty(&c).unwrap());
}
