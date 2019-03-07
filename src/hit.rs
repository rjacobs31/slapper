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
    pub name: String,

    pub url: String,

    #[serde(default = "method_default", skip_serializing_if = "skip_if_get")]
    pub method: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<Auth>,
}

fn method_default() -> String {
    "GET".into()
}

fn skip_if_get(value: &str) -> bool {
    value.is_empty() || value.to_uppercase() == "GET"
}

#[derive(Serialize, Deserialize)]
pub struct Environment {
    pub name: String,

    #[serde(with = "url_serde")]
    pub base_url: Url,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<Auth>,
}

#[derive(Serialize, Deserialize)]
pub struct Project {
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<Auth>,

    pub environments: Vec<Environment>,

    pub endpoints: Vec<Endpoint>,
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
    let method = Method::from_str(&endpoint.method).unwrap_or(Method::default());
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

pub fn url_replace<'a>(url: &'a str, values: &HashMap<String, String>) -> Result<String, &'a str> {
    let pos = find_interest(url);
    if pos.is_none() {
        return Ok(url.to_owned());
    }

    let (to_add, remainder) = url.split_at(pos.unwrap());
    url_replace_inner(String::new(), to_add, remainder, values)
}

fn find_interest(input: &str) -> Option<usize> {
    input.find(|x| x == '\\' || x == '{')
}

fn url_replace_inner<'a>(
    result: String,
    to_add: &str,
    remainder: &str,
    values: &HashMap<String, String>,
) -> Result<String, &'a str> {
    use std::fmt::Write;

    let mut result = String::from(result);
    result.write_str(to_add).expect("could not write string");
    if remainder.is_empty() {
        return Ok(result);
    }

    if remainder.starts_with(r"\{") {
        let (to_add, remainder) = remainder.split_at(br"\{".len());
        return url_replace_inner(result, to_add, remainder, values);
    } else if remainder.starts_with('{') {
        let (_, remainder) = remainder.split_at(br"{".len());
        if let Some(pos) = remainder.find('}') {
            let (name, remainder) = remainder.split_at(pos);
            let val = values
                .get(&name.to_owned())
                .expect(format!("could not find value: {}", name).as_str());
            let (_, remainder) = remainder.split_at(br"}".len());
            return url_replace_inner(result, val, remainder, values);
        } else {
            return Err("Unterminated variable tag");
        }
    }

    let pos = find_interest(remainder);
    if pos.is_none() {
        result
            .write_str(remainder)
            .expect("could not write remainder");
        return Ok(result);
    }

    let (to_add, remainder) = remainder.split_at(pos.unwrap());
    url_replace_inner(result, to_add, remainder, values)
}

#[cfg(test)]
mod url_replace_tests {
    use super::*;

    #[test]
    fn test_plain() {
        let test_url = "test/something/blah";
        let result = url_replace(test_url, &HashMap::new());
        assert_eq!(result, Ok("test/something/blah".to_owned()));
    }

    #[test]
    fn test_parse() {
        let test_url = "test/{val}/blah";
        let mut test_values = HashMap::new();
        test_values.insert("val".to_owned(), "something".to_owned());
        let result = url_replace(test_url, &test_values);
        assert_eq!(result, Ok("test/something/blah".to_owned()));
    }

    #[test]
    fn test_multi_parse() {
        let test_url = "{v1}/{v2}{v3}";
        let mut test_values = HashMap::new();
        test_values.insert("v1".to_owned(), "la".to_owned());
        test_values.insert("v2".to_owned(), "de".to_owned());
        test_values.insert("v3".to_owned(), "dah".to_owned());
        let result = url_replace(test_url, &test_values);
        assert_eq!(result, Ok("la/dedah".to_owned()));
    }
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
