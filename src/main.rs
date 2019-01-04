#[macro_use]
extern crate log;
extern crate log4rs;
extern crate tokio;
extern crate futures;

mod satip;

use log4rs::append::console::ConsoleAppender;
use log4rs::config::Config;
use log4rs::config::Appender;
use log4rs::config::Root;
use log::LevelFilter;
use tokio::prelude::future;
use futures::Future;

use crate::satip::errors::Error;

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

    let config_future = future::ok::<satip::config::Config, Error>(satip::config::default_config());

    let full_future = config_future.map(|config| {
            debug!("Loaded configuration: \n{:?}", config);
            config
        })
        .and_then(|config| satip::discovery::discover_satip_servers(config))
        .map_err(|err| { error!("SAT>IP server discovery failed! {}", err); () });

    tokio::run(full_future);
}
