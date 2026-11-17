//! Vuelca el contenido de la bóveda de Raúl a JSON (para comparar con Frost Study).
fn main() {
    let c = zerostudy_nucleo::boveda::leer(&zerostudy_nucleo::config::Config::raul(), "2026-11-17T00:00").unwrap();
    println!("{}", serde_json::to_string(&c).unwrap());
}
