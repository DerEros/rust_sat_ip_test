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
use http::uri::Uri;
use httparse::{Response, EMPTY_HEADER, Status::Complete, Status::Partial, Header};
use http::uri::Authority;
use hyper::Client;
use minidom::Element;

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

#[derive(Debug, Clone, Eq, PartialEq)]
struct RawDiscoveryResponse {
    pub buffer: Vec<u8>,
    pub size: usize,
    pub sender_addr: SocketAddr,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DiscoveryResponse {
    pub description_location: Uri,
    pub usn: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SatIpServer {}

fn find_header<'a>(name: &str, headers: &'a [Header]) -> Option<&'a [u8]> {
    headers.iter().find(|h| h.name.eq(name)).map(|header| header.value)
}

impl <'headers, 'buf> From<Response<'headers, 'buf>> for DiscoveryResponse {
    fn from(response: Response<'headers, 'buf>) -> Self {
        DiscoveryResponse {
            usn: find_header("USN", response.headers)
                .map(String::from_utf8_lossy)
                .map(|cow_string| String::from(cow_string))
                .unwrap_or("".to_string()),
            description_location: find_header("LOCATION", response.headers)
                .map(String::from_utf8_lossy)
                .map(|cow_string| String::from(cow_string))
                .map(|uri_str| Uri::from_str(uri_str.as_ref()))
                .map(|uri_parse_result| {
                    match uri_parse_result {
                        Ok(uri) => uri,
                        Err(e) => {
                            warn!("Error parsing discovered location URI: {}", e);
                            Uri::from_str("http://invalid.inv").unwrap()
                        }
                    }
                }).unwrap()
        }
    }
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
        .and_then(move |socket| wait_for_discovery_responses(socket, config.discovery_wait_time)
            .map(log_discovery_response)
            .map(move |raw| raw.map(|r| parse_discovery_response(config.prefer_source_addr, r)))
        )
        .map(|satip_server| { info!("Discovered SAT>IP server: {:?}", satip_server); satip_server })
        .and_then(|wrapped_discovery_response| get_device_description(wrapped_discovery_response.unwrap().unwrap()))
        .and_then(parse_device_description)
        .map(|_| ())
        .map_err(|err| { error!("Error discovering servers: {:?}", err); err })
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

fn parse_discovery_response(prefer_source_addr: bool, raw_response: RawDiscoveryResponse) -> Result<DiscoveryResponse, Error> {
    debug!("Parsing raw discovery response from {}", raw_response.sender_addr);

    let mut headers = [EMPTY_HEADER; 16];
    let mut response = httparse::Response::new(&mut headers);

    match response.parse(raw_response.buffer.as_ref()) {
        Ok(Complete(size)) => {
            debug!("Parsed response headers of size {}", size);
            trace_parsed_response(&response);
            Ok(response.into())
        },
        Ok(Partial) => Err(Error {
            error_type: ErrorType::CouldNotParseDiscoveryResponse,
            message: format!("Received incomplete discovery response from {:?}", raw_response.sender_addr)
        }),
        Err(e) => Err(Error {
            error_type: ErrorType::CouldNotParseDiscoveryResponse,
            message: format!("Could not parse discovery response. Got parse error: {}", e)
        })
    }
        .map(|r| if prefer_source_addr { replace_source(raw_response.sender_addr, r) } else {r})
        .map(|r| {debug!("Parsing successful"); r})
        .map_err(|e| {error!("Parsing not successful: {:?}", e); e})
}

fn trace_parsed_response(response: &Response) -> () {
    let mut header_str = String::new();
    for Header{ name, value } in response.headers.iter() {
        header_str += format!("\tName: {}; Value: {}\n", name, String::from_utf8_lossy(value))
            .as_str();
    }

    trace!("\nHTTP Version: {}\nStatus Code: {}\nReason: {}\nHeaders:\n{}",
        response.version.map(|v| v.to_string()).unwrap_or("<none>".to_string()),
        response.code.map(|c| c.to_string()).unwrap_or("<none>".to_string()),
        response.reason.map(String::from).unwrap_or("<none>".to_string()),
        header_str,
    );
}

fn replace_source(new_source: SocketAddr, response: DiscoveryResponse) -> DiscoveryResponse {
    let original_uri = response.description_location;
    let uri = Uri::builder()
        .path_and_query(original_uri.path_and_query().unwrap().clone())
        .authority(Authority::from_str(new_source.ip().to_string().as_ref()).unwrap())
        .scheme(original_uri.scheme_part().unwrap().clone())
        .build()
        .unwrap();
    DiscoveryResponse { description_location: uri, .. response }
}

fn get_device_description(discovery_response: DiscoveryResponse)
    -> impl Future<Item = Vec<u8>, Error = Error> {

    let client = Client::new();
    client.get(discovery_response.description_location)
        .and_then(|response| response
            .into_body()
            .collect()
            .map(|chunks| chunks.iter().fold(Vec::with_capacity(1_024),
                  |mut acc, chunk| { acc.extend_from_slice(chunk.as_ref()); acc})
            )
        )
        .map_err(|err| Error {
            error_type: ErrorType::CouldNotRetrieveServerDescription,
            message: format!("Unable to retrieve server description. Encountered error: {}", err)
        })
        .inspect(|buffer|
            trace!("Received device description:\n{}", String::from_utf8_lossy(buffer))
        )
}

fn parse_device_description(raw_response: Vec<u8>) -> Result<SatIpServer, Error> {

    let element =
        String::from_utf8_lossy(raw_response.as_ref())
            .parse()
            .map(|element: Element| (extract_manufacturer(&element), extract_model_name(&element)))
            .map(|name| { info!("Name: {:?}", name); name });

    Ok(SatIpServer {})
}

fn extract_manufacturer(root: &Element) -> Option<String> {
    root.get_child("device", "")
        .and_then(|device_node| device_node.get_child("manufacturer", ""))
        .map(|manufacturer_node| manufacturer_node.text())
}

fn extract_model_name(root: &Element) -> Option<String> {
    root.get_child("device", "")
        .and_then(|device_node| device_node.get_child("modelName", ""))
        .map(|manufacturer_node| manufacturer_node.text())
}