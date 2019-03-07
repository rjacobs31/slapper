use reqwest;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", content = "values")]
pub enum Auth<'a> {
    Inherit,
    ClientCredentials {
        authority: &'a str,

        client_id: &'a str,

        client_secret: &'a str,

        grant_type: &'a str,

        resource: &'a str,

        #[serde(skip_serializing_if = "Option::is_none")]
        scopes: Option<&'a str>,
    },
}

#[derive(Serialize, Deserialize)]
pub struct Endpoint<'a> {
    pub name: &'a str,

    pub url: String,

    #[serde(default = "method_default", skip_serializing_if = "skip_if_get")]
    pub method: &'a str,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<Auth<'a>>,
}

const fn method_default() -> &'static str {
    "GET"
}

fn skip_if_get(value: &str) -> bool {
    value.is_empty() || value.to_uppercase() == "GET"
}

#[derive(Serialize, Deserialize)]
pub struct Environment<'a> {
    pub name: &'a str,

    #[serde(with = "url_serde")]
    pub base_url: Url,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<Auth<'a>>,
}

#[derive(Serialize, Deserialize)]
pub struct Project<'a> {
    pub name: &'a str,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<Auth<'a>>,

    pub environments: Vec<Environment<'a>>,

    pub endpoints: Vec<Endpoint<'a>>,
}

pub fn do_request(
    projects: &[Project],
    project_name: &str,
    env_name: &str,
    endpoint_name: &str,
) -> String {
    use http::header::CONTENT_TYPE;
    use reqwest::{Client, Method};
    use serde_json::Value;
    use std::str::FromStr;
    use std::time::Instant;

    // TODO Handle errors.
    let start_time = Instant::now();
    let project = projects.iter().find(|&p| p.name == project_name).unwrap();
    let environment = project
        .environments
        .iter()
        .find(|&e| e.name == env_name)
        .unwrap();
    let endpoint = project
        .endpoints
        .iter()
        .find(|&e| e.name == endpoint_name)
        .unwrap();

    let auth = match endpoint.auth {
        Some(Auth::Inherit) => match environment.auth {
            Some(Auth::Inherit) => &project.auth,
            _ => &environment.auth,
        },
        _ => &endpoint.auth,
    };

    let url = environment.base_url.join(&endpoint.url).unwrap();
    let method = Method::from_str(endpoint.method).unwrap_or(Method::default());
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
    use reqwest::Client;
    use std::collections::HashMap;

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

fn url_replace<'a>(url: &'a str, values: HashMap<String, String>) -> Result<String, &'a str> {
    use std::fmt::Write;

    let find_interest = |input: &str| input.find(|x| x == '\\' || x == '{');
    let pos = find_interest(url);
    if pos.is_none() {
        return Ok(url.to_owned());
    }

    let mut generated_url = String::new();
    let (prefix, mut remainder) = url.split_at(pos.unwrap());
    generated_url.write_str(prefix).unwrap();

    while !remainder.is_empty() {
        if remainder.starts_with(r"\{") {
            generated_url.push('{');
            remainder = remainder.split_at(br"\{".len()).1;
        } else if remainder.starts_with('{') {
            remainder = remainder.split_at(br"{".len()).1;
            if let Some(pos) = remainder.find('}') {
                let res = remainder.split_at(pos);
                let name = res.0;
                let val = values
                    .get(&name.to_owned())
                    .expect(format!("Could not find value: {}", name).as_str());
                generated_url.write_str(val).unwrap();
                remainder = remainder.split_at(name.len() + br"}".len()).1;
            } else {
                return Err("Unterminated variable tag");
            }
        } else {
            let pos = find_interest(url);
            if pos.is_none() {
                generated_url.write_str(remainder).unwrap();
                break;
            }

            let res = url.split_at(pos.unwrap());
            let prefix = res.0;
            remainder = res.1;
            generated_url.write_str(prefix).unwrap();
        }
    }
    Ok(generated_url)
}

mod url_serde {
    use serde::{
        de::{self, Visitor},
        Deserializer, Serializer,
    };
    use std::fmt;
    use url::Url;

    struct UrlVisitor;

    impl<'de> Visitor<'de> for UrlVisitor {
        type Value = Url;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string value to parse as URL")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            match Url::parse(value) {
                Ok(parsed) => Ok(parsed),
                Err(_) => Err(E::custom(format!("could not parse URL: {}", value))),
            }
        }
    }

    pub fn serialize<S>(url: &Url, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(url.as_str())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Url, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(UrlVisitor)
    }
}
