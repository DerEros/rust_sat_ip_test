use std::net::SocketAddr;

use crate::satip::config::Config;
use crate::satip::errors::*;
use tokio::net::UdpSocket;
use tokio::prelude::Future;
use tokio::prelude::future;
use std::str::FromStr;

//fn search_servers_request() -> String {
//    format!("M-SEARCH * HTTP/1.1
//HOST: {}
//MAN: \"ssdp:discover\"
//MX: 2
//ST: urn:ses-com:device:SatIPServer:1
//USER-AGENT: Linux/1.0 UPnP/1.1 ernasatip/1.0
//\r\n
//", discovery_address())
//}

#[derive(Debug)]
struct DiscoveryContext {
    pub config: Config,
    pub socket_addr: SocketAddr,
    pub socket: UdpSocket
}

impl DiscoveryContext {
    fn new(config: &Config) -> impl Future<Item = DiscoveryContext, Error = Error> {
        future::ok::<DiscoveryContext, Error>(DiscoveryContext {
            config: *config,
            socket_addr: SocketAddr::from_str("0.0.0.0:0").unwrap(),
            socket: UdpSocket::bind(&SocketAddr::from_str("0.0.0.0:0").unwrap()).unwrap()
        })
    }
}

pub fn discover_satip_servers(config: &Config) -> impl Future<Item = (), Error = Error> {
    info!("Going to discover available SAT>IP servers");

    let discovery_context = DiscoveryContext::new(config);

    discovery_context.map(|context| debug!("Using discovery context:\n{:?}", context))
        .and_then(|_| future::err(Error{error_type: ErrorType::InvalidIpFormat, message: "Foo bar" }))
}