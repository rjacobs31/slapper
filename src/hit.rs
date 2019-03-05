use reqwest;
use serde::{Serialize, Deserialize};
use url::Url;

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", content = "values")]
pub enum Auth<'a> {
    Inherit,
    ClientCredentials{
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
    name: &'a str,
    url: &'a str,
    auth: Option<Auth<'a>>,
}

#[derive(Serialize, Deserialize)]
pub struct Environment<'a> {
    name: &'a str,
    auth: Option<Auth<'a>>,
}

#[derive(Serialize, Deserialize)]
pub struct Project<'a> {
    name: &'a str,
    #[serde(with = "url_serde")]
    base_url: Url,
    auth: Option<Auth<'a>>,
    environments: Vec<Environment<'a>>,
    endpoints: Vec<Endpoint<'a>>,
}

mod url_serde {
    use std::fmt;
    use serde::{Serializer, Deserializer, de::{self, Visitor}};
    use url::Url;

    struct UrlVisitor;

    impl<'de> Visitor<'de> for UrlVisitor {
        type Value = Url;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string value to parse as URL")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where E: de::Error
        {
            match Url::parse(value) {
                Ok(parsed) => Ok(parsed),
                Err(_) => Err(E::custom(format!("could not parse URL: {}", value))),
            }
        }
    }

    pub fn serialize<S>(url: &Url, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer
    {
        serializer.serialize_str(url.as_str())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Url, D::Error> where D: Deserializer<'de> {
        deserializer.deserialize_str(UrlVisitor)
    }
}