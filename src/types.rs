use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Payload {
    holo_network: Option(String),
    channel: Option(String),
    model: Option(String),
    ssh_status: Option(bool),
    holo_network: Option(String),
    zt_ip: Option(String),
    wan_ip: Option(String),
    holoport_id: Option(String),
    timestamp: Option(u32)
}