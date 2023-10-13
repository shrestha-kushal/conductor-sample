use super::*;
use crate::entities::EventPayload;
use crate::pipeline_effects::gathering::TriggerPermitType;
use crate::pipeline_effects::ProcessingError;
use rand::thread_rng;
use rand::{self, Rng};
use time::format_description::well_known::iso8601::Iso8601;
use time::{self, OffsetDateTime};

#[tokio::test]
async fn gather_data_pipelines_test_happy_path_data_source_event_one_dependent_pipeline() {
    let rand_num_source_id = rand::random::<u32>();
    let rand_num_pipeline_id = rand::random::<u32>();
    let mut rng = thread_rng();
    let unix_ts = rng.gen_range(338250323..1663680521);
    let event_payload = EventPayload::DataSource {
        id: format!("source{}", rand_num_source_id),
    };
    let data_source_read_dummy_fn = |id: String| async move {
        let expected_id = format!("source{}", rand_num_source_id);
        assert!(id.eq(&expected_id));
        Ok(DataSourceRestModel {
            id: expected_id,
            description: "Some data source description".to_string(),
            dependent_pipelines: vec![format!("pipeline{}", rand_num_pipeline_id)],
        })
    };
    let data_pipeline_read_dummy_fn = |id: String| async move {
        let expected_id = format!("pipeline{}", rand_num_pipeline_id);
        assert!(id.eq(&expected_id));
        // NOTE: use below snippet to print out desired ENCODED_DT_FMT
        // use time::format_description::well_known::iso8601::{Config, TimePrecision};
        // use std::num::NonZeroU8;
        // let dt_fmt_config = Config::DEFAULT.set_time_precision(TimePrecision::Second {
        //     decimal_digits: NonZeroU8::new(6),
        // });
        // let dt_fmt = dt_fmt_config.encode();
        // println!("encoded format: {}", dt_fmt);
        const ENCODED_DT_FMT: u128 = 6651332276409342489074426579873955840u128;
        let dt = OffsetDateTime::from_unix_timestamp(unix_ts).unwrap();
        Ok(PipelineRestModel {
            id: String::from(&id),
            description: format!("pipeline: {}", &id),
            last_success_time: Some(dt.format(&Iso8601::<ENCODED_DT_FMT>).unwrap()),
            source_dependencies: vec![format!(
                "https://api.hotpotato.com/v1/source{}",
                rand_num_source_id
            )],
            trigger_rule: String::from("LENIENT"),
            callback_token: None,
        })
    };
    let result = gather_data_pipelines(
        event_payload,
        data_source_read_dummy_fn,
        data_pipeline_read_dummy_fn,
    )
    .await;
    assert!(result.is_ok());
    let data_pipelines = result.unwrap();
    assert_eq!(data_pipelines.len(), 1);
    let data_pipeline = &data_pipelines[0];
    assert_eq!(
        data_pipeline.id,
        format!("pipeline{}", rand_num_pipeline_id)
    );
    assert_eq!(
        data_pipeline.description,
        format!("pipeline: pipeline{}", rand_num_pipeline_id)
    );
    assert_eq!(
        data_pipeline.last_success_time.unwrap(),
        OffsetDateTime::from_unix_timestamp(unix_ts).unwrap()
    );
    assert!(
        if let TriggerPermitType::Lenient(None) = data_pipeline.permit {
            true
        } else {
            false
        }
    );
    assert_eq!(data_pipeline.dependency_urls.len(), 1);
    assert_eq!(
        data_pipeline.dependency_urls[0],
        Url::parse(&(format!("https://api.hotpotato.com/v1/source{}", rand_num_source_id)))
            .unwrap()
    );
}

