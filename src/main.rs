#[macro_use]
extern crate clap;

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
    ).get_matches();
}
