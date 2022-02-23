use crate::types::Stats;
use anyhow::{bail, Context, Result};
use ed25519_dalek::*;
use hpos_config_core::{public_key::to_base36_id, Config};
use serde_json;
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

    pub async fn sign(&self, payload: &Stats) -> Result<Signature> {
        self.keypair
            .try_sign(&payload.into_bytes()?)
            .context("Failed to sign payload")
    }
}

async fn keypair_from_config() -> Result<Keypair> {
    let config_path = "/a/b/c/config.toml";
    let password = "pass".to_owned();

    let config_file =
        File::open(&config_path).context(format!("Failed to open config file {}", config_path))?;

    match serde_json::from_reader(config_file)? {
        Config::V1 { seed, .. } => Keypair::from_bytes(&seed).context(format!(
            "Unable to read seed in config V1 file {}",
            config_path
        )),
        Config::V2 { device_bundle, .. } => {
            // FIXME: ugly as hell
            hpos_config_seed_bundle_explorer::unlock(&device_bundle, Some(password))
                .await
                .context(format!(
                    "Unable to unlock the device bundle from {}",
                    config_path
                ))
        }
        _ => {
            bail!("Wrong config version in {}", config_path)
        }
    }
}
