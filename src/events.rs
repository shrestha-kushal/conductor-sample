use crate::entities::{Event, EventPayload};
use crate::Request;
use lambda_runtime::{Error, LambdaEvent};
use simple_error::simple_error;
use time::OffsetDateTime;

#[derive(Debug)]
pub enum EventProcessingError {
    EventValidationError(String),
    EventInitializationError(String),
    EventPersistingError(String),
    EventTimeConversionError(String),
}

pub async fn process_input_event(
    _event: LambdaEvent<Request>,
) -> Result<crate::entities::Event, EventProcessingError> {
    // ommiting implementation from github.com sample version
    // replacing with below hardcoded return value
    Ok(Event {
        id: String::from("event_id"),
        event_time: OffsetDateTime::now_utc(),
        payload: EventPayload::DataSource {
            id: String::from("ds_id"),
        },
    })
}

pub async fn process_lambda_event(event: LambdaEvent<Request>) -> Result<Event, Error> {
    let event = process_input_event(event.clone())
        .await
        .map_err(|e| Box::new(simple_error!(&format!("{:?}", e))))?;
    Ok(event)
}
