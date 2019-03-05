use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct Endpoint<'a> {
    name: &'a str,
    url: &'a str,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", content = "values")]
pub enum Auth<'a> {
    None,
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
pub struct Environment<'a> {
    name: &'a str,
    auth: Auth<'a>,
}

#[derive(Serialize, Deserialize)]
pub struct Project<'a> {
    name: &'a str,
    base_url: &'a str,
    environments: Vec<Environment<'a>>,
    endpoints: Vec<Endpoint<'a>>,
}