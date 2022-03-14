pub mod app_health;
pub mod config;
mod keypair;
pub mod stats;
pub mod websocket;

use anyhow::Result;
use env_logger;
use keypair::Keys;
use log::{debug, info};
use reqwest::Client;
use stats::Stats;

use config::{load_happ_file, Config};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let hpos_key = Keys::new().await?;

    info!("Collecting core apps from holoport config");
    let config = Config::load();
    let happ_file = load_happ_file(&config.happs_file_path)?;
    let core_happ_id = match happ_file.find_core_app() {
        Some(core_happ) => Some(core_happ.id()),
        None => {
            // throw warning -> cannot find core happ (hha)
            None
        }
    };

    info!("Collecting payload from holoport");
    let payload = Stats::new(&hpos_key.pubkey_base36, core_happ_id).await;
    debug!("Payload: '{:?}'", &payload);

    let signature = hpos_key.sign(&payload).await?;
    debug!("Signature: '{:?}'", &signature);

    info!("Sending statistics to server");
    let client = Client::new();
    let res = client
        .post("https://network-statistics.holo.host/hosts/stats")
        .json(&payload)
        .header("x-hpos-signature", signature)
        .send()
        .await?;

    debug!("API response: {:?}", res);
    Ok(())
}
