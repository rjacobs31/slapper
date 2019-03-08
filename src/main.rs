#[macro_use]
extern crate clap;

mod hit;

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
        )
        (@subcommand write =>
            (about: "Writes a config file")
        )
    ).get_matches();

    let _config_file = matches.value_of("CONFIG").unwrap_or("slapper.toml");

    match matches.subcommand() {
        ("hit", Some(matches)) => {
            let project_name = matches.value_of("PROJECT").unwrap();
            let environment_name = matches.value_of("ENVIRONMENT").unwrap();
            let endpoint_name = matches.value_of("ENDPOINT").unwrap();
            let conf = config::get_projects();

            let result = hit::do_request(
                &conf.projects,
                project_name,
                environment_name,
                endpoint_name,
            );
            println!("{}", result);
        }
        ("write", _) => {
            let projects = config::get_projects();
            let serialized = toml::to_string_pretty(&projects).unwrap();
            println!("{}", serialized);
        }
        _ => {}
    }
}

mod config {
    use crate::hit::{Endpoint, Environment, Project};
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;
    use std::iter::FromIterator;
    use url::Url;

    #[derive(Serialize, Deserialize)]
    pub struct Config {
        pub some_value: String,
        pub projects: HashMap<String, Project>,
    }

    pub fn get_projects() -> Config {
        Config {
            some_value: String::from("String"),
            projects: HashMap::from_iter(vec![
                (
                    String::from("project1"),
                    Project::from_full(
                        None,
                        vec![(
                            String::from("dev"),
                            Environment::new(Url::parse("http://localhost:8000").unwrap()),
                        )],
                        vec![(String::from("some_object"), Endpoint::new("/"))],
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
}
