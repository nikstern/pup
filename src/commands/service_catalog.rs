use anyhow::Result;
use datadog_api_client::datadogV2::api_service_definition::{
    GetServiceDefinitionOptionalParams, ListServiceDefinitionsOptionalParams, ServiceDefinitionAPI,
};

use crate::client;
use crate::config::Config;
use crate::formatter;

pub async fn list(cfg: &Config, page_size: Option<i64>, page_number: Option<i64>) -> Result<()> {
    let dd_cfg = client::make_dd_config(cfg);
    let api = match client::make_bearer_client(cfg) {
        Some(c) => ServiceDefinitionAPI::with_client_and_config(dd_cfg, c),
        None => ServiceDefinitionAPI::with_config(dd_cfg),
    };
    let mut params = ListServiceDefinitionsOptionalParams::default();
    if let Some(s) = page_size {
        params = params.page_size(s);
    }
    if let Some(n) = page_number {
        params = params.page_number(n);
    }
    let resp = api
        .list_service_definitions(params)
        .await
        .map_err(|e| anyhow::anyhow!("failed to list services: {e:?}"))?;
    let raw = serde_json::to_value(&resp)?;
    // Note: the Service Catalog API returns no pagination metadata (no `meta` field),
    // so agent mode will not emit a `pagination` envelope for this command.
    formatter::output_with_raw(cfg, &resp, &raw)
}

pub async fn get(cfg: &Config, service_name: &str) -> Result<()> {
    let dd_cfg = client::make_dd_config(cfg);
    let api = match client::make_bearer_client(cfg) {
        Some(c) => ServiceDefinitionAPI::with_client_and_config(dd_cfg, c),
        None => ServiceDefinitionAPI::with_config(dd_cfg),
    };
    let resp = api
        .get_service_definition(
            service_name.to_string(),
            GetServiceDefinitionOptionalParams::default(),
        )
        .await
        .map_err(|e| anyhow::anyhow!("failed to get service: {e:?}"))?;
    formatter::output(cfg, &resp)
}
