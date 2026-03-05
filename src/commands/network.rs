use anyhow::Result;
#[cfg(not(target_arch = "wasm32"))]
use datadog_api_client::datadogV2::api_network_device_monitoring::{
    GetInterfacesOptionalParams, NetworkDeviceMonitoringAPI,
};
#[cfg(not(target_arch = "wasm32"))]
use datadog_api_client::datadogV2::model::{ListInterfaceTagsResponse, ListTagsResponse};

#[cfg(not(target_arch = "wasm32"))]
use crate::client;
use crate::config::Config;
use crate::formatter;
use crate::util;

#[cfg(not(target_arch = "wasm32"))]
fn make_api(cfg: &Config) -> NetworkDeviceMonitoringAPI {
    let dd_cfg = client::make_dd_config(cfg);
    match client::make_bearer_client(cfg) {
        Some(c) => NetworkDeviceMonitoringAPI::with_client_and_config(dd_cfg, c),
        None => NetworkDeviceMonitoringAPI::with_config(dd_cfg),
    }
}

// ---- Devices ----

#[cfg(not(target_arch = "wasm32"))]
pub async fn devices_list(cfg: &Config) -> Result<()> {
    let api = make_api(cfg);
    let resp = api
        .list_devices(Default::default())
        .await
        .map_err(|e| anyhow::anyhow!("failed to list devices: {e:?}"))?;
    formatter::output(cfg, &resp)
}

#[cfg(target_arch = "wasm32")]
pub async fn devices_list(cfg: &Config) -> Result<()> {
    let data = crate::api::get(cfg, "/api/v2/ndm/devices", &[]).await?;
    crate::formatter::output(cfg, &data)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn devices_get(cfg: &Config, device_id: &str) -> Result<()> {
    let api = make_api(cfg);
    let resp = api
        .get_device(device_id.to_string())
        .await
        .map_err(|e| anyhow::anyhow!("failed to get device: {e:?}"))?;
    formatter::output(cfg, &resp)
}

#[cfg(target_arch = "wasm32")]
pub async fn devices_get(cfg: &Config, device_id: &str) -> Result<()> {
    let data = crate::api::get(cfg, &format!("/api/v2/ndm/devices/{device_id}"), &[]).await?;
    crate::formatter::output(cfg, &data)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn devices_interfaces(cfg: &Config, device_id: &str, ip_addresses: bool) -> Result<()> {
    let api = make_api(cfg);
    let params = GetInterfacesOptionalParams::default().get_ip_addresses(ip_addresses);
    let resp = api
        .get_interfaces(device_id.to_string(), params)
        .await
        .map_err(|e| anyhow::anyhow!("failed to get interfaces: {e:?}"))?;
    formatter::output(cfg, &resp)
}

#[cfg(target_arch = "wasm32")]
pub async fn devices_interfaces(cfg: &Config, device_id: &str, ip_addresses: bool) -> Result<()> {
    let q = vec![
        ("filter[device_id]", device_id.to_string()),
        ("get_ip_addresses", ip_addresses.to_string()),
    ];
    let data = crate::api::get(cfg, "/api/v2/ndm/interfaces", &q).await?;
    crate::formatter::output(cfg, &data)
}

// ---- Device tags ----

#[cfg(not(target_arch = "wasm32"))]
pub async fn devices_tags_list(cfg: &Config, device_id: &str) -> Result<()> {
    let api = make_api(cfg);
    let resp = api
        .list_device_user_tags(device_id.to_string())
        .await
        .map_err(|e| anyhow::anyhow!("failed to list device tags: {e:?}"))?;
    formatter::output(cfg, &resp)
}

#[cfg(target_arch = "wasm32")]
pub async fn devices_tags_list(cfg: &Config, device_id: &str) -> Result<()> {
    let data = crate::api::get(cfg, &format!("/api/v2/ndm/tags/devices/{device_id}"), &[]).await?;
    crate::formatter::output(cfg, &data)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn devices_tags_update(cfg: &Config, device_id: &str, file: &str) -> Result<()> {
    let body: ListTagsResponse = util::read_json_file(file)?;
    let api = make_api(cfg);
    let resp = api
        .update_device_user_tags(device_id.to_string(), body)
        .await
        .map_err(|e| anyhow::anyhow!("failed to update device tags: {e:?}"))?;
    formatter::output(cfg, &resp)
}

#[cfg(target_arch = "wasm32")]
pub async fn devices_tags_update(cfg: &Config, device_id: &str, file: &str) -> Result<()> {
    let body: serde_json::Value = util::read_json_file(file)?;
    let data =
        crate::api::patch(cfg, &format!("/api/v2/ndm/tags/devices/{device_id}"), &body).await?;
    crate::formatter::output(cfg, &data)
}

// ---- Interface tags ----

#[cfg(not(target_arch = "wasm32"))]
pub async fn interfaces_tags_list(cfg: &Config, interface_id: &str) -> Result<()> {
    let api = make_api(cfg);
    let resp = api
        .list_interface_user_tags(interface_id.to_string())
        .await
        .map_err(|e| anyhow::anyhow!("failed to list interface tags: {e:?}"))?;
    formatter::output(cfg, &resp)
}

#[cfg(target_arch = "wasm32")]
pub async fn interfaces_tags_list(cfg: &Config, interface_id: &str) -> Result<()> {
    let data = crate::api::get(
        cfg,
        &format!("/api/v2/ndm/tags/interfaces/{interface_id}"),
        &[],
    )
    .await?;
    crate::formatter::output(cfg, &data)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn interfaces_tags_update(cfg: &Config, interface_id: &str, file: &str) -> Result<()> {
    let body: ListInterfaceTagsResponse = util::read_json_file(file)?;
    let api = make_api(cfg);
    let resp = api
        .update_interface_user_tags(interface_id.to_string(), body)
        .await
        .map_err(|e| anyhow::anyhow!("failed to update interface tags: {e:?}"))?;
    formatter::output(cfg, &resp)
}

#[cfg(target_arch = "wasm32")]
pub async fn interfaces_tags_update(cfg: &Config, interface_id: &str, file: &str) -> Result<()> {
    let body: serde_json::Value = util::read_json_file(file)?;
    let data = crate::api::patch(
        cfg,
        &format!("/api/v2/ndm/tags/interfaces/{interface_id}"),
        &body,
    )
    .await?;
    crate::formatter::output(cfg, &data)
}

// ---- Flows ----

pub async fn flows_list(cfg: &Config) -> Result<()> {
    let placeholder = serde_json::json!({
        "data": [],
        "meta": {
            "message": "Network flows list - API endpoint implementation pending"
        }
    });
    formatter::output(cfg, &placeholder)
}
