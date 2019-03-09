use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;

pub type ProjectMap = HashMap<String, Project>;

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
