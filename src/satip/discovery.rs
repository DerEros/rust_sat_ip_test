use std::net::SocketAddr;

use crate::satip::config::Config;
use crate::satip::errors::*;
use tokio::net::UdpSocket;
use tokio::prelude::Future;
use std::str::FromStr;
use tokio::prelude::future::IntoFuture;

//fn search_servers_request(target_address: &SocketAddr, user_agent: &str) -> String {
//    format!("M-SEARCH * HTTP/1.1
//HOST: {}
//MAN: \"ssdp:discover\"
//MX: 2
//ST: urn:ses-com:device:SatIPServer:1
//USER-AGENT: {}
//\r\n
//", target_address, user_agent)
//}

#[derive(Debug)]
struct DiscoveryContext {
    pub config: Config,
    pub broadcast_address: SocketAddr,
    pub socket: UdpSocket
}

impl DiscoveryContext {
    fn new(config: Config) -> Result<DiscoveryContext, Error> {
        let broadcast_address = parse_address(config.discovery_broadcast_address)?;
        let bind_address = parse_address(config.bind_address)?;
        let socket = bind_udp_socket(bind_address)?;

        Ok(DiscoveryContext {
            config,
            broadcast_address,
            socket
        })
    }
}

pub fn discover_satip_servers(config: Config) -> impl Future<Item = (), Error = Error> {
    info!("Going to discover available SAT>IP servers");

    let discovery_context = DiscoveryContext::new(config);

    discovery_context
        .map(|context| debug!("Using discovery context:\n{:?}", context))
        .into_future()
}

fn parse_address(address_string: &str) -> Result<SocketAddr, Error> {
    SocketAddr::from_str(address_string)
        .map_err(|_| Error {
            error_type: ErrorType::InvalidIpFormat,
            message: format!("Could not parse address")
        })
}

fn bind_udp_socket(socket_address: SocketAddr) -> Result<UdpSocket, Error> {
    UdpSocket::bind(&socket_address)
        .map_err(|err| Error {
                error_type: ErrorType::CouldNotBindUdpSocket,
                message: format!("Unable to bind to UDP socket. Cause: {}", err)
        })
}