use super::config::get_all_hha_happs;
use super::stats::EnabledAppStats;
use super::websocket::{AdminWebsocket, AppWebsocket};

use anyhow::{anyhow, Context, Result};
use holochain::conductor::api::{AppStatusFilter, InstalledAppInfoStatus};
use holochain_types::app::InstalledAppId;
use std::collections::HashMap;

const ADMIN_PORT: u16 = 4444;
const APP_PORT: u16 = 42233;

pub async fn get_hpos_app_health() -> Result<HashMap<InstalledAppId, AppStatusFilter>> {
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
            hpos_happ_health_map.insert(happ.installed_app_id.clone(), happ_status);
        }),
        Err(e) => return Err(anyhow!("Error calling `admin/list_apps`. {:?}", e)),
    };

    Ok(hpos_happ_health_map)
}

pub async fn get_running_apps() -> Result<EnabledAppStats> {
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
                read_only: Some(read_only),
                sl: Some(sl),
                core: Some(core),
            });
        }
        Err(e) => return Err(anyhow!("Error calling `admin/list_apps`. {:?}", e)),
    };
}

pub async fn get_installed_app_map(
    core_hha_id: InstalledAppId,
) -> Result<HashMap<InstalledAppId, i32>> {
    let mut installed_app_map = HashMap::new();

    let app_websocket = AppWebsocket::connect(APP_PORT)
        .await
        .context("Failed to connect to the holochain admin interface.")?;

    let hha_happ_ids = get_all_hha_happs(app_websocket, core_hha_id).await.unwrap();
    for hha_happ_id in hha_happ_ids.clone() {
        installed_app_map.insert(hha_happ_id.0.to_string(), 0);
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
                    .unwrap()
                    .0
                    .to_string();

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
