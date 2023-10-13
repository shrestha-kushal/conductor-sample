use super::{
    basenames, construct_endpoint_url, fetch_rest_model, DataSourceRestModel, PipelineRestModel,
    ProcessingError,
};
use reqwest::Client;
use std::collections::HashMap;

pub async fn fetch_data_source_model(
    data_source_id: &str,
    endpoint_prefix: &str,
    aws_region: &str,
    client: &Client,
) -> Result<DataSourceRestModel, ProcessingError> {
    let endpoint_url = construct_endpoint_url(
        endpoint_prefix,
        &vec![String::from("data-sources"), String::from(data_source_id)],
    )?;
    fetch_rest_model::<DataSourceRestModel, String>(
        endpoint_url.as_str(),
        aws_region,
        client,
        "".to_string(),
        &HashMap::new(),
        "GET",
    )
    .await
    .map(|model| DataSourceRestModel {
        dependent_pipelines: basenames(&model.dependent_pipelines),
        ..model
    })
}

pub async fn fetch_pipeline_model(
    pipeline_id: &str,
    endpoint_prefix: &str,
    aws_region: &str,
    client: &Client,
) -> Result<PipelineRestModel, ProcessingError> {
    let endpoint_url = construct_endpoint_url(
        endpoint_prefix,
        &vec![String::from("pipelines"), String::from(pipeline_id)],
    )?;
    fetch_rest_model::<PipelineRestModel, String>(
        endpoint_url.as_str(),
        aws_region,
        client,
        "".to_string(),
        &HashMap::new(),
        "GET",
    )
    .await
}
