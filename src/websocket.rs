use anyhow::{anyhow, Context, Result};
use holochain::conductor::api::{AdminRequest, AdminResponse, AppStatusFilter, InstalledAppInfo};

use holochain_websocket::{connect, WebsocketConfig, WebsocketSender};
use std::sync::Arc;

use log::{debug, info};
use url::Url;

#[derive(Clone, Debug)]
pub struct AdminWebsocket {
    tx: WebsocketSender,
}

impl AdminWebsocket {
    pub async fn connect(admin_port: u16) -> Result<Self> {
        let url = format!("ws://localhost:{}/", admin_port);
        debug!("Connecting to Conductor Admin Interface at: {:?}", url);
        let url = Url::parse(&url).context("invalid ws:// URL")?;
        let websocket_config = Arc::new(WebsocketConfig::default());
        match again::retry(|| {
            let websocket_config = Arc::clone(&websocket_config);
            connect(url.clone().into(), websocket_config)
        })
        .await
        {
            Ok((tx, _rx)) => Ok(Self { tx }),
            Err(e) => Err(anyhow!("error: {:?}", e)),
        }
    }

    pub async fn list_apps(
        &mut self,
        status_filter: Option<AppStatusFilter>,
    ) -> Result<Vec<InstalledAppInfo>> {
        let response = self.send(AdminRequest::ListApps { status_filter }).await?;
        match response {
            AdminResponse::AppsListed(apps_infos) => Ok(apps_infos),
            _ => Err(anyhow!("unexpected response: {:?}", response)),
        }
    }

    async fn send(&mut self, msg: AdminRequest) -> Result<AdminResponse> {
        let response = self
            .tx
            .request(&msg)
            .await
            .context("failed to send message")?;
        match response {
            AdminResponse::Error(error) => Err(anyhow!("error: {:?}", error)),
            _ => {
                info!("Successful admin request for message {:?} : ", msg);
                Ok(response)
            }
        }
    }
}
