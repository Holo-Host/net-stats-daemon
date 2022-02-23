mod keypair;
pub mod types;

use anyhow::Result;
use env_logger;
use keypair::Keys;
use log::info;
use types::Stats;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let hpos_key = Keys::new().await?;

    info!("Collecting payload from holoport");
    let payload = Stats::new(&hpos_key.pubkey_base36);

    let signature = hpos_key.sign(&payload).await?;
    println!("Result: '{:?}'", payload);

    Ok(())
}