#[tokio::test]
async fn gather_data_pipelines_test_happy_path_data_source_event_two_dependent_pipelines() {
    let rand_num_source_id = rand::random::<u32>();
    let first_rand_num_pipeline_id = rand::random::<u32>();
    let second_rand_num_pipeline_id = rand::random::<u32>();
    let rand_num_token = rand::random::<u32>();
    let mut rng = thread_rng();
    let unix_ts = rng.gen_range(338250323..1663680521);
    let event_payload = EventPayload::DataSource {
        id: format!("source{}", rand_num_source_id),
    };
    let data_source_read_dummy_fn = |id: String| async move {
        let expected_id = format!("source{}", rand_num_source_id);
        assert!(id.eq(&expected_id));
        Ok(DataSourceRestModel {
            id: expected_id,
            description: "Some data source description".to_string(),
            dependent_pipelines: vec![
                format!("pipeline{}", first_rand_num_pipeline_id),
                format!("pipeline{}", second_rand_num_pipeline_id),
            ],
        })
    };
    let data_pipeline_read_dummy_fn = |id: String| async move {
        let expected_id1 = format!("pipeline{}", first_rand_num_pipeline_id);
        let expected_id2 = format!("pipeline{}", second_rand_num_pipeline_id);
        assert!(id.eq(&expected_id1) || id.eq(&expected_id2));
        const ENCODED_DT_FMT: u128 = 6651332276409342489074426579873955840u128;
        let dt = OffsetDateTime::from_unix_timestamp(unix_ts).unwrap();
        let (dt_option, token_option) = if id.eq(&expected_id1) {
            (
                Some(dt.format(&Iso8601::<ENCODED_DT_FMT>).unwrap()),
                Some(format!("token{}", rand_num_token)),
            )
        } else {
            (None, None)
        };
        Ok(PipelineRestModel {
            id: String::from(&id),
            description: format!("pipeline: {}", &id),
            last_success_time: dt_option,
            source_dependencies: vec![format!(
                "https://api.hotpotato.com/v1/source{}",
                rand_num_source_id
            )],
            trigger_rule: String::from("STRICT"),
            callback_token: token_option,
        })
    };
    let result = gather_data_pipelines(
        event_payload,
        data_source_read_dummy_fn,
        data_pipeline_read_dummy_fn,
    )
    .await;
    assert!(result.is_ok());
    let data_pipelines = result.unwrap();
    assert_eq!(data_pipelines.len(), 2);
    // preserve the order in DataSourceRestModel.dependent_pipelines. may be too restrictive?
    let first_data_pipeline = &data_pipelines[0];
    assert_eq!(
        first_data_pipeline.id,
        format!("pipeline{}", first_rand_num_pipeline_id)
    );
    assert_eq!(
        first_data_pipeline.description,
        format!("pipeline: pipeline{}", first_rand_num_pipeline_id)
    );
    assert_eq!(
        first_data_pipeline.last_success_time.unwrap(),
        OffsetDateTime::from_unix_timestamp(unix_ts).unwrap()
    );
    if let TriggerPermitType::Strict(Some(permit)) = &first_data_pipeline.permit {
        assert_eq!(permit.content, format!("token{}", rand_num_token));
    } else {
        assert!(false);
    };
    assert_eq!(first_data_pipeline.dependency_urls.len(), 1);
    assert_eq!(
        first_data_pipeline.dependency_urls[0],
        Url::parse(&(format!("https://api.hotpotato.com/v1/source{}", rand_num_source_id)))
            .unwrap()
    );
    let second_data_pipeline = &data_pipelines[1];
    assert_eq!(
        second_data_pipeline.id,
        format!("pipeline{}", second_rand_num_pipeline_id)
    );
    assert_eq!(
        second_data_pipeline.description,
        format!("pipeline: pipeline{}", second_rand_num_pipeline_id)
    );
    assert_eq!(second_data_pipeline.last_success_time, None);

    if let TriggerPermitType::Strict(x) = &second_data_pipeline.permit {
        assert!(x.is_none())
    } else {
        assert!(false)
    };
    assert_eq!(second_data_pipeline.dependency_urls.len(), 1);
    assert_eq!(
        second_data_pipeline.dependency_urls[0],
        Url::parse(&(format!("https://api.hotpotato.com/v1/source{}", rand_num_source_id)))
            .unwrap()
    );
}

