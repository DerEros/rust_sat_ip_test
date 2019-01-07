use std::net::SocketAddr;

use crate::satip::config::Config;
use crate::satip::errors::*;
use crate::satip::helpers::*;
use tokio::net::UdpSocket;
use tokio::prelude::Future;
use std::str::FromStr;
use tokio::prelude::future::IntoFuture;
use hyper::Request;
use hyper::client::Client;

fn search_servers_request(target_address: SocketAddr, user_agent: &str) -> String {
    debug!("Generating discovery request for target '{}' using user agent '{}'",
           target_address.to_string(),
           user_agent);

    let request = format!("M-SEARCH * HTTP/1.1
HOST: {}
MAN: \"ssdp:discover\"
MX: 2
ST: urn:ses-com:device:SatIPServer:1
USER-AGENT: {}
\r\n
", target_address, user_agent);

    let mut req = Request::builder();
    let r = req.method("M-SEARCH").uri("*")
        .header("HOST", target_address.to_string())
        .header("MAN", "ssdp:discover")
        .header("MX", "2")
        .header("ST", "urn:ses-com:device:SatIPServer:1")
        .header("USER-AGENT", user_agent).body(()).unwrap();

    info!("Generated request: {}", String::from(RenderableRequest(r)));

    trace!("Generated request:\n{}", request);
    request
}

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

        Ok(DiscoveryContext { config, broadcast_address, socket })
    }
}

pub fn discover_satip_servers(config: Config) -> impl Future<Item = (), Error = Error> {
    info!("Going to discover available SAT>IP servers");

    let discovery_context = DiscoveryContext::new(config);

    discovery_context
        .map(|context| { debug!("Using discovery context:\n{:?}", context); context })
        .map(|context| (search_servers_request(context.broadcast_address, config.user_agent)
                            .as_bytes().to_vec(), context))
        .into_future()
        .and_then(|(request, context)|
            send_discovery_request(context.socket, context.broadcast_address, request)
        )
        .map_err(|err| { error!("Could not send discovery request. Cause: {}", err); err } )
        .and_then(wait_for_discovery_responses)
        .map(|(socket, buffer, size, sender)| {
            debug!("Received {} bytes discovery message from {:?}", size, sender);
            trace!("Discovered:\n{}",
                   String::from_utf8(buffer.clone()).unwrap_or("<Unable to parse result>".to_string()));
            (socket, buffer, size, sender)
        })
        .map(|_| ())
}

fn parse_address(address_string: &str) -> Result<SocketAddr, Error> {
    trace!("Parsing address '{}'", address_string);
    SocketAddr::from_str(address_string)
        .map_err(|_| Error {
            error_type: ErrorType::InvalidIpFormat,
            message: format!("Could not parse address")
        })
}

fn bind_udp_socket(socket_address: SocketAddr) -> Result<UdpSocket, Error> {
    trace!("Binding to socket '{:?}'", socket_address);
    UdpSocket::bind(&socket_address)
        .map_err(|err| Error {
                error_type: ErrorType::CouldNotBindUdpSocket,
                message: format!("Unable to bind to UDP socket. Cause: {}", err)
        })
}

fn send_discovery_request(socket: UdpSocket,
                          recipient: SocketAddr,
                          request: Vec<u8>) -> impl Future<Item = UdpSocket, Error = Error> {
    debug!("Sending discovery request to '{:?}'", recipient);

    socket.send_dgram(request, &recipient)
        .map(|(socket, _)| socket)
        .map_err(|err| Error {
            error_type: ErrorType::SendUdpRequestError,
            message: format!("Error sending discovery request. Cause {}", err)
        })
}

fn wait_for_discovery_responses(socket: UdpSocket) ->
        impl Future<Item = (UdpSocket, Vec<u8>, usize, SocketAddr), Error = Error> {
    let buffer = [0u8; 65_536].to_vec();
    debug!("Waiting for discovery message to arrive on {:?}", socket);
    socket.recv_dgram(buffer)
        .map_err(|err| Error {
            error_type: ErrorType::ReceivingDiscoveryMessageError,
            message: format!("Error receiving discovery response. Cause {}", err)
        })
}