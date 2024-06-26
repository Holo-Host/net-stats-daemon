use crate::stats::Stats;
use anyhow::{Context, Result};
use base64::encode_config;
use ed25519_dalek::*;
use hpos_config_core::{public_key::to_base36_id, Config};
use hpos_config_seed_bundle_explorer::holoport_key;
use std::env;
use std::fs::File;

pub struct Keys {
    keypair: SigningKey,
    pub pubkey_base36: String,
}

impl Keys {
    pub async fn new() -> Result<Self> {
        let keypair = keypair_from_config().await?;
        let pubkey_base36 = to_base36_id(&keypair.verifying_key());
        Ok(Self {
            keypair,
            pubkey_base36,
        })
    }

    pub async fn sign(&self, payload: &Stats) -> Result<String> {
        let signature = self
            .keypair
            .try_sign(&payload.into_bytes()?)
            .context("Failed to sign payload")?;
        Ok(encode_config(
            &signature.to_bytes()[..],
            base64::STANDARD_NO_PAD,
        ))
    }
}

async fn keypair_from_config() -> Result<SigningKey> {
    let config_path =
        env::var("HPOS_CONFIG_PATH").context("Cannot read HPOS_CONFIG_PATH from env var")?;

    let password = env::var("DEVICE_SEED_DEFAULT_PASSWORD")
        .context("Cannot read bundle password from env var")?;

    let config_file =
        File::open(&config_path).context(format!("Failed to open config file {}", config_path))?;

    let config: Config = serde_json::from_reader(config_file)
        .context(format!("Failed to read config from file {}", &config_path))?;

    holoport_key(&config, Some(password)).await.context(format!(
        "Failed to obtain holoport signing key from file {}",
        config_path
    ))
}
