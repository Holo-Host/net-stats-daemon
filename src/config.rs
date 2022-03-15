// NOTE: Most of the below code is a copy of the types, traits, and fns from  holo-auto-installer.
// If continue with installed_app_map endpoint / approach, then publish module for holo-auto-installer and import relevant code.
#![allow(clippy::unit_arg)]
use super::websocket::AppWebsocket;

use log::{debug, info};
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::{env, path::PathBuf};
use url::Url;

use hc_utils::{WrappedAgentPubKey, WrappedHeaderHash};
use holochain::conductor::api::ZomeCall;
use holochain::conductor::api::{AppResponse, InstalledAppInfo};
use holochain_types::app::InstalledAppId;
use holochain_types::prelude::{zome_io::ExternIO, FunctionName, ZomeName};

use holofuel_types::fuel::Fuel;

pub fn configure_holochain_yaml() -> String {
    match env::var("CONFIGURE_HOLOCHAIN_YAML_PATH") {
        Ok(path) => path,
        _ => "/var/lib/configure-holochain/config.yaml".to_string(),
    }
}
#[derive(Deserialize, Debug, Clone)]
pub struct DnaResource {
    pub hash: String, // hash of the dna, not a stored dht address
    pub src_url: String,
    pub nick: String,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug, Clone)]
pub struct HostingPrices {
    cpu: Fuel,
    storage: Fuel,
    bandwidth: Fuel,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug, Clone)]
pub struct LoginConfig {
    require_joining_code: bool,
    display_publisher_name: bool,
    help_url: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PresentedHappBundle {
    pub id: WrappedHeaderHash,
    pub provider_pubkey: WrappedAgentPubKey,
    pub is_draft: bool,
    pub is_clone: bool,
    pub bundle_url: String,
    pub ui_src_url: String,
    pub dnas: Vec<DnaResource>,
    pub hosted_url: String,
    pub name: String,
    pub logo_url: String,
    pub description: String,
    pub categories: Vec<String>,
    pub jurisdictions: Vec<String>,
    pub hosting_prices: HostingPrices,
    pub login_config: LoginConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Happ {
    pub bundle_url: Option<Url>,
    pub bundle_path: Option<PathBuf>,
    pub ui_url: Option<Url>,
    pub ui_path: Option<PathBuf>,
}

impl Happ {
    pub fn id(&self) -> String {
        let name = if let Some(ref bundle) = self.bundle_path {
            bundle
                .file_name()
                .unwrap()
                .to_os_string()
                .to_string_lossy()
                .to_string()
        } else if let Some(ref bundle) = self.bundle_url {
            bundle.path_segments().unwrap().last().unwrap().to_string()
        } else {
            "unreabable".to_string()
        };
        if let Ok(uid) = env::var("DEV_UID_OVERRIDE") {
            format!("{}::{}", name.replace(".happ", "").replace(".", ":"), uid)
        } else {
            name.replace(".happ", "").replace(".", ":")
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct HappsFile {
    pub self_hosted_happs: Vec<Happ>,
    pub core_happs: Vec<Happ>,
}

impl HappsFile {
    pub fn find_core_app(self) -> Option<Happ> {
        let core_app = &self
            .core_happs
            .into_iter()
            .find(|x| x.id().contains("core-app"));
        core_app.clone()
    }
}

pub async fn get_all_hha_happs(
    mut app_websocket: AppWebsocket,
    core_hha_id: InstalledAppId,
) -> Result<Vec<WrappedHeaderHash>> {
    match app_websocket.get_app_info(core_hha_id).await {
        Some(InstalledAppInfo { cell_data, .. }) => {
            let zome_call_payload = ZomeCall {
                cell_id: cell_data[0].as_id().clone(),
                zome_name: ZomeName::from("hha"),
                fn_name: FunctionName::from("get_happs"),
                payload: ExternIO::encode(())?,
                cap_secret: None,
                provenance: cell_data[0].clone().into_id().into_dna_and_agent().1,
            };
            let response = app_websocket.zome_call(zome_call_payload).await?;
            match response {
                AppResponse::ZomeCall(r) => {
                    info!("ZomeCall Response - Hosted happs List {:?}", r);
                    let happ_bundles: Vec<PresentedHappBundle> =
                        rmp_serde::from_read_ref(r.as_bytes())?;
                    let happ_bundle_ids = happ_bundles.into_iter().map(|happ| happ.id).collect();
                    Ok(happ_bundle_ids)
                }
                _ => Err(anyhow!("unexpected response: {:?}", response)),
            }
        }
        None => Err(anyhow!("Core happ HHA is not installed")),
    }
}

pub fn load_happ_file(path: impl AsRef<Path>) -> Result<HappsFile> {
    use std::fs::File;
    let file = File::open(path).context("failed to open file")?;
    let happ_file =
        serde_yaml::from_reader(&file).context("failed to deserialize YAML as HappsFile")?;
    debug!("YAML Happs File: {:?}", happ_file);
    Ok(happ_file)
}
