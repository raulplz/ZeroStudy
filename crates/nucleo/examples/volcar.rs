//! Vuelca el contenido de una bóveda a JSON a partir de un archivo de configuración.
//! Uso: `cargo run -p zerostudy-nucleo --features boveda --example volcar -- config.json`
fn main() {
    let ruta = std::env::args().nth(1).expect("uso: volcar <config.json>");
    let cfg: zerostudy_nucleo::config::Config =
        serde_json::from_str(&std::fs::read_to_string(ruta).expect("no puedo leer el archivo")).expect("JSON no válido");
    let c = zerostudy_nucleo::boveda::leer(&cfg, "2026-11-17T00:00").unwrap();
    println!("{}", serde_json::to_string(&c).unwrap());
}
