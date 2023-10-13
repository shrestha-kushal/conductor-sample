pub mod config;
pub mod entities;
pub mod events;
pub mod pipeline_effects;
pub mod signing;

use config::Config;
use events::process_lambda_event;
use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use pipeline_effects::generate_pipeline_effects;
use serde::Deserialize;
use simple_error::simple_error;
use std::env::var;

#[derive(Deserialize, Debug, Clone, PartialEq)]
enum RequestType {
    DataSource,
    Pipeline,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Request {
    source: String,
    event_id: String,
    detail: RequestDetail,
}

#[derive(Deserialize, Debug, Clone)]
pub struct RequestDetail {
    creator_type: RequestType,
    creator_id: String,
    payload: RequestPayload,
}

#[derive(Deserialize, Debug, Clone)]
pub struct RequestPayload {
    callback_token: Option<String>,
    success_time: Option<String>,
    event_time: String,
}

pub async fn handler(event: LambdaEvent<Request>) -> Result<(), Error> {
    let config = Config {
        endpoint_prefix: var("ENV_ENDPOINT_URL")
            .map_err(|_| Box::new(simple_error!("Env var ENV_ENDPOINT_URL undefined.")))?,
        aws_region: var("ENV_AWS_REGION")
            .map_err(|_| Box::new(simple_error!("Env var ENV_AWS_REGION undefined.")))?,
    };
    let processed_event = process_lambda_event(event).await?;
    generate_pipeline_effects(processed_event, &config.endpoint_prefix, &config.aws_region).await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        // disable printing the name of the module in every log line.
        .with_target(false)
        // disabling time is handy because CloudWatch will add the ingestion time.
        .without_time()
        .init();
    run(service_fn(handler)).await
}
