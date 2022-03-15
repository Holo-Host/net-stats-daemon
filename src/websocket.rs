use anyhow::{anyhow, Context, Result};
use holochain::conductor::api::{
    AdminRequest, AdminResponse, AppRequest, AppResponse, AppStatusFilter, InstalledAppInfo,
    ZomeCall,
};

use holochain_types::{app::InstalledAppId, dna::AgentPubKey};
use holochain_websocket::{connect, WebsocketConfig, WebsocketSender};
use std::sync::Arc;

use log::{debug, info};
use url::Url;

#[derive(Clone, Debug)]
pub struct AdminWebsocket {
    tx: WebsocketSender,
    agent_key: Option<AgentPubKey>,
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
            Ok((tx, _rx)) => Ok(Self {
                tx,
                agent_key: None,
            }),
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

#[derive(Clone)]
pub struct AppWebsocket {
    tx: WebsocketSender,
}

impl AppWebsocket {
    pub async fn connect(app_port: u16) -> Result<Self> {
        let url = format!("ws://localhost:{}/", app_port);
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

    pub async fn zome_call(&mut self, msg: ZomeCall) -> Result<AppResponse> {
        let app_request = AppRequest::ZomeCall(Box::new(msg));
        let response = self.send(app_request).await;
        response
    }

    pub async fn get_app_info(&mut self, app_id: InstalledAppId) -> Option<InstalledAppInfo> {
        let msg = AppRequest::AppInfo {
            installed_app_id: app_id,
        };
        let response = self.send(msg).await.ok()?;
        match response {
            AppResponse::AppInfo(app_info) => app_info,
            _ => None,
        }
    }

    async fn send(&mut self, msg: AppRequest) -> Result<AppResponse> {
        let response = self
            .tx
            .request(msg.clone())
            .await
            .context("failed to send message")?;
        match response {
            AppResponse::Error(error) => Err(anyhow!("error: {:?}", error)),
            _ => {
                info!("Successful app request for message {:?} : ", msg);
                Ok(response)
            }
        }
    }
}
