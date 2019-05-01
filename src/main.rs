mod config;
mod hit;
mod parse;
mod project;

use crate::config::Config;
use crate::hit::process_hit_subcommand;
use clap::{App, Arg, SubCommand};
use hit::get_hit_subcommand;
use std::fs::File;
use std::io::Write;

fn main() {
    let matches = App::new("slapper")
        .version("0.1.0")
        .author("R. Jacobs")
        .about("Hits endpoints")
        .arg(
            Arg::with_name("CONFIG")
                .short("c")
                .long("config")
                .global(true)
                .takes_value(true)
                .help("Sets a custom config file"),
        )
        .subcommand(get_hit_subcommand())
        .subcommand(
            SubCommand::with_name("list").about("Lists projects, environments, and endpoints"),
        )
        .subcommand(SubCommand::with_name("write").about("Writes a config file"))
        .get_matches();

    let config_file = matches.value_of("CONFIG").unwrap_or("slapper.json");
    let config = Config::from_file(config_file).expect("could not load config");

    match matches.subcommand() {
        ("hit", Some(matches)) => {
            process_hit_subcommand(matches, config);
        }
        ("write", _) => {
            let projects = config::get_example_config().projects;
            let serialized = serde_json::to_string_pretty(&projects).unwrap();
            File::create(config_file)
                .expect("could not open file for writing")
                .write_all(serialized.as_bytes())
                .expect("could not write to file");
        }
        _ => {}
    }
}
