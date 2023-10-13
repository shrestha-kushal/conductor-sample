#[cfg(test)]
mod test_endpoint;
use super::ProcessingError;
use crate::signing::get_signed_request_for_aws;
use http;
use reqwest::{self, Body, Client};
use serde::de::DeserializeOwned;
use serde_json;
use std::collections::HashMap;
use url::Url;

pub async fn fetch_rest_model<T1: DeserializeOwned, T2: AsRef<[u8]> + Into<Body>>(
    endpoint_url: &str,
    aws_region: &str,
    client: &Client,
    body: T2,
    headers: &HashMap<String, String>,
    method: &str,
) -> Result<T1, ProcessingError> {
    let http_request = get_signed_request_for_aws(
        endpoint_url,
        headers,
        method,
        body,
        aws_region,
        "execute-api",
    )
    .await
    .map_err(|e| {
        ProcessingError::ModelFetchFailure(format!("Failed to fetch data pipeline model:\n{:?}", e))
    })?;
    let response_text = _fetch_response_text(http_request, client).await?;
    serde_json::from_str::<T1>(&response_text).map_err(|e| {
        ProcessingError::ModelFetchFailure(format!(
            "Failed to deserialize response from remote api:\n{:?}\n{}",
            e, &response_text
        ))
    })
}

pub fn basenames(paths: &Vec<String>) -> Vec<String> {
    paths.iter().map(|path| _basename(path)).collect()
}

fn _basename(path: &str) -> String {
    if let Some(terminus) = path.split("/").last() {
        String::from(terminus)
    } else {
        String::from(path) // technically this will never happen
    }
}

async fn _fetch_response_text<T: Into<Body>>(
    http_request: http::Request<T>,
    client: &Client,
) -> Result<String, ProcessingError> {
    let request = reqwest::Request::try_from(http_request).map_err(|e| {
        ProcessingError::ModelFetchFailure(format!("Failed to construct http request:\n{:?}", e))
    })?;
    let response = client.execute(request).await.map_err(|e| {
        ProcessingError::ModelFetchFailure(format!(
            "Failed to complete request to fetch remote model:\n{:?}",
            e
        ))
    })?;
    response.text().await.map_err(|e| {
        ProcessingError::ModelFetchFailure(format!(
            "Failed to convert response from remote api to text:\n{:?}",
            e
        ))
    })
}

// TODO: add ability to specify query params
pub fn construct_endpoint_url(
    endpoint_prefix: &str,
    path_extension: &Vec<String>,
) -> Result<String, ProcessingError> {
    let begin_url = Url::parse(endpoint_prefix).map_err(|e| {
        ProcessingError::UrlParseFailure(format!("Endpoint prefix may not be valid url:\n{:?}", e))
    })?;
    let endpoint_url = Ok(_get_segments_count(&begin_url))
        .and_then(|n| _drop_n_trailing_slashes(begin_url, n))
        .and_then(|url| _append_to_path(url, path_extension))?;
    Ok(endpoint_url.as_str().to_string())
}

fn _get_segments_count(url: &Url) -> usize {
    if let Some(split) = url.path_segments() {
        split.count()
    } else {
        0
    }
}

fn _drop_n_trailing_slashes(url: Url, n: usize) -> Result<Url, ProcessingError> {
    let mut url = url;
    let mut segment_mutator = url.path_segments_mut().map_err(|e| {
        ProcessingError::UrlParseFailure(format!(
            "Endpoint prefix might be a cannot-be-base URL:\n{:?}",
            e
        ))
    })?;
    for _ in 0..n {
        segment_mutator.pop_if_empty();
    }
    drop(segment_mutator);
    Ok(url)
}

fn _append_to_path(url: Url, path_extension: &Vec<String>) -> Result<Url, ProcessingError> {
    let mut url = url;
    let mut segment_mutator = url.path_segments_mut().map_err(|e| {
        ProcessingError::UrlParseFailure(format!(
            "Endpoint prefix might be a cannot-be-base URL:\n{:?}",
            e
        ))
    })?;
    segment_mutator.extend(path_extension);
    drop(segment_mutator);
    Ok(url)
}
