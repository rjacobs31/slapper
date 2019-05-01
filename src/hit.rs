use crate::config::Config;
use crate::parse::SubstitutingUrl;
use crate::project::Auth;
use clap::{Arg, ArgMatches, SubCommand};
use http::header::CONTENT_TYPE;
use reqwest::{self, Client, Method, RequestBuilder};
use serde_json::Value;
use std::collections::HashMap;
use std::str::FromStr;
use std::time::Instant;

pub fn get_hit_subcommand<'a, 'b>() -> clap::App<'a, 'b> {
    SubCommand::with_name("hit")
        .about("Hits an endpoint in a specific project and environment")
        .arg(
            Arg::with_name("PROJECT")
                .required(true)
                .help("The project to load"),
        )
        .arg(
            Arg::with_name("ENVIRONMENT")
                .required(true)
                .help("The environment to load"),
        )
        .arg(Arg::with_name("ENDPOINT").help("The named endpoint to hit"))
        .arg(
            Arg::with_name("CUSTOM")
                .long("custom")
                .takes_value(true)
                .conflicts_with("ENDPOINT")
                .help("A custom path to hit"),
        )
        .arg(
            Arg::with_name("METHOD")
                .short("m")
                .long("method")
                .takes_value(true)
                .help("The HTTP method to use"),
        )
        .arg(
            Arg::with_name("MEDIA")
                .long("media")
                .takes_value(true)
                .help("The media type of the request"),
        )
        .arg(
            Arg::with_name("HEADER")
                .long("header")
                .takes_value(true)
                .multiple(true)
                .validator(validate_header)
                .help("Additional headers (e.g. \"subscription-key: 1234\""),
        )
        .arg(
            Arg::with_name("DATA")
                .short("d")
                .long("data")
                .takes_value(true)
                .help("The body of the request"),
        )
        .arg(
            Arg::with_name("DATA_FILE")
                .long("data-file")
                .takes_value(true)
                .conflicts_with("DATA")
                .help("File to read data from"),
        )
        .arg(Arg::with_name("URL_VALUES").help("Variable values to pass to parsed URL"))
}

fn validate_header(header: String) -> Result<(), String> {
    let h = header.as_str();
    match h.find(':') {
        Some(pos) if (0 < pos && pos < (h.len() - 1)) => Ok(()),
        _ => Err("no value after header key".into()),
    }
}

pub fn process_hit_subcommand<'a>(matches: &ArgMatches<'a>, conf: Config) {
    let project_name = matches.value_of("PROJECT").unwrap();
    let environment_name = matches.value_of("ENVIRONMENT").unwrap();
    let endpoint_name = matches.value_of("ENDPOINT").unwrap();
    let project = &conf.projects[project_name];
    let environment = &project.environments[environment_name];
    let endpoint = &project.endpoints[endpoint_name];

    let auth = match endpoint.auth {
        Some(Auth::Inherit) => match environment.auth {
            Some(Auth::Inherit) => &project.auth,
            _ => &environment.auth,
        },
        _ => &endpoint.auth,
    };

    let url_values = matches
        .values_of("URL_VALUES")
        .unwrap_or_default()
        .map(String::from)
        .collect::<Vec<_>>();
    let parsed_path = SubstitutingUrl::from_str(&endpoint.url_path).expect("could not parse URL");
    let subbed_path = parsed_path
        .sub_by_index(url_values.into_iter())
        .expect("could not sub variables");
    let url = &environment
        .base_url
        .join(&subbed_path)
        .expect("could not join path to URL");

    let method = Method::from_str(&endpoint.method).unwrap_or_default();

    let client = Client::new();
    let mut request = client.request(method, url.clone());

    let auth_start_time = Instant::now();
    request = apply_auth(request, &auth);
    let auth_end_time = Instant::now();

    let request_start_time = Instant::now();
    let mut response = request.send().unwrap();
    let request_end_time = Instant::now();

    println!(
        r#"
=================================
url:    {0}
status: {1}
=================================
auth duration:    {2:>12} ms
request duration: {3:>12} ms
---------------------------------
total:            {4:>12} ms
================================="#,
        url,
        response.status(),
        auth_end_time.duration_since(auth_start_time).as_millis(),
        request_end_time
            .duration_since(request_start_time)
            .as_millis(),
        request_end_time.duration_since(auth_start_time).as_millis()
    );
    let content = match response.headers().get(CONTENT_TYPE) {
        Some(val) if val.to_str().unwrap().contains("json") => response
            .json::<Value>()
            .map(|x| serde_json::to_string_pretty(&x))
            .unwrap()
            .unwrap(),
        _ => response.text().unwrap(),
    };
    println!("{}", content);
}

pub fn apply_auth(request: RequestBuilder, auth: &Option<Auth>) -> RequestBuilder {
    match auth {
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
