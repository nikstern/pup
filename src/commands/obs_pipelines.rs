use anyhow::Result;
use datadog_api_client::datadogV2::api_observability_pipelines::{
    ListPipelinesOptionalParams, ObservabilityPipelinesAPI,
};
use datadog_api_client::datadogV2::model::{ObservabilityPipeline, ObservabilityPipelineSpec};

use crate::client;
use crate::config::Config;
use crate::formatter;
use crate::util;

fn make_api(cfg: &Config) -> ObservabilityPipelinesAPI {
    // Observability Pipelines does not support OAuth — API key auth only.
    ObservabilityPipelinesAPI::with_config(client::make_dd_config(cfg))
}

pub async fn list(cfg: &Config, limit: i64, page_number: Option<i64>) -> Result<()> {
    let api = make_api(cfg);
    let mut params = ListPipelinesOptionalParams::default().page_size(limit);
    if let Some(n) = page_number {
        params = params.page_number(n);
    }
    let resp = api
        .list_pipelines(params)
        .await
        .map_err(|e| anyhow::anyhow!("failed to list pipelines: {e:?}"))?;
    let pagination = resp.meta.as_ref().and_then(|m| m.total_count).map(|total| {
        let current_page = page_number.unwrap_or(0);
        let has_next = (current_page + 1) * limit < total;
        formatter::PaginationInfo {
            cursor: None,
            next_offset: None,
            page: Some(current_page),
            page_count: None,
            per_page: Some(limit),
            total_count: Some(total),
            has_next_page: has_next,
            pagination_type: formatter::PaginationType::PageNumber,
        }
    });
    formatter::output_with_pagination(cfg, &resp, pagination)
}

pub async fn get(cfg: &Config, pipeline_id: &str) -> Result<()> {
    let api = make_api(cfg);
    let resp = api
        .get_pipeline(pipeline_id.to_string())
        .await
        .map_err(|e| anyhow::anyhow!("failed to get pipeline: {e:?}"))?;
    formatter::output(cfg, &resp)
}

pub async fn create(cfg: &Config, file: &str) -> Result<()> {
    let body: ObservabilityPipelineSpec = util::read_json_file(file)?;
    let api = make_api(cfg);
    let resp = api
        .create_pipeline(body)
        .await
        .map_err(|e| anyhow::anyhow!("failed to create pipeline: {e:?}"))?;
    formatter::output(cfg, &resp)
}

pub async fn update(cfg: &Config, pipeline_id: &str, file: &str) -> Result<()> {
    let body: ObservabilityPipeline = util::read_json_file(file)?;
    let api = make_api(cfg);
    let resp = api
        .update_pipeline(pipeline_id.to_string(), body)
        .await
        .map_err(|e| anyhow::anyhow!("failed to update pipeline: {e:?}"))?;
    formatter::output(cfg, &resp)
}

pub async fn delete(cfg: &Config, pipeline_id: &str) -> Result<()> {
    let api = make_api(cfg);
    api.delete_pipeline(pipeline_id.to_string())
        .await
        .map_err(|e| anyhow::anyhow!("failed to delete pipeline: {e:?}"))?;
    eprintln!("Pipeline {pipeline_id} deleted.");
    Ok(())
}

pub async fn validate(cfg: &Config, file: &str) -> Result<()> {
    let body: ObservabilityPipelineSpec = util::read_json_file(file)?;
    let api = make_api(cfg);
    let resp = api
        .validate_pipeline(body)
        .await
        .map_err(|e| anyhow::anyhow!("failed to validate pipeline: {e:?}"))?;
    formatter::output(cfg, &resp)
}
