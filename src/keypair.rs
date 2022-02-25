use crate::stats::Stats;
use anyhow::{Context, Result};
use base64::encode_config;
use ed25519_dalek::*;
use hpos_config_core::{public_key::to_base36_id, Config};
use serde_json;
use std::env;
use std::fs::File;

pub struct Keys {
    keypair: Keypair,
    pub pubkey_base36: String,
}

impl Keys {
    pub async fn new() -> Result<Self> {
        let keypair = keypair_from_config().await?;
        let pubkey_base36 = to_base36_id(&keypair.public);
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

async fn keypair_from_config() -> Result<Keypair> {
    let config_path = "/run/hpos-init/hp-*.json";
    let password = env::var("DEVICE_SEED_DEFAULT_PASSWORD")
        .context("Cannot read bundle password from env var")?;

    let config_file =
        File::open(&config_path).context(format!("Failed to open config file {}", config_path))?;

    match serde_json::from_reader(config_file)? {
        Config::V1 { seed, .. } => Keypair::from_bytes(&seed).context(format!(
            "Unable to read seed in config V1 file {}",
            config_path
        )),
        Config::V2 { device_bundle, .. } => {
            hpos_config_seed_bundle_explorer::unlock(&device_bundle, Some(password))
                .await
                .context(format!(
                    "Unable to unlock the device bundle from {}",
                    config_path
                ))
        }
    }
}
