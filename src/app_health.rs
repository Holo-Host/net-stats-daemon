use anyhow::{anyhow, Result};
use holochain_conductor_api::{AppInfoStatus, AppStatusFilter};
use holochain_types::app::InstalledAppId;
use std::collections::HashMap;

use hpos_hc_connect::AdminWebsocket;

pub async fn get_hpos_app_health(
    admin_ws: &mut AdminWebsocket,
) -> Result<HashMap<InstalledAppId, AppStatusFilter>> {
    let mut hpos_happ_health_map = HashMap::new();
    match admin_ws.list_apps(None).await {
        Ok(hpos_happs) => hpos_happs.iter().for_each(|happ| {
            let happ_status = match &happ.status {
                AppInfoStatus::Paused { .. } => AppStatusFilter::Paused,
                AppInfoStatus::Disabled { .. } => AppStatusFilter::Disabled,
                AppInfoStatus::Running => AppStatusFilter::Running,
                AppInfoStatus::AwaitingMemproofs => AppStatusFilter::Enabled,
            };
            hpos_happ_health_map.insert(happ.installed_app_id.clone(), happ_status);
        }),
        Err(e) => return Err(anyhow!("Error calling `admin/list_apps`. {:?}", e)),
    };

    Ok(hpos_happ_health_map)
}
