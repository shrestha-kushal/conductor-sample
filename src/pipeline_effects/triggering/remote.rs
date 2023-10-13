use super::{fetch_rest_model, ProcessingError};
use aws_sdk_sfn::{self, operation::send_task_heartbeat::SendTaskHeartbeatError};
use aws_smithy_http::result::SdkError;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use time::format_description::well_known::iso8601::Iso8601;
use time::OffsetDateTime;

pub async fn send_task_success(
    token: &str,
    client: &aws_sdk_sfn::Client,
) -> Result<(), ProcessingError> {
    let result_future = client
        .send_task_success()
        .set_task_token(Some(token.to_string()))
        .send();
    match result_future.await {
        Ok(_) => Ok(()),
        Err(error) => Err(ProcessingError::RelayTaskSuccessError(format!(
            "{:?}",
            error
        ))),
    }
}

pub async fn fetch_state_machine_names(
    client: &aws_sdk_sfn::Client,
) -> Result<Vec<String>, ProcessingError> {
    let response_future = client.list_state_machines().send();
    match response_future.await {
        Ok(response) => match response.state_machines() {
            Some(items) => {
                let sm_names: Vec<String> = items
                    .iter()
                    .map(|item| {
                        item.name()
                            .unwrap_or("dontmatchc706c64c6a794a87890ae0aa46b17d4c")
                            .to_string()
                    })
                    .collect();
                Ok(sm_names)
            }
            None => Ok(vec![]),
        },
        Err(error) => Err(ProcessingError::StateMachineFetchingError(format!(
            "{:?}",
            error
        ))),
    }
}

pub async fn send_task_heartbeat(
    token: &str,
    client: &aws_sdk_sfn::Client,
) -> Result<(), ProcessingError> {
    let result_future = client
        .send_task_heartbeat()
        .set_task_token(Some(token.to_string()))
        .send();
    match result_future.await {
        Ok(_) => Ok(()),
        Err(error) => Err(ProcessingError::RelayTaskHeartbeatError(format!(
            "State machine heatbeat relay failed. Perhaps state machine down or busy.\n{:?}",
            error
        ))),
    }
}

pub async fn is_task_ready(
    token: &str,
    client: &aws_sdk_sfn::Client,
) -> Result<bool, ProcessingError> {
    let result_future = client
        .send_task_heartbeat()
        .set_task_token(Some(token.to_string()))
        .send();
    match result_future.await {
        Ok(_) => Ok(true),
        Err(SdkError::ServiceError(service_error)) => match service_error.err() {
            SendTaskHeartbeatError::TaskTimedOut(_) => Ok(false),
            error => Err(ProcessingError::RelayTaskHeartbeatError(format!(
                "State machine heartbeat relay failed.\n{:?}",
                error
            ))),
        },
        Err(error) => Err(ProcessingError::RelayTaskHeartbeatError(format!(
            "State machine heartbeat relay failed.\n{:?}",
            error
        ))),
    }
}

#[derive(Deserialize, Clone)]
struct EventRestModel {
    id: String,
    description: Option<String>,
    event_time: String,
    event_type: EventType,
    raised_by: String,
}

#[derive(Deserialize, Clone)]
enum EventType {
    #[serde(rename = "data_source")]
    DataSource,
    #[serde(rename = "data_pipeline")]
    DataPipeline,
}

pub async fn fetch_latest_datasource_events(
    ds_events_url: &url::Url,
    aws_region: &str,
    client: &Client,
) -> Result<Vec<OffsetDateTime>, ProcessingError> {
    let mut ds_events_url = ds_events_url.clone();
    match ds_events_url.path_segments_mut() {
        Ok(mut mutable_segments) => {
            mutable_segments.push("events");
            Ok(())
        }
        Err(_) => Err(ProcessingError::UrlParseFailure(
            "data source events endpoint url is a cannot-be-a-base url.".to_string(),
        )),
    }?;
    ds_events_url
        .query_pairs_mut()
        .clear()
        .append_pair("descending_order", "true");
    let event_models = fetch_rest_model::<Vec<EventRestModel>, String>(
        ds_events_url.as_str(),
        aws_region,
        client,
        String::from(""),
        &HashMap::<String, String>::new(),
        "GET",
    )
    .await?;
    let mut event_times = vec![];
    for event in event_models {
        let event_time = OffsetDateTime::parse(event.event_time.as_str(), &Iso8601::DEFAULT)
            .map_err(|e| {
                ProcessingError::DatatimeParseFailure(format!("Failed to parse datetime:\n{:?}", e))
            })?;
        event_times.push(event_time);
    }
    Ok(event_times)
}
