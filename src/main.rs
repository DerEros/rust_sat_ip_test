#[macro_use]
extern crate log;
extern crate log4rs;
extern crate tokio;

use log4rs::append::console::ConsoleAppender;
use log4rs::config::Config;
use log4rs::config::Appender;
use log4rs::config::Root;
use log::LevelFilter;

mod satip;

const BIND_ADDRESS: &str = "0.0.0.0:0";

fn default_logging_setup() -> () {
    let stdout = ConsoleAppender::builder().build();
    let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .build(Root::builder().appender("stdout").build(LevelFilter::Debug))
        .unwrap();

    let _ = log4rs::init_config(config).unwrap();
}

fn main() {
    default_logging_setup();

    info!("Hello, world!");

    info!("Reply:\n{}",
             satip::discovery::discover_servers(BIND_ADDRESS).unwrap());
}
