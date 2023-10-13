mod remote;

use super::{
    endpoint::fetch_rest_model, DataPipeline, PipelineTriggerPermit, ProcessingError,
    TriggerPermitType,
};
use aws_sdk_sfn;
use remote::{
    fetch_latest_datasource_events, fetch_state_machine_names, is_task_ready, send_task_heartbeat,
    send_task_success,
};
use reqwest::Client;
use std::future::Future;
use time::OffsetDateTime;
use tracing::{event, Level};
use url::Url;

fn get_task_token(data_pipeline: &DataPipeline) -> Result<String, ProcessingError> {
    match &data_pipeline.permit {
        TriggerPermitType::Lenient(Some(permit)) | TriggerPermitType::Strict(Some(permit)) => {
            Ok((&permit.content).to_string())
        }
        _ => Err(ProcessingError::MissingPipelinePermit(format!(
            "Missing pipeline trigger permit for {}",
            &data_pipeline.id
        ))),
    }
}

async fn does_state_machine_exist<F, FutF>(
    data_pipeline: &DataPipeline,
    sm_names_fetching_fn: F,
) -> Result<bool, ProcessingError>
where
    F: Fn() -> FutF,
    FutF: Future<Output = Result<Vec<String>, ProcessingError>>,
{
    let sm_vec = sm_names_fetching_fn().await?;
    Ok(sm_vec
        .iter()
        .any(|sm_name| -> bool { sm_name.eq(&data_pipeline.id) }))
}

async fn trigger_pipeline(data_pipeline: &DataPipeline) -> Result<(), ProcessingError> {
    let task_token = get_task_token(data_pipeline)?;
    let config = aws_config::load_from_env().await;
    let client = aws_sdk_sfn::Client::new(&config);
    send_task_heartbeat(&task_token, &client).await?;
    send_task_success(&task_token, &client).await
}

async fn is_pipeline_ready(data_pipeline: &DataPipeline) -> Result<bool, ProcessingError> {
    let task_token = get_task_token(data_pipeline)?;
    let config = aws_config::load_from_env().await;
    let client = aws_sdk_sfn::Client::new(&config);
    is_task_ready(&task_token, &client).await
}

async fn can_trigger_pipeline<'a, F, FutF, G, FutG>(
    data_pipeline: &'a DataPipeline,
    latest_data_source_event_times_fn: F,
    is_pipeline_ready_fn: G,
) -> Result<bool, ProcessingError>
where
    F: Fn(Url) -> FutF,
    FutF: Future<Output = Result<Vec<OffsetDateTime>, ProcessingError>>,
    G: Fn(&'a DataPipeline) -> FutG,
    FutG: Future<Output = Result<bool, ProcessingError>>,
{
    let last_pipeline_success_time = match &data_pipeline.last_success_time {
        Some(success_time) => Ok(success_time),
        None => Err(ProcessingError::MissingSuccessTime(format!(
            "Last success time missing for pipeline with id {}",
            &data_pipeline.id
        ))),
    }?;
    let has_new_source_event = match data_pipeline.permit {
        TriggerPermitType::Lenient(_) => {
            let mut flag = false;
            for ds_url in &data_pipeline.dependency_urls {
                let event_times = latest_data_source_event_times_fn(ds_url.clone()).await?;
                flag = event_times.iter().any(|t| t.ge(last_pipeline_success_time));
                if flag {
                    // early break here might save us a few
                    // extra calls for fetching event times
                    break;
                }
            }
            Ok(flag)
        }
        TriggerPermitType::Strict(_) => {
            let mut flags = vec![];
            for ds_url in &data_pipeline.dependency_urls {
                let flag = &latest_data_source_event_times_fn(ds_url.clone())
                    .await?
                    .iter()
                    .any(|t| t.ge(last_pipeline_success_time));
                flags.push(*flag);
            }
            if flags.len() > 0 {
                Ok(flags.iter().all(|flag| *flag == true))
            } else {
                // NOTE: pipelines with no data source events
                // will not be triggered for now. Flip this to
                // reverse this decision.
                Ok(false)
            }
        }
    }?;
    Ok(has_new_source_event && is_pipeline_ready_fn(data_pipeline).await?)
}

pub async fn maybe_trigger_pipeline(
    data_pipeline: &DataPipeline,
    aws_region: &str,
    client: &Client,
) -> Result<(), ProcessingError> {
    let times_fetching_fn =
        |url: Url| async move { fetch_latest_datasource_events(&url, aws_region, client).await };
    if can_trigger_pipeline(data_pipeline, times_fetching_fn, is_pipeline_ready).await? {
        trigger_pipeline(data_pipeline).await?;
    } else {
        let mssg = format!("Pipeline with id {} was not triggered.", &data_pipeline.id);
        event!(Level::INFO, mssg);
    };
    Ok(())
}