#[tokio::test]
async fn gather_data_pipelines_test_happy_path_lenient_trigger_some_token() {
    let rand_num_source_id = rand::random::<u32>();
    let rand_num_pipeline_id = rand::random::<u32>();
    let rand_num_token = rand::random::<u32>();
    let mut rng = thread_rng();
    let unix_ts = rng.gen_range(338250323..1663680521);
    let event_payload = EventPayload::DataSource {
        id: format!("source{}", rand_num_source_id),
    };
    let data_source_read_dummy_fn = |id: String| async move {
        let expected_id = format!("source{}", rand_num_source_id);
        assert!(id.eq(&expected_id));
        Ok(DataSourceRestModel {
            id: expected_id,
            description: "Some data source description".to_string(),
            dependent_pipelines: vec![format!("pipeline{}", rand_num_pipeline_id)],
        })
    };
    let data_pipeline_read_dummy_fn = |id: String| async move {
        let expected_id = format!("pipeline{}", rand_num_pipeline_id);
        assert!(id.eq(&expected_id));
        const ENCODED_DT_FMT: u128 = 6651332276409342489074426579873955840u128;
        let dt = OffsetDateTime::from_unix_timestamp(unix_ts).unwrap();
        Ok(PipelineRestModel {
            id: String::from(&id),
            description: format!("pipeline: {}", &id),
            last_success_time: Some(dt.format(&Iso8601::<ENCODED_DT_FMT>).unwrap()),
            source_dependencies: vec![format!(
                "https://api.hotpotato.com/v1/source{}",
                rand_num_source_id
            )],
            trigger_rule: String::from("LENIENT"),
            callback_token: Some(format!("token{}", rand_num_token)),
        })
    };
    let result = gather_data_pipelines(
        event_payload,
        data_source_read_dummy_fn,
        data_pipeline_read_dummy_fn,
    )
    .await;
    assert!(result.is_ok());
    let data_pipelines = result.unwrap();
    assert_eq!(data_pipelines.len(), 1);
    let data_pipeline = &data_pipelines[0];
    if let TriggerPermitType::Lenient(Some(permit)) = &data_pipeline.permit {
        assert_eq!(permit.content, format!("token{}", rand_num_token));
    } else {
        assert!(false);
    };
}

#[tokio::test]
async fn gather_data_pipelines_test_unrecognized_trigger_type() {
    let rand_num_source_id = rand::random::<u32>();
    let rand_num_pipeline_id = rand::random::<u32>();
    let rand_num_token = rand::random::<u32>();
    let mut rng = thread_rng();
    let unix_ts = rng.gen_range(338250323..1663680521);
    let event_payload = EventPayload::DataSource {
        id: format!("source{}", rand_num_source_id),
    };
    let data_source_read_dummy_fn = |id: String| async move {
        let expected_id = format!("source{}", rand_num_source_id);
        assert!(id.eq(&expected_id));
        Ok(DataSourceRestModel {
            id: expected_id,
            description: "Some data source description".to_string(),
            dependent_pipelines: vec![format!("pipeline{}", rand_num_pipeline_id)],
        })
    };
    let data_pipeline_read_dummy_fn = |id: String| async move {
        let expected_id = format!("pipeline{}", rand_num_pipeline_id);
        assert!(id.eq(&expected_id));
        // NOTE: use below snippet to print out desired ENCODED_DT_FMT
        // use time::format_description::well_known::iso8601::{Config, TimePrecision};
        // use std::num::NonZeroU8;
        // let dt_fmt_config = Config::DEFAULT.set_time_precision(TimePrecision::Second {
        //     decimal_digits: NonZeroU8::new(6),
        // });
        // let dt_fmt = dt_fmt_config.encode();
        // println!("encoded format: {}", dt_fmt);
        const ENCODED_DT_FMT: u128 = 6651332276409342489074426579873955840u128;
        let dt = OffsetDateTime::from_unix_timestamp(unix_ts).unwrap();
        Ok(PipelineRestModel {
            id: String::from(&id),
            description: format!("pipeline: {}", &id),
            last_success_time: Some(dt.format(&Iso8601::<ENCODED_DT_FMT>).unwrap()),
            source_dependencies: vec![format!(
                "https://api.hotpotato.com/v1/source{}",
                rand_num_source_id
            )],
            trigger_rule: String::from(format!("badtype{}", rand_num_pipeline_id)),
            callback_token: Some(format!("token{}", rand_num_token)),
        })
    };
    let result = gather_data_pipelines(
        event_payload,
        data_source_read_dummy_fn,
        data_pipeline_read_dummy_fn,
    )
    .await;
    assert!(result.is_err());
    if let Err(ProcessingError::UnrecognizedTriggerType(s)) = result {
        assert_eq!(s, "Error: Unrecognized trigger permit type.")
    } else {
        assert!(false)
    };
}

