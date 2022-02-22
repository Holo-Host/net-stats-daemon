mod types;

use env_logger;
use log::info;
use types::Stats;

fn main() {
    env_logger::init();

    info!("Collecting payload from holoport");
    let payload = Stats::new();

    println!("Result: '{:?}'", payload);
}
