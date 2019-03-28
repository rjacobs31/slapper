#[macro_use]
extern crate clap;

mod hit;
mod parse;
mod project;

use config::Config;
use std::fs::File;
use std::io::prelude::*;
use std::io::{BufReader, Write};

fn main() {
    let matches = clap_app!(slapper =>
        (version: "0.1.0")
        (author: "R. Jacobs")
        (about: "Hits endpoints")
        (@arg CONFIG: -c --config +takes_value "Sets a custom config file")
        (@subcommand hit =>
            (about: "Hits an endpoint in a specific project and environment")
            (@arg PROJECT: +required "The project to load")
            (@arg ENVIRONMENT: +required "The environment to load")
            (@arg ENDPOINT: "The named endpoint to hit")
            (@arg CUSTOM: --custom +takes_value conflicts_with[ENDPOINT] "A custom path to hit")
            (@arg METHOD: -m --method +takes_value "The HTTP method to use")
            (@arg MEDIA: --media +takes_value "The media type of the request")
            (@arg DATA: -d --data +takes_value "The body of the request")
            (@arg DATA_FILE: --("data-file") +takes_value conflicts_with[DATA] "File to read data from")
            (@arg URL_VALUES: ... "Variable values to pass to parsed URL")
        )
        (@subcommand write =>
            (about: "Writes a config file")
        )
    ).get_matches();

    let config_file = matches.value_of("CONFIG").unwrap_or("slapper.json");

    match matches.subcommand() {
        ("hit", Some(matches)) => {
            let project_name = matches.value_of("PROJECT").unwrap();
            let environment_name = matches.value_of("ENVIRONMENT").unwrap();
            let endpoint_name = matches.value_of("ENDPOINT").unwrap();
            let url_values = matches.values_of("URL_VALUES").unwrap_or_default();

            let projects = if let Ok(f) = File::open(config_file).map(BufReader::new) {
                serde_json::from_reader::<_, Config>(f)
                    .map(|c| c.projects)
                    .unwrap_or_else(|_| {
                        println!("could not read config file");
                        config::get_projects()
                    })
            } else {
                println!("could not open config file");
                config::get_projects()
            };

            let project = &projects[project_name];
            let result = hit::do_request(
                &project,
                &project.environments[environment_name],
                &project.endpoints[endpoint_name],
                url_values.map(String::from).collect::<Vec<_>>().into_iter(),
            );
            println!("{}", result);
        }
        ("write", _) => {
            let projects = config::get_projects();
            let serialized = serde_json::to_string_pretty(&projects).unwrap();
            File::create(config_file)
                .expect("could not open file for writing")
                .write_all(serialized.as_bytes())
                .expect("could not write to file");
        }
        _ => {}
    }
}

mod config {
    use crate::project::{Endpoint, Environment, Project, ProjectMap};
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;
    use std::iter::FromIterator;
    use url::Url;

    #[derive(Serialize, Deserialize)]
    pub struct Config {
        pub projects: HashMap<String, Project>,
    }

    pub fn get_projects() -> ProjectMap {
        HashMap::from_iter(vec![
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
        ])
    }
}