#[tokio::test]
async fn gather_data_pipelines_test_happy_path_data_pipeline_event_single_upstream_source() {
    let rand_num_source_id = rand::random::<u32>();
    let rand_num_pipeline_id = rand::random::<u32>();
    let rand_num_token = rand::random::<u32>();
    let rand_num_desc = rand::random::<u32>();
    let mut rng = thread_rng();
    let unix_ts = rng.gen_range(338250323..1663680521);
    let dt = OffsetDateTime::from_unix_timestamp(unix_ts).unwrap();
    let event_payload = EventPayload::DataPipeline {
        id: format!("pipeline{}", rand_num_pipeline_id),
        success_time: dt,
        callback_token: format!("token{}", rand_num_token),
    };
    let data_source_read_dummy_fn = |id: String| async move {
        let expected_id = format!("source{}", rand_num_source_id);
        assert!(id.eq(&expected_id));
        Ok::<DataSourceRestModel, ProcessingError>(DataSourceRestModel {
            id: expected_id,
            description: "Some data source description".to_string(),
            dependent_pipelines: vec![format!("pipeline{}", rand_num_pipeline_id)],
        })
    };
    let data_pipeline_read_dummy_fn = |id: String| async move {
        let expected_id = format!("pipeline{}", rand_num_pipeline_id);
        assert!(id.eq(&expected_id));
        const ENCODED_DT_FMT: u128 = 6651332276409342489074426579873955840u128;
        let dt = OffsetDateTime::from_unix_timestamp(unix_ts).unwrap();
        Ok::<PipelineRestModel, ProcessingError>(PipelineRestModel {
            id: String::from(&id),
            description: format!("pipeline description {}: {}", rand_num_desc, &id),
            last_success_time: Some(dt.format(&Iso8601::<ENCODED_DT_FMT>).unwrap()),
            source_dependencies: vec![format!(
                "https://api.hotpotato.com/v1/source{}",
                rand_num_source_id
            )],
            trigger_rule: String::from("LENIENT"),
            callback_token: Some(format!("token{}", rand_num_token)),
        })
    };
    let result = gather_data_pipelines(
        event_payload,
        data_source_read_dummy_fn,
        data_pipeline_read_dummy_fn,
    )
    .await;
    if let Err(error) = &result {
        println!("{:?}", error);
    }
    assert!(result.is_ok());
    let data_pipelines = result.unwrap();
    assert_eq!(data_pipelines.len(), 1);
    let data_pipeline = &data_pipelines[0];
    assert_eq!(
        data_pipeline.id,
        format!("pipeline{}", rand_num_pipeline_id)
    );
    assert_eq!(
        data_pipeline.description,
        format!(
            "pipeline description {}: pipeline{}",
            rand_num_desc, rand_num_pipeline_id
        )
    );
    assert_eq!(
        data_pipeline.last_success_time.unwrap(),
        OffsetDateTime::from_unix_timestamp(unix_ts).unwrap()
    );
    if let TriggerPermitType::Lenient(Some(permit)) = &data_pipeline.permit {
        assert_eq!(permit.content, format!("token{}", rand_num_token));
    } else {
        assert!(false);
    };
    assert_eq!(data_pipeline.dependency_urls.len(), 1);
    assert_eq!(
        data_pipeline.dependency_urls[0],
        Url::parse(&(format!("https://api.hotpotato.com/v1/source{}", rand_num_source_id)))
            .unwrap()
    );
}

