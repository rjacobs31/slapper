use crate::parse::SubstitutingUrl;
use crate::project::{Auth, Endpoint, Environment, Project};
use http::header::CONTENT_TYPE;
use reqwest::{self, Client, Method};
use serde_json::Value;
use std::collections::HashMap;
use std::str::FromStr;
use std::time::Instant;

pub fn do_request<I>(
    project: &Project,
    environment: &Environment,
    endpoint: &Endpoint,
    url_values: I,
) -> String
where
    I: IntoIterator<Item = String>,
{
    // TODO Handle errors.
    let start_time = Instant::now();

    let auth = match endpoint.auth {
        Some(Auth::Inherit) => match environment.auth {
            Some(Auth::Inherit) => &project.auth,
            _ => &environment.auth,
        },
        _ => &endpoint.auth,
    };

    let parsed_path = SubstitutingUrl::from_str(&endpoint.url_path).expect("could not parse URL");
    let subbed_path = parsed_path
        .sub_by_index(url_values)
        .expect("could not sub variables");
    println!("{}", subbed_path);
    let url = environment
        .base_url
        .join(&subbed_path)
        .expect("could not join path to URL");
    let method = Method::from_str(&endpoint.method).unwrap_or_default();
    let client = Client::new();

    let mut request = client.request(method, url);
    request = match auth {
        Some(Auth::ClientCredentials {
            authority,
            client_id,
            client_secret,
            grant_type,
            resource,
            ..
        }) => {
            let token = get_client_credentials_token(
                authority,
                client_id,
                client_secret,
                grant_type,
                resource,
            )
            .unwrap();
            request.bearer_auth(token)
        }
        _ => request,
    };

    let auth_end_time = Instant::now();
    let mut response = request.send().unwrap();
    let request_end_time = Instant::now();
    println!(
        r#"
============================
auth duration: {} ms
request duration: {} ms
----------------------------
total: {} ms
============================"#,
        auth_end_time.duration_since(start_time).as_millis(),
        request_end_time.duration_since(auth_end_time).as_millis(),
        request_end_time.duration_since(start_time).as_millis()
    );
    match response.headers().get(CONTENT_TYPE) {
        Some(val) if val.to_str().unwrap().contains("json") => response
            .json::<Value>()
            .map(|x| serde_json::to_string_pretty(&x))
            .unwrap()
            .unwrap(),
        _ => response.text().unwrap(),
    }
}

fn get_client_credentials_token<'a>(
    authority: &'a str,
    client_id: &'a str,
    client_secret: &'a str,
    grant_type: &'a str,
    resource: &'a str,
) -> Option<String> {
    let params = &[
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("grant_type", grant_type),
        ("resource", resource),
    ];

    // TODO Handle errors.
    Client::new()
        .post(authority)
        .form(params)
        .send()
        .unwrap()
        .json::<HashMap<String, String>>()
        .unwrap()
        .get("access_token")
        .map(|t| (*t).clone())
}
