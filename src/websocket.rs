use anyhow::{anyhow, Context, Result};
use holochain::conductor::api::{
    AdminRequest, AdminResponse, AppRequest, AppResponse, AppStatusFilter, InstalledAppInfo,
    ZomeCall,
};

use holochain_types::dna::AgentPubKey;
use holochain_websocket::{connect, WebsocketConfig, WebsocketSender};
use std::sync::Arc;

use tracing::{instrument, trace};
use url::Url;

#[derive(Clone, Debug)]
pub struct AdminWebsocket {
    tx: WebsocketSender,
    agent_key: Option<AgentPubKey>,
}

impl AdminWebsocket {
    #[instrument(err)]
    pub async fn connect(admin_port: u16) -> Result<Self> {
        let url = format!("ws://localhost:{}/", admin_port);
        let url = Url::parse(&url).context("invalid ws:// URL")?;
        let websocket_config = Arc::new(WebsocketConfig::default());
        let (tx, _rx) = again::retry(|| {
            let websocket_config = Arc::clone(&websocket_config);
            connect(url.clone().into(), websocket_config)
        })
        .await?;
        Ok(Self {
            tx,
            agent_key: None,
        })
    }

    #[instrument(skip(self), err)]
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

    #[instrument(skip(self))]
    async fn send(&mut self, msg: AdminRequest) -> Result<AdminResponse> {
        let response = self
            .tx
            .request(msg)
            .await
            .context("failed to send message")?;
        match response {
            AdminResponse::Error(error) => Err(anyhow!("error: {:?}", error)),
            _ => {
                trace!("send successful");
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
    #[instrument(err)]
    pub async fn connect(app_port: u16) -> Result<Self> {
        let url = format!("ws://localhost:{}/", app_port);
        let url = Url::parse(&url).context("invalid ws:// URL")?;
        let websocket_config = Arc::new(WebsocketConfig::default());
        let (tx, _rx) = again::retry(|| {
            let websocket_config = Arc::clone(&websocket_config);
            connect(url.clone().into(), websocket_config)
        })
        .await?;
        Ok(Self { tx })
    }

    #[instrument(skip(self), err)]
    pub async fn zome_call(&mut self, msg: ZomeCall) -> Result<AppResponse> {
        let app_request = AppRequest::ZomeCall(Box::new(msg));
        let response = self.send(app_request).await;
        response
    }

    #[instrument(skip(self))]
    async fn send(&mut self, msg: AppRequest) -> Result<AppResponse> {
        let response = self
            .tx
            .request(msg)
            .await
            .context("failed to send message")?;
        match response {
            AppResponse::Error(error) => Err(anyhow!("error: {:?}", error)),
            _ => {
                trace!("send successful");
                Ok(response)
            }
        }
    }
}