#[tokio::test]
async fn gather_data_pipelines_test_missing_success_time_in_db() {
    let rand_num_source_id = rand::random::<u32>();
    let rand_num_pipeline_id = rand::random::<u32>();
    let rand_num_token = rand::random::<u32>();
    let rand_num_desc = rand::random::<u32>();
    let mut rng = thread_rng();
    let unix_ts = rng.gen_range(338250323..1663680521);
    let dt = OffsetDateTime::from_unix_timestamp(unix_ts).unwrap();
    let event_payload = EventPayload::DataPipeline {
        id: format!("pipeline{}", rand_num_pipeline_id),
        success_time: dt,
        callback_token: format!("token{}", rand_num_token),
    };
    let data_source_read_dummy_fn = |id: String| async move {
        let expected_id = format!("source{}", rand_num_source_id);
        assert!(id.eq(&expected_id));
        Ok::<DataSourceRestModel, ProcessingError>(DataSourceRestModel {
            id: expected_id,
            description: "Some data source description".to_string(),
            dependent_pipelines: vec![format!("pipeline{}", rand_num_pipeline_id)],
        })
    };
    let data_pipeline_read_dummy_fn = |id: String| async move {
        let expected_id = format!("pipeline{}", rand_num_pipeline_id);
        assert!(id.eq(&expected_id));
        const ENCODED_DT_FMT: u128 = 6651332276409342489074426579873955840u128;
        let _dt = OffsetDateTime::from_unix_timestamp(unix_ts).unwrap();
        Ok::<PipelineRestModel, ProcessingError>(PipelineRestModel {
            id: String::from(&id),
            description: format!("pipeline description {}: {}", rand_num_desc, &id),
            last_success_time: None,
            source_dependencies: vec![format!(
                "https://api.hotpotato.com/v1/source{}",
                rand_num_source_id
            )],
            trigger_rule: String::from("LENIENT"),
            callback_token: Some(format!("token{}", rand_num_token)),
        })
    };
    let result = gather_data_pipelines(
        event_payload,
        data_source_read_dummy_fn,
        data_pipeline_read_dummy_fn,
    )
    .await;
    assert!(result.is_err());
    if let Err(ProcessingError::MissingSuccessTime(s)) = result {
        assert_eq!(
            s,
            "Last success time should have been stored; found nothing."
        )
    } else {
        assert!(false)
    };
}

#[tokio::test]
async fn gather_data_pipelines_test_success_time_conflict_in_db() {
    let rand_num_source_id = rand::random::<u32>();
    let rand_num_pipeline_id = rand::random::<u32>();
    let rand_num_token = rand::random::<u32>();
    let rand_num_desc = rand::random::<u32>();
    let mut rng = thread_rng();
    let unix_ts = rng.gen_range(338250323..1663680521);
    let dt = OffsetDateTime::from_unix_timestamp(unix_ts).unwrap();
    let event_payload = EventPayload::DataPipeline {
        id: format!("pipeline{}", rand_num_pipeline_id),
        success_time: dt,
        callback_token: format!("token{}", rand_num_token),
    };
    let data_source_read_dummy_fn = |id: String| async move {
        let expected_id = format!("source{}", rand_num_source_id);
        assert!(id.eq(&expected_id));
        Ok::<DataSourceRestModel, ProcessingError>(DataSourceRestModel {
            id: expected_id,
            description: "Some data source description".to_string(),
            dependent_pipelines: vec![format!("pipeline{}", rand_num_pipeline_id)],
        })
    };
    let data_pipeline_read_dummy_fn = |id: String| async move {
        let expected_id = format!("pipeline{}", rand_num_pipeline_id);
        assert!(id.eq(&expected_id));
        const ENCODED_DT_FMT: u128 = 6651332276409342489074426579873955840u128;
        let dt2 = OffsetDateTime::from_unix_timestamp(unix_ts + 1).unwrap();
        Ok::<PipelineRestModel, ProcessingError>(PipelineRestModel {
            id: String::from(&id),
            description: format!("pipeline description {}: {}", rand_num_desc, &id),
            last_success_time: Some(dt2.format(&Iso8601::<ENCODED_DT_FMT>).unwrap()),
            source_dependencies: vec![format!(
                "https://api.hotpotato.com/v1/source{}",
                rand_num_source_id
            )],
            trigger_rule: String::from("LENIENT"),
            callback_token: Some(format!("token{}", rand_num_token)),
        })
    };
    let result = gather_data_pipelines(
        event_payload,
        data_source_read_dummy_fn,
        data_pipeline_read_dummy_fn,
    )
    .await;
    assert!(result.is_err());
    if let Err(ProcessingError::SuccessTimeConflict(s)) = result {
        assert_eq!(
            s,
            "Conflict between provided success time and stored success time."
        )
    } else {
        assert!(false)
    };
}

