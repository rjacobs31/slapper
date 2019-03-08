use reqwest;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", content = "values")]
pub enum Auth {
    Inherit,
    ClientCredentials {
        authority: String,

        client_id: String,

        client_secret: String,

        grant_type: String,

        resource: String,

        #[serde(skip_serializing_if = "Option::is_none")]
        scopes: Option<String>,
    },
}

#[derive(Serialize, Deserialize)]
pub struct Endpoint {
    pub url_path: String,

    #[serde(default = "method_default", skip_serializing_if = "skip_if_get")]
    pub method: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<Auth>,
}

impl Endpoint {
    pub fn new(url_path: &str) -> Self {
        Self {
            url_path: url_path.to_owned(),
            method: method_default(),
            auth: None,
        }
    }
}

fn method_default() -> String {
    "GET".into()
}

fn skip_if_get(value: &str) -> bool {
    value.is_empty() || value.to_uppercase() == "GET"
}

#[derive(Serialize, Deserialize)]
pub struct Environment {
    #[serde(with = "url_serde")]
    pub base_url: Url,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<Auth>,
}

impl Environment {
    pub fn new(base_url: Url) -> Self {
        Self {
            base_url,
            auth: None,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Project {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<Auth>,

    pub environments: HashMap<String, Environment>,

    pub endpoints: HashMap<String, Endpoint>,
}

impl Project {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_full<IEnvironment, IEndpoint>(
        auth: Option<Auth>,
        environments: IEnvironment,
        endpoints: IEndpoint,
    ) -> Self
    where
        IEnvironment: IntoIterator<Item = (String, Environment)>,
        IEndpoint: IntoIterator<Item = (String, Endpoint)>,
    {
        let mut env_map = HashMap::new();
        for (k, v) in environments {
            env_map.insert(k, v);
        }

        let mut end_map = HashMap::new();
        for (k, v) in endpoints {
            end_map.insert(k, v);
        }

        Self {
            auth,
            environments: env_map,
            endpoints: end_map,
        }
    }
}

impl Default for Project {
    fn default() -> Self {
        Self {
            auth: None,
            environments: HashMap::new(),
            endpoints: HashMap::new(),
        }
    }
}

pub fn do_request(
    projects: &HashMap<String, Project>,
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
    let project = &projects[project_name];
    let environment = &project.environments[env_name];
    let endpoint = &project.endpoints[endpoint_name];

    let auth = match endpoint.auth {
        Some(Auth::Inherit) => match environment.auth {
            Some(Auth::Inherit) => &project.auth,
            _ => &environment.auth,
        },
        _ => &endpoint.auth,
    };

    let url = environment.base_url.join(&endpoint.url_path).unwrap();
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
