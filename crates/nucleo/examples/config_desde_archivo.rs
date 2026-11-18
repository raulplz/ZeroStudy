//! Lee una configuración (archivo JSON, por ejemplo una plantilla) y la imprime normalizada.
//! Uso: `cargo run -p zerostudy-nucleo --example config_desde_archivo -- ruta.json [--configurado]`
fn main() {
    let ruta = std::env::args().nth(1).expect("uso: config_desde_archivo <config.json> [--configurado]");
    let texto = std::fs::read_to_string(&ruta).expect("no puedo leer el archivo");
    let mut c: zerostudy_nucleo::config::Config = serde_json::from_str(&texto).expect("JSON no válido");
    c.configurado = std::env::args().any(|a| a == "--configurado");
    println!("{}", serde_json::to_string_pretty(&c).unwrap());
}