#[tokio::test]
async fn gather_data_pipelines_test_missing_permit_content_in_db() {
    let rand_num_source_id = rand::random::<u32>();
    let rand_num_pipeline_id = rand::random::<u32>();
    let rand_num_token = rand::random::<u32>();
    let rand_num_desc = rand::random::<u32>();
    let mut rng = thread_rng();
    let unix_ts = rng.gen_range(338250323..1663680521);
    let dt = OffsetDateTime::from_unix_timestamp(unix_ts).unwrap();
    let event_payload = EventPayload::DataPipeline {
        id: format!("pipeline{}", rand_num_pipeline_id),
        success_time: dt,
        callback_token: format!("token{}", rand_num_token),
    };
    let data_source_read_dummy_fn = |id: String| async move {
        let expected_id = format!("source{}", rand_num_source_id);
        assert!(id.eq(&expected_id));
        Ok::<DataSourceRestModel, ProcessingError>(DataSourceRestModel {
            id: expected_id,
            description: "Some data source description".to_string(),
            dependent_pipelines: vec![format!("pipeline{}", rand_num_pipeline_id)],
        })
    };
    let data_pipeline_read_dummy_fn = |id: String| async move {
        let expected_id = format!("pipeline{}", rand_num_pipeline_id);
        assert!(id.eq(&expected_id));
        const ENCODED_DT_FMT: u128 = 6651332276409342489074426579873955840u128;
        let dt2 = OffsetDateTime::from_unix_timestamp(unix_ts).unwrap();
        Ok::<PipelineRestModel, ProcessingError>(PipelineRestModel {
            id: String::from(&id),
            description: format!("pipeline description {}: {}", rand_num_desc, &id),
            last_success_time: Some(dt2.format(&Iso8601::<ENCODED_DT_FMT>).unwrap()),
            source_dependencies: vec![format!(
                "https://api.hotpotato.com/v1/source{}",
                rand_num_source_id
            )],
            trigger_rule: String::from("LENIENT"),
            callback_token: None,
        })
    };
    let result = gather_data_pipelines(
        event_payload,
        data_source_read_dummy_fn,
        data_pipeline_read_dummy_fn,
    )
    .await;
    assert!(result.is_err());
    if let Err(ProcessingError::MissingPermitContent(s)) = result {
        assert_eq!(s, "Permit content should have been stored; found nothing.")
    } else {
        assert!(false)
    };
}

#[tokio::test]
async fn gather_data_pipelines_test_permit_content_conflict_in_db() {
    let rand_num_source_id = rand::random::<u32>();
    let rand_num_pipeline_id = rand::random::<u32>();
    let rand_num_token = rand::random::<u32>();
    let rand_num_desc = rand::random::<u32>();
    let mut rng = thread_rng();
    let unix_ts = rng.gen_range(338250323..1663680521);
    let dt = OffsetDateTime::from_unix_timestamp(unix_ts).unwrap();
    let event_payload = EventPayload::DataPipeline {
        id: format!("pipeline{}", rand_num_pipeline_id),
        success_time: dt,
        callback_token: format!("token{}", rand_num_token),
    };
    let data_source_read_dummy_fn = |id: String| async move {
        let expected_id = format!("source{}", rand_num_source_id);
        assert!(id.eq(&expected_id));
        Ok::<DataSourceRestModel, ProcessingError>(DataSourceRestModel {
            id: expected_id,
            description: "Some data source description".to_string(),
            dependent_pipelines: vec![format!("pipeline{}", rand_num_pipeline_id)],
        })
    };
    let data_pipeline_read_dummy_fn = |id: String| async move {
        let expected_id = format!("pipeline{}", rand_num_pipeline_id);
        assert!(id.eq(&expected_id));
        const ENCODED_DT_FMT: u128 = 6651332276409342489074426579873955840u128;
        let dt2 = OffsetDateTime::from_unix_timestamp(unix_ts).unwrap();
        Ok::<PipelineRestModel, ProcessingError>(PipelineRestModel {
            id: String::from(&id),
            description: format!("pipeline description {}: {}", rand_num_desc, &id),
            last_success_time: Some(dt2.format(&Iso8601::<ENCODED_DT_FMT>).unwrap()),
            source_dependencies: vec![format!(
                "https://api.hotpotato.com/v1/source{}",
                rand_num_source_id
            )],
            trigger_rule: String::from("LENIENT"),
            callback_token: Some(format!("tokens{}", rand_num_token)),
        })
    };
    let result = gather_data_pipelines(
        event_payload,
        data_source_read_dummy_fn,
        data_pipeline_read_dummy_fn,
    )
    .await;
    assert!(result.is_err());
    if let Err(ProcessingError::PermitContentConflict(s)) = result {
        assert_eq!(
            s,
            "Conflict between provided permit content and stored permit content."
        )
    } else {
        assert!(false)
    };
}
