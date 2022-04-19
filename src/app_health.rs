use anyhow::{anyhow, Context, Result};
use holochain::conductor::api::{AppStatusFilter, InstalledAppInfoStatus};
use holochain_types::app::InstalledAppId;
use std::collections::HashMap;

use super::websocket::AdminWebsocket;

const ADMIN_PORT: u16 = 4444;

pub async fn get_hpos_app_health(
    port: Option<u16>,
) -> Result<HashMap<InstalledAppId, AppStatusFilter>> {
    let admin_port = match port {
        Some(p) => p,
        None => ADMIN_PORT,
    };

    let mut admin_websocket = AdminWebsocket::connect(admin_port)
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

#[cfg(test)]
mod test {
    use holochain::sweettest::SweetConductor;
    #[tokio::test(flavor = "multi_thread")]
    async fn hpos_app_health() {
        let conductor = SweetConductor::from_standard_config().await;
        let admin_port = conductor.get_arbitrary_admin_websocket_port();
        let hpos_happ_health_map = super::get_hpos_app_health(admin_port).await.unwrap();
        assert_eq!(hpos_happ_health_map.len(), 0);
    }
}
