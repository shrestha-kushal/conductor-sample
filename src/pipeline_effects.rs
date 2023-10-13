mod endpoint;
mod gathering;
mod triggering;

use crate::entities::Event;
use gathering::get_data_pipelines;
use lambda_runtime::Error;
use reqwest::Client;
use simple_error::simple_error;
use time::OffsetDateTime;
use triggering::maybe_trigger_pipeline;
use url::Url;

#[derive(Debug)]
pub enum ProcessingError {
    UnrecognizedTriggerType(String),
    MissingSuccessTime(String),
    SuccessTimeConflict(String),
    MissingPermitContent(String),
    PermitContentConflict(String),
    DatatimeParseFailure(String),
    UrlParseFailure(String),
    ModelFetchFailure(String),
    MissingPipelinePermit(String),
    StateMachineFetchingError(String),
    RelayTaskSuccessError(String),
    RelayTaskHeartbeatError(String),
    PipelineStateMachineMissing(String),
}

pub struct DataPipeline {
    id: String,
    description: String,
    last_success_time: Option<OffsetDateTime>,
    permit: TriggerPermitType,
    dependency_urls: Vec<Url>,
}

enum TriggerPermitType {
    Lenient(Option<PipelineTriggerPermit>),
    Strict(Option<PipelineTriggerPermit>),
}

struct PipelineTriggerPermit {
    content: String,
    is_expired: bool,
}

pub async fn generate_pipeline_effects(
    event: Event,
    endpoint_prefix: &str,
    aws_region: &str,
) -> Result<(), Error> {
    let mut error_strings = vec![];
    let client = Client::new();
    let relevant_pipelines =
        get_data_pipelines(event.payload, endpoint_prefix, aws_region, &client)
            .await
            .map_err(|e| Box::new(simple_error!(format!("{:?}", e))))?;
    for data_pipeline in &relevant_pipelines {
        match maybe_trigger_pipeline(data_pipeline, aws_region, &client).await {
            Ok(_) => {}
            Err(error) => {
                error_strings.push(format!("{:?}", error));
            }
        }
    }
    if error_strings.len() > 0 {
        Err(Box::new(simple_error!(format!(
            "Not all relevant pipelines were successfully triggered.\n{:?}",
            error_strings
        ))))
    } else {
        Ok(())
    }
}
