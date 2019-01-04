use std::net::SocketAddr;

use crate::satip::config::Config;
use crate::satip::errors::*;
use tokio::net::UdpSocket;
use tokio::prelude::Future;
use tokio::prelude::future;
use std::str::FromStr;
use tokio::prelude::future::IntoFuture;

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
    fn new(config: Config) -> impl Future<Item = DiscoveryContext, Error = Error> {
        parse_address_future(config.bind_address)
            .and_then(move |parsed_address| {
                let udp_socket_future = bind_udp_socket(parsed_address);

                udp_socket_future.map(move |socket| DiscoveryContext {
                    config,
                    socket_addr: parsed_address,
                    socket
                })
            })
    }
}

pub fn discover_satip_servers(config: Config) -> impl Future<Item = (), Error = Error> {
    info!("Going to discover available SAT>IP servers");

    let discovery_context = DiscoveryContext::new(config);

    discovery_context.map(|context| debug!("Using discovery context:\n{:?}", context))
}

fn parse_address_future(address_string: &str) -> impl Future<Item = SocketAddr, Error = Error> {
    SocketAddr::from_str(address_string)
        .into_future()
        .map_err(|_| Error {
            error_type: ErrorType::InvalidIpFormat,
            message: format!("Could not parse address")
        })
}

fn bind_udp_socket(socket_address: SocketAddr) -> impl Future<Item = UdpSocket, Error = Error> {
    match UdpSocket::bind(&socket_address) {
        Ok(socket) => future::ok(socket),
        Err(err) => future::err(Error {
            error_type: ErrorType::CouldNotBindUdpSocket,
            message: format!("Unable to bind to UDP socket. Cause: {}", err)
        })
    }
}