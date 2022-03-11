use anyhow::{anyhow, Context, Result};
use hc_utils::WrappedHeaderHash;
use holochain::conductor::api::{AppStatusFilter, InstalledAppInfoStatus};
use holochain_types::app::InstalledAppId;
use log::warn;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use subprocess::{CaptureData, Exec, Result as PopenResult};

use super::config::get_all_hha_happs;
use super::websocket::{AdminWebsocket, AppWebsocket};

const ADMIN_PORT: u16 = 4444;
const APP_PORT: u16 = 0; // 42233

pub struct EnabledAppStats {
    read_only: Vec<InstalledAppId>,
    sl: Vec<InstalledAppId>,
    core: Vec<InstalledAppId>,
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
    hpos_app_health_map: HashMap<InstalledAppId, AppStatusFilter>,
    running_read_only_happs: Vec<InstalledAppId>,
    running_sl_cells: Vec<InstalledAppId>,
    running_core_happs: Vec<InstalledAppId>,
    installed_app_map: HashMap<WrappedHeaderHash, i32>,
}

impl Stats {
    pub async fn new(pubkey_base36: &str, core_hha_id: InstalledAppId) -> Self {
        let running_apps = get_running_apps().await.unwrap();
        Self {
            holo_network: wrap(get_network()),
            channel: wrap(get_channel()),
            holoport_model: wrap(get_holoport_model()),
            ssh_status: string_2_bool(wrap(get_ssh_status())),
            zt_ip: wrap(get_zt_ip()),
            wan_ip: wrap(get_wan_ip()),
            holoport_id: Some(pubkey_base36.to_owned()),
            timestamp: None,
            hpos_app_health_map: get_hpos_app_health().await.unwrap(),
            running_read_only_happs: running_apps.read_only,
            running_sl_cells: running_apps.sl,
            running_core_happs: running_apps.core,
            installed_app_map: get_installed_app_map(core_hha_id).await.unwrap(),
        }
    }

    pub fn into_bytes(&self) -> Result<Vec<u8>> {
        serde_json::to_vec(&self).context("Failed to convert payload to bytes")
    }
}

type ExecResult = (&'static str, PopenResult<CaptureData>);

async fn get_hpos_app_health() -> Result<HashMap<InstalledAppId, AppStatusFilter>> {
    let mut admin_websocket = AdminWebsocket::connect(ADMIN_PORT)
        .await
        .context("Failed to connect to the holochain admin interface.")?;

    let mut hpos_happ_health_map = HashMap::new();
    match admin_websocket.list_apps(None).await {
        Ok(hpos_happs) => hpos_happs.iter().for_each(|happ| {
            let happ_status = match &happ.status {
                InstalledAppInfoStatus::Paused { .. } => AppStatusFilter::Paused,
                InstalledAppInfoStatus::Disabled { .. } => AppStatusFilter::Disabled,
                InstalledAppInfoStatus::Running => AppStatusFilter::Running,
            };
            println!(">>>>>>>>>>> happ_status : {:?} ", happ_status);
            hpos_happ_health_map.insert(happ.installed_app_id.clone(), happ_status);
        }),
        Err(e) => return Err(anyhow!("Error calling `admin/list_apps`. {:?}", e)),
    };

    println!(
        ">>>>>>>>>>> HPOS APP HEATH map : {:?} ",
        hpos_happ_health_map
    );
    Ok(hpos_happ_health_map)
}

async fn get_running_apps() -> Result<EnabledAppStats> {
    let mut admin_websocket = AdminWebsocket::connect(ADMIN_PORT)
        .await
        .context("Failed to connect to the holochain admin interface.")?;

    let mut read_only = Vec::new();
    let mut sl = Vec::new();
    let mut core = Vec::new();

    match admin_websocket
        .list_apps(Some(AppStatusFilter::Running))
        .await
    {
        Ok(hpos_happs) => {
            hpos_happs.iter().for_each(|happ| {
                // Note: Read_only instance ids exactly equal the happ's hha_id, and thereby exclude any colon
                if !happ.installed_app_id.contains(":") {
                    read_only.push(happ.installed_app_id.clone())
                } else if happ.installed_app_id.contains("::servicelogger") {
                    sl.push(happ.installed_app_id.clone())
                // Note: There are only 2 hard coded core apps.  Their ids follow the pattern: `app_name:v_number::uuid`
                } else if happ.installed_app_id.contains("core-app:v_")
                    || happ.installed_app_id.contains("servicelogger:v_")
                {
                    core.push(happ.installed_app_id.clone())
                }
            });

            return Ok(EnabledAppStats {
                read_only,
                sl,
                core,
            });
        }
        Err(e) => return Err(anyhow!("Error calling `admin/list_apps`. {:?}", e)),
    };
}

async fn get_installed_app_map(
    core_hha_id: InstalledAppId,
) -> Result<HashMap<WrappedHeaderHash, i32>> {
    let mut installed_app_map = HashMap::new();

    let app_websocket = AppWebsocket::connect(APP_PORT)
        .await
        .context("Failed to connect to the holochain admin interface.")?;

    let hha_happ_ids = get_all_hha_happs(app_websocket, core_hha_id).await.unwrap();
    for hha_happ_id in hha_happ_ids.clone() {
        installed_app_map.insert(hha_happ_id.clone(), 0);
    }

    let mut admin_websocket = AdminWebsocket::connect(ADMIN_PORT)
        .await
        .context("Failed to connect to the holochain admin interface.")?;

    match admin_websocket.list_apps(None).await {
        Ok(hpos_happs) => {
            for happ in hpos_happs {
                let happ_id = hha_happ_ids
                    .clone()
                    .into_iter()
                    .find(|id| {
                        happ.installed_app_id
                            .clone()
                            .contains(&format!("{:?}", id))
                            .to_owned()
                    })
                    .unwrap();
                // // if let Some((k, v)) = installed_app_map.get_key_value(&happ_id.unwrap()) {
                // //     installed_app_map.insert(k, v + 1);
                // // }
                if let Some(v) = installed_app_map.get_mut(&happ_id) {
                    let new_value = *v;
                    *v = new_value + 1;
                }
            }
        }
        Err(e) => return Err(anyhow!("Error calling `admin/list_apps`. {:?}", e)),
    };

    Ok(installed_app_map)
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
