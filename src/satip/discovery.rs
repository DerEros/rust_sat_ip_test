use std::net::SocketAddr;

use crate::satip::config::Config;
use crate::satip::errors::*;
use crate::satip::helpers::*;
use tokio::net::UdpSocket;
use tokio::prelude::*;
use std::str::FromStr;
use tokio::prelude::future::IntoFuture;
use hyper::Request;
use std::time::Duration;

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

#[derive(Debug)]
struct RawDiscoveryResponse {
    pub buffer: Vec<u8>,
    pub size: usize,
    pub sender_addr: SocketAddr,
}

pub fn discover_satip_servers(config: Config) -> impl Future<Item = (), Error = Error> {
    info!("Going to discover available SAT>IP servers");

    let discovery_context = DiscoveryContext::new(config);

    discovery_context
        .map(|context| { debug!("Using discovery context:\n{:?}", context); context })
        .map(|context| (search_servers_request(context.broadcast_address, config.user_agent), context))
        .into_future()
        .and_then(|(request, context)|
            send_discovery_request(context.socket, context.broadcast_address, request)
        )
        .map_err(|err| { error!("Could not send discovery request. Cause: {}", err); err } )
        .and_then(move |socket| wait_for_discovery_responses(socket, config.discovery_wait_time))
        .map(log_discovery_response)
        .map(|_| ())
}

fn search_servers_request(target_address: SocketAddr, user_agent: &str) -> Vec<u8> {
    debug!("Generating discovery request for target '{}' using user agent '{}'",
           target_address.to_string(),
           user_agent);

    let request = Request::builder().method("M-SEARCH").uri("*")
        .header("HOST", target_address.to_string())
        .header("MAN", "ssdp:discover")
        .header("MX", "2")
        .header("ST", "urn:ses-com:device:SatIPServer:1")
        .header("USER-AGENT", user_agent).body(()).unwrap();
    let serialized_request: Vec<u8> = RenderableRequest(request).into();

    trace!("Generated request:\n{}",
           String::from_utf8(serialized_request.clone()).unwrap_or("<unable to stringify>".to_string()));

    serialized_request
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

fn wait_for_discovery_responses(socket: UdpSocket, wait_time: Duration) ->
impl Future<Item = Option<RawDiscoveryResponse>, Error = Error> {
    let buffer = [0u8; 65_536].to_vec();
    debug!("Waiting for discovery message to arrive on {:?}", socket);
    socket.recv_dgram(buffer)
        .map(|(_, buffer, size, sender_addr)| RawDiscoveryResponse { buffer, size, sender_addr })
        .map_err(|err| Error {
            error_type: ErrorType::ReceivingDiscoveryMessageError,
            message: format!("Error receiving discovery response. Cause {}", err)
        })
        .timeout(wait_time)
        .then(translate_timeout_error)
}

fn translate_timeout_error<T>(result: Result<T, tokio::timer::timeout::Error<Error>>)
                              -> Result<Option<T>, Error> {
    let _dummy_error = Error {
        error_type: ErrorType::ServerDiscoveryUnknownTimeoutError,
        message: "Unknown error while waiting for discovery replies".to_string()
    };

    result
        .map(Some)
        .or_else(|err| if err.is_elapsed() { Ok(None) } else { Err(err) })
        .map_err(|err|
            if err.is_inner() { err.into_inner().unwrap() }
            else if err.is_timer() { get_timer_error_message(err) }
            else { _dummy_error }
        )
}

fn get_timer_error_message(err: tokio::timer::timeout::Error<Error>) -> Error {
    Error {
        error_type: ErrorType::ServerDiscoveryTimeoutError,
        message: format!("Timer related error while waiting for discovery replies: {}", err).to_string()
    }
}

fn log_discovery_response(discovery_response: Option<RawDiscoveryResponse>) -> Option<RawDiscoveryResponse> {
    match discovery_response {
        Some(response) => {
            debug!("Received {} bytes discovery message from {:?}", response.size, response.sender_addr);
            trace!("Discovered:\n{}", String::from_utf8_lossy(response.buffer.clone().as_slice()));
            Some(response)
        }
        None => {
            info!("SAT>IP server discovery finished but found no servers");
            None
        }
    }
}