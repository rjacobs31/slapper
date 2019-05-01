use crate::project::{Endpoint, Environment, Project, ProjectMap};
use failure::Fail;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::iter::FromIterator;
use std::path::Path;
use url::Url;

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub projects: ProjectMap,
}

impl Config {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Config, ConfigError> {
        let file = File::open(path)
            .map(BufReader::new)
            .map_err(ConfigError::Io)?;
        Ok(serde_json::from_reader::<_, Config>(file)
            .map_err(|e| ConfigError::Deserialize(failure::Error::from(e)))?)
    }
}

pub fn get_example_config() -> Config {
    Config {
        projects: HashMap::from_iter(vec![
            (
                String::from("project1"),
                Project::from_full(
                    None,
                    vec![(
                        String::from("dev"),
                        Environment::new(Url::parse("http://localhost:8000").unwrap()),
                    )],
                    vec![(String::from("some_object"), Endpoint::new("/{blah}"))],
                ),
            ),
            (
                String::from("project2"),
                Project::from_full(
                    None,
                    vec![(
                        String::from("dev"),
                        Environment::new(Url::parse("http://localhost:8000").unwrap()),
                    )],
                    vec![(String::from("some_other_object"), Endpoint::new("/"))],
                ),
            ),
        ]),
    }
}

#[derive(Debug, Fail)]
pub enum ConfigError {
    #[fail(display = "{}", _0)]
    Io(#[fail(cause)] std::io::Error),

    #[fail(display = "{}", _0)]
    Deserialize(#[fail(cause)] failure::Error),
}
