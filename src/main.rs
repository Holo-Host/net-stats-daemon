mod keypair;
pub mod stats;

use anyhow::Result;
use env_logger;
use keypair::Keys;
use log::{debug, info};
use reqwest::Client;
use stats::Stats;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let hpos_key = Keys::new().await?;

    info!("Collecting payload from holoport");
    let payload = Stats::new(&hpos_key.pubkey_base36);
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
