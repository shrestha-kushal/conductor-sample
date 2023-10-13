pub mod remote;
#[cfg(test)]
mod test_gathering;

use super::{
    endpoint::{basenames, construct_endpoint_url, fetch_rest_model},
    DataPipeline, PipelineTriggerPermit, ProcessingError, TriggerPermitType,
};
use crate::entities::EventPayload;
use remote::{fetch_data_source_model, fetch_pipeline_model};
use reqwest::Client;
use serde::Deserialize;
use std::future::Future;
use time::format_description::well_known::iso8601::Iso8601;
use time::OffsetDateTime;
use url::Url;

async fn gather_data_pipelines<F, FutF, G, FutG>(
    event_payload: EventPayload,
    fn_fetch_data_source: F,
    fn_fetch_data_pipeline: G,
) -> Result<Vec<DataPipeline>, ProcessingError>
where
    F: Fn(String) -> FutF,
    FutF: Future<Output = Result<DataSourceRestModel, ProcessingError>>,
    G: Fn(String) -> FutG,
    FutG: Future<Output = Result<PipelineRestModel, ProcessingError>>,
{
    match event_payload {
        EventPayload::DataSource { id } => {
            let source_id = id.to_string();
            let data_source_rest_model = fn_fetch_data_source(source_id).await?;
            let mut pipelines = Vec::new();
            for pipeline_id in data_source_rest_model.dependent_pipelines {
                let pipeline_model = fn_fetch_data_pipeline(String::from(pipeline_id)).await?;
                pipelines.push(DataPipeline {
                    permit: _get_trigger_permit(&pipeline_model)?,
                    id: String::from(&pipeline_model.id),
                    description: String::from(&pipeline_model.description),
                    last_success_time: _map_to_offsetdatetime(&pipeline_model.last_success_time)?,
                    dependency_urls: _map_str_to_url(&pipeline_model.source_dependencies)?,
                })
            }
            let pipelines = pipelines;
            Ok(pipelines)
        }
        EventPayload::DataPipeline {
            id,
            success_time,
            callback_token,
        } => {
            let pipeline_model = fn_fetch_data_pipeline(id).await?;
            let maybe_fetched_dt = _map_to_offsetdatetime(&pipeline_model.last_success_time)?;
            if let Some(fetched_dt) = maybe_fetched_dt {
                if fetched_dt == success_time {
                    let permit = _get_trigger_permit(&pipeline_model)?;
                    if let TriggerPermitType::Lenient(Some(fetched_permit))
                    | TriggerPermitType::Strict(Some(fetched_permit)) = &permit
                    {
                        if callback_token.eq(&fetched_permit.content) {
                            Ok(vec![DataPipeline {
                                permit: permit,
                                id: pipeline_model.id,
                                description: pipeline_model.description,
                                last_success_time: maybe_fetched_dt,
                                dependency_urls: _map_str_to_url(
                                    &pipeline_model.source_dependencies,
                                )?,
                            }])
                        } else {
                            Err(ProcessingError::PermitContentConflict(String::from(
                                "Conflict between provided permit content and stored permit content."
                            )))
                        }
                    } else {
                        Err(ProcessingError::MissingPermitContent(String::from(
                            "Permit content should have been stored; found nothing.",
                        )))
                    }
                } else {
                    Err(ProcessingError::SuccessTimeConflict(String::from(
                        "Conflict between provided success time and stored success time.",
                    )))
                }
            } else {
                Err(ProcessingError::MissingSuccessTime(String::from(
                    "Last success time should have been stored; found nothing.",
                )))
            }
        }
    }
}

fn _map_str_to_url(url_strings: &Vec<String>) -> Result<Vec<Url>, ProcessingError> {
    let mut urls: Vec<Url> = Vec::new();
    for url_string in url_strings {
        let result = Url::parse(url_string);
        if let Ok(url) = result {
            urls.push(url);
        } else {
            return Err(ProcessingError::UrlParseFailure(String::from(
                "Failed to convert strings to urls",
            )));
        }
    }
    let urls = urls;
    Ok(urls)
}

fn _map_to_offsetdatetime(
    maybe_dt_str: &Option<String>,
) -> Result<Option<OffsetDateTime>, ProcessingError> {
    if let Some(dt_str) = maybe_dt_str {
        if let Ok(date_time) = OffsetDateTime::parse(dt_str, &Iso8601::DEFAULT) {
            Ok(Some(date_time))
        } else {
            Err(ProcessingError::DatatimeParseFailure(String::from(
                "Failed to parse datetime.",
            )))
        }
    } else {
        Ok(None)
    }
}

fn _get_trigger_permit(
    pipeline_model: &PipelineRestModel,
) -> Result<TriggerPermitType, ProcessingError> {
    let maybe_permit = match &pipeline_model.callback_token {
        Some(token) => Some(PipelineTriggerPermit {
            content: String::from(token),
            is_expired: false,
        }),
        _ => None,
    };
    match pipeline_model.trigger_rule.as_str() {
        "LENIENT" => Ok(TriggerPermitType::Lenient(maybe_permit)),
        "STRICT" => Ok(TriggerPermitType::Strict(maybe_permit)),
        _ => Err(ProcessingError::UnrecognizedTriggerType(String::from(
            "Error: Unrecognized trigger permit type.",
        ))),
    }
}

#[derive(Deserialize, Clone)]
pub struct PipelineRestModel {
    id: String,
    description: String,
    last_success_time: Option<String>,
    source_dependencies: Vec<String>,
    trigger_rule: String,
    callback_token: Option<String>,
}

#[derive(Deserialize, Clone)]
pub struct DataSourceRestModel {
    id: String,
    description: String,
    dependent_pipelines: Vec<String>,
}

pub async fn get_data_pipelines(
    event_payload: EventPayload,
    endpoint_prefix: &str,
    aws_region: &str,
    client: &Client,
) -> Result<Vec<DataPipeline>, ProcessingError> {
    let fn_fetch_data_source = |ds_id: String| async move {
        fetch_data_source_model(&ds_id, endpoint_prefix, aws_region, client).await
    };
    let fn_fetch_data_pipeline = |pipeline_id: String| async move {
        fetch_pipeline_model(&pipeline_id, endpoint_prefix, aws_region, client).await
    };
    gather_data_pipelines(event_payload, fn_fetch_data_source, fn_fetch_data_pipeline).await
}
