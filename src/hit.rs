use reqwest;
use serde::{Deserialize, Serialize};
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
        scopes: Option<&'a str>,
    },
}

#[derive(Serialize, Deserialize)]
pub struct Endpoint<'a> {
    pub name: &'a str,
    pub url: &'a str,
    pub method: &'a str,
    pub auth: Option<Auth<'a>>,
}

#[derive(Serialize, Deserialize)]
pub struct Environment<'a> {
    pub name: &'a str,
    #[serde(with = "url_serde")]
    pub base_url: Url,
    pub auth: Option<Auth<'a>>,
}

#[derive(Serialize, Deserialize)]
pub struct Project<'a> {
    pub name: &'a str,
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
    use reqwest::{Client, Method};
    use std::str::FromStr;

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

    let url = environment.base_url.join(endpoint.url).unwrap();
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

    let mut response = request.send().unwrap();
    response.text().unwrap()
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

    let client = Client::new();
    let params = &[
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("grant_type", grant_type),
        ("resource", resource),
    ];

    let mut response = client.post(authority).form(params).send().unwrap();
    let result: HashMap<String, String> = response.json().unwrap();
    result.get("token").map(|t| (*t).clone())
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
