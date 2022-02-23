mod keypair;
pub mod stats;

use anyhow::Result;
use env_logger;
use keypair::Keys;
use log::info;
use reqwest::Client;
use stats::Stats;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let hpos_key = Keys::new().await?;

    info!("Collecting payload from holoport");
    let payload = Stats::new(&hpos_key.pubkey_base36);

    let signature = hpos_key.sign(&payload).await?;
    println!("Result: '{:?}'", payload); // TODO: debug! for payload and signature and response status

    info!("Sending statistics to server");
    let client = Client::new();
    let res = client
        .post("http://httpbin.org/post") // TODO: update endpoint
        .json(&payload)
        .header("x-hpos-signature", signature)
        .send()
        .await?;

    println!("{:?}", res);

    Ok(())
}
