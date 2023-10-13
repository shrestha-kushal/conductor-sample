use time::OffsetDateTime;

pub enum EventPayload {
    DataSource {
        id: String,
    },
    DataPipeline {
        id: String,
        success_time: OffsetDateTime,
        callback_token: String,
    },
}

pub struct Event {
    pub id: String,
    pub event_time: OffsetDateTime,
    pub payload: EventPayload,
}
