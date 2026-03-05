use anyhow::Result;
#[cfg(not(target_arch = "wasm32"))]
use datadog_api_client::datadogV2::api_product_analytics::ProductAnalyticsAPI;
#[cfg(not(target_arch = "wasm32"))]
use datadog_api_client::datadogV2::model::{
    ProductAnalyticsAnalyticsRequest, ProductAnalyticsServerSideEventItem,
};

#[cfg(not(target_arch = "wasm32"))]
use crate::client;
use crate::config::Config;
use crate::formatter;
use crate::util;

#[cfg(not(target_arch = "wasm32"))]
pub async fn events_send(cfg: &Config, file: &str) -> Result<()> {
    let body: ProductAnalyticsServerSideEventItem = util::read_json_file(file)?;
    let dd_cfg = client::make_dd_config(cfg);
    let api = match client::make_bearer_client(cfg) {
        Some(c) => ProductAnalyticsAPI::with_client_and_config(dd_cfg, c),
        None => ProductAnalyticsAPI::with_config(dd_cfg),
    };
    let resp = api
        .submit_product_analytics_event(body)
        .await
        .map_err(|e| anyhow::anyhow!("failed to send product analytics event: {e:?}"))?;
    formatter::output(cfg, &resp)
}

#[cfg(target_arch = "wasm32")]
pub async fn events_send(cfg: &Config, file: &str) -> Result<()> {
    let body: serde_json::Value = util::read_json_file(file)?;
    let data = crate::api::post(cfg, "/api/v2/product-analytics/events", &body).await?;
    crate::formatter::output(cfg, &data)
}

// ---- Query ----

#[cfg(not(target_arch = "wasm32"))]
fn make_api(cfg: &Config) -> ProductAnalyticsAPI {
    let dd_cfg = client::make_dd_config(cfg);
    match client::make_bearer_client(cfg) {
        Some(c) => ProductAnalyticsAPI::with_client_and_config(dd_cfg, c),
        None => ProductAnalyticsAPI::with_config(dd_cfg),
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn query_scalar(cfg: &Config, file: &str) -> Result<()> {
    let body: ProductAnalyticsAnalyticsRequest = util::read_json_file(file)?;
    let api = make_api(cfg);
    let resp = api
        .query_product_analytics_scalar(body)
        .await
        .map_err(|e| anyhow::anyhow!("failed to query product analytics scalar: {e:?}"))?;
    formatter::output(cfg, &resp)
}

#[cfg(target_arch = "wasm32")]
pub async fn query_scalar(cfg: &Config, file: &str) -> Result<()> {
    let body: serde_json::Value = util::read_json_file(file)?;
    let data = crate::api::post(cfg, "/api/v2/product-analytics/analytics/scalar", &body).await?;
    crate::formatter::output(cfg, &data)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn query_timeseries(cfg: &Config, file: &str) -> Result<()> {
    let body: ProductAnalyticsAnalyticsRequest = util::read_json_file(file)?;
    let api = make_api(cfg);
    let resp = api
        .query_product_analytics_timeseries(body)
        .await
        .map_err(|e| anyhow::anyhow!("failed to query product analytics timeseries: {e:?}"))?;
    formatter::output(cfg, &resp)
}

#[cfg(target_arch = "wasm32")]
pub async fn query_timeseries(cfg: &Config, file: &str) -> Result<()> {
    let body: serde_json::Value = util::read_json_file(file)?;
    let data =
        crate::api::post(cfg, "/api/v2/product-analytics/analytics/timeseries", &body).await?;
    crate::formatter::output(cfg, &data)
}
