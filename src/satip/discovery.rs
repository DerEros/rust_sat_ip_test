use std::net::SocketAddr;

use crate::satip::config::Config;
use tokio::net::UdpSocket;
use tokio::prelude::Future;
use std::error::Error;
use tokio::prelude::future;
use std::str::FromStr;

pub const SAT_IP_DISCOVERY_ADDRESS: &str = "239.255.255.250";
pub const SAT_IP_DISCOVERY_PORT: i32 = 1900;

fn discovery_address() -> String {
    format!("{}:{}", SAT_IP_DISCOVERY_ADDRESS, SAT_IP_DISCOVERY_PORT)
}

fn search_servers_request() -> String {
    format!("M-SEARCH * HTTP/1.1
HOST: {}
MAN: \"ssdp:discover\"
MX: 2
ST: urn:ses-com:device:SatIPServer:1
USER-AGENT: Linux/1.0 UPnP/1.1 ernasatip/1.0
\r\n
", discovery_address())
}

#[derive(Debug)]
struct DiscoveryContext {
    pub config: Config,
    pub socket_addr: SocketAddr,
    pub socket: UdpSocket
}

impl DiscoveryContext {
    fn new(config: &Config) -> impl Future<Item = DiscoveryContext, Error = ()> {
        future::ok::<DiscoveryContext, ()>(DiscoveryContext {
            config: *config,
            socket_addr: SocketAddr::from_str("0.0.0.0:0").unwrap(),
            socket: UdpSocket::bind(&SocketAddr::from_str("0.0.0.0:0").unwrap()).unwrap()
        })
    }
}

pub fn discover_satip_servers(config: &Config) -> impl Future<Item = (), Error = ()> {
    info!("Going to discover available SAT>IP servers");

    let discovery_context = DiscoveryContext::new(config);

    discovery_context.map(|context| debug!("Using discovery context:\n{:?}", context))
}