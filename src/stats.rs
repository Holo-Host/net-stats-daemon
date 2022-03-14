use anyhow::{Context, Result};
use holochain::conductor::api::AppStatusFilter;
use holochain_types::app::InstalledAppId;
use log::warn;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use subprocess::{CaptureData, Exec, Result as PopenResult};

use super::app_health;

pub struct EnabledAppStats {
    pub read_only: Option<Vec<InstalledAppId>>,
    pub sl: Option<Vec<InstalledAppId>>,
    pub core: Option<Vec<InstalledAppId>>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    holo_network: Option<String>,
    channel: Option<String>,
    holoport_model: Option<String>,
    ssh_status: Option<bool>,
    zt_ip: Option<String>,
    wan_ip: Option<String>,
    holoport_id: Option<String>,
    timestamp: Option<u32>,
    // either
    hpos_app_health_map: Option<HashMap<InstalledAppId, AppStatusFilter>>,
    // or
    running_read_only_happs: Option<Vec<InstalledAppId>>,
    running_sl_happs: Option<Vec<InstalledAppId>>,
    running_core_happs: Option<Vec<InstalledAppId>>,
    installed_app_map: Option<HashMap<InstalledAppId, i32>>,
}

impl Stats {
    pub async fn new(pubkey_base36: &str, core_hha_id: Option<InstalledAppId>) -> Self {
        let running_apps = get_running_apps().await;
        Self {
            holo_network: wrap(get_network()),
            channel: wrap(get_channel()),
            holoport_model: wrap(get_holoport_model()),
            ssh_status: string_2_bool(wrap(get_ssh_status())),
            zt_ip: wrap(get_zt_ip()),
            wan_ip: wrap(get_wan_ip()),
            holoport_id: Some(pubkey_base36.to_owned()),
            timestamp: None,
            hpos_app_health_map: get_hpos_app_health().await,
            running_read_only_happs: running_apps.read_only,
            running_sl_happs: running_apps.sl,
            running_core_happs: running_apps.core,
            installed_app_map: match core_hha_id {
                Some(core_hha_id) => get_installed_app_map(core_hha_id).await,
                None => None,
            },
        }
    }

    pub fn into_bytes(&self) -> Result<Vec<u8>> {
        serde_json::to_vec(&self).context("Failed to convert payload to bytes")
    }
}

type ExecResult = (&'static str, PopenResult<CaptureData>);

async fn get_hpos_app_health() -> Option<HashMap<InstalledAppId, AppStatusFilter>> {
    match app_health::get_hpos_app_health().await {
        Ok(data) => Some(data),
        Err(e) => {
            warn!("Failed when calling `get_hpos_app_health`: {:?}", e);
            return None;
        }
    }
}

async fn get_running_apps() -> EnabledAppStats {
    match app_health::get_running_apps().await {
        Ok(data) => data,
        Err(e) => {
            warn!("Failed when calling `get_running_apps`: {:?}", e);
            return EnabledAppStats {
                read_only: None,
                sl: None,
                core: None,
            };
        }
    }
}

async fn get_installed_app_map(
    core_hha_id: InstalledAppId,
) -> Option<HashMap<InstalledAppId, i32>> {
    match app_health::get_installed_app_map(core_hha_id).await {
        Ok(data) => Some(data),
        Err(e) => {
            warn!("Failed when calling `get_installed_app_map`: {:?}", e);
            return None;
        }
    }
}

fn get_network() -> ExecResult {
    (
        "holo_network",
        (Exec::shell("nixos-option system.holoNetwork") | Exec::shell("sed -n '2 p'")).capture(),
    )
}

fn get_channel() -> ExecResult {
    (
        "channel",
        (Exec::shell("nix-channel --list")
            | Exec::shell("grep holo-nixpkgs")
            | Exec::shell("cut -d '/' -f 7"))
        .capture(),
    )
}

fn get_holoport_model() -> ExecResult {
    (
        "holoport_model",
        (Exec::shell("nixos-option system.hpos.target 2>/dev/null") | Exec::shell("sed -n '2 p'"))
            .capture(),
    )
}

fn get_ssh_status() -> ExecResult {
    (
        "ssh_status",
        (Exec::shell("nixos-option profiles.development.enable 2>/dev/null")
            | Exec::shell("sed -n '2 p'")
            | Exec::shell("grep true || echo 'false'"))
        .capture(),
    )
}

fn get_zt_ip() -> ExecResult {
    (
        "zt_ip",
        (Exec::shell("zerotier-cli listnetworks")
            | Exec::shell("sed -n '2 p'")
            | Exec::shell("awk -F ' ' '{print $NF}'")
            | Exec::shell("awk -F ',' '{print $NF}'")
            | Exec::shell("awk -F '/' '{print $1}'"))
        .capture(),
    )
}

fn get_wan_ip() -> ExecResult {
    (
        "wan_ip",
        Exec::shell("curl -s https://ipecho.net/plain").capture(),
    )
}

/// Function parses result of Exec.capture()
/// In case of a failure in execution or non-zero exit status
/// logs an error and returns None, otherwise returns Some(stdout)
fn wrap(res: ExecResult) -> Option<String> {
    match res.1 {
        Ok(data) => {
            if data.success() {
                return Some(data.stdout_str().trim().trim_matches('"').to_owned());
            } else {
                warn!("Failed to get {}, {}", res.0, data.stderr_str());
                return None;
            }
        }
        Err(e) => {
            warn!("Failed to get {}: {:?}", res.0, e);
            return None;
        }
    };
}

/// Parses String looking for false or true
fn string_2_bool(val: Option<String>) -> Option<bool> {
    if let Some(str) = val {
        if let Ok(res) = &str.trim().parse::<bool>() {
            return Some(*res);
        }
    }
    None
}
