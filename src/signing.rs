use aws_credential_types::Credentials;
use aws_sigv4::http_request::{sign, SignableRequest, SigningParams, SigningSettings};
use http;
use std::collections::HashMap;
use std::time::SystemTime;

#[derive(Debug)]
pub enum SigningError {
    RequestBuildingError(String),
    CredentialsError(String),
    SettingsError(String),
    SigningError(String),
}

pub async fn get_signed_request_for_aws<T: AsRef<[u8]>>(
    uri: &str,
    headers: &HashMap<String, String>,
    method: &str,
    body: T,
    aws_region: &str,
    aws_service_name: &str,
) -> Result<http::Request<T>, SigningError> {
    let creds = _get_aws_credentials_from_env().await?;
    let signing_params = _get_signing_params(
        creds.access_key_id(),
        creds.secret_access_key(),
        creds.session_token(),
        aws_region,
        aws_service_name,
    )?;
    let mut request = _build_request(uri, method, headers, body)?;
    _apply_sigv4_to_request(&mut request, &signing_params)?;
    let output = request;
    Ok(output)
}

fn _apply_sigv4_to_request<T: AsRef<[u8]>>(
    request: &mut http::Request<T>,
    params: &SigningParams,
) -> Result<(), SigningError> {
    let signable_request = SignableRequest::from(&*request);
    match sign(signable_request, params) {
        Ok(signing_output) => {
            let (signing_instructions, _signature) = signing_output.into_parts();
            signing_instructions.apply_to_request(request);
            Ok(())
        }
        Err(e) => Err(SigningError::SigningError(format!(
            "Failed to generate sigv4 signing instructions for request.\n{:?}",
            e
        ))),
    }
}

async fn _get_aws_credentials_from_env() -> Result<Credentials, SigningError> {
    if let Some(provider) = aws_config::load_from_env().await.credentials_provider() {
        provider.as_ref().provide_credentials().await.map_err(|e| {
            SigningError::CredentialsError(format!("Failed to obtain AWS credentials.\n{:?}", e))
        })
    } else {
        Err(SigningError::CredentialsError(
            "Failed to find any AWS credentials".to_owned(),
        ))
    }
}

fn _build_request<T>(
    uri: &str,
    method: &str,
    headers: &HashMap<String, String>,
    body: T,
) -> Result<http::Request<T>, SigningError> {
    let mut request_builder = http::Request::builder()
        .uri::<&str>(uri)
        .method::<&str>(method);
    for (k, v) in headers.iter() {
        request_builder = request_builder.header::<&String, &String>(k, v);
    }
    request_builder.body(body).map_err(|e| {
        SigningError::RequestBuildingError(format!("Failed to build request.\n{:?}", e))
    })
}

fn _get_signing_params<'a>(
    aws_access_key_id: &'a str,
    aws_secret_key: &'a str,
    aws_session_token: Option<&'a str>,
    aws_region: &'a str,
    aws_service_name: &'a str,
) -> Result<SigningParams<'a>, SigningError> {
    let signing_params = SigningParams::builder()
        .access_key(aws_access_key_id)
        .secret_key(aws_secret_key)
        .region(aws_region)
        .service_name(aws_service_name)
        .time(SystemTime::now());
    let signing_params = match aws_session_token {
        Some(token) => signing_params.security_token(token),
        None => signing_params,
    };
    signing_params
        .settings(SigningSettings::default())
        .build()
        .map_err(|e| {
            SigningError::SettingsError(format!(
                "Failed to generate settings for signing AWS requests:\n{:?}",
                e
            ))
        })
}
