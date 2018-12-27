use std::net::UdpSocket;
use std::str::from_utf8;
use std::io;
use std::net::SocketAddr;

pub const SAT_IP_DISCOVERY_ADDRESS: &str = "239.255.255.250";
pub const SAT_IP_DISCOVERY_PORT: i32 = 1900;

pub fn discovery_address() -> String {
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

pub fn discover_servers(bind_address: &str) -> io::Result<String> {
    let socket = UdpSocket::bind(bind_address)?;

    send_discovery_request(&socket)?;
    let (notify_message, source) = receive_notify_message(&socket)?;

    println!("Got reply from: {}", source);
    Ok(notify_message)
}

pub fn send_discovery_request(socket: &UdpSocket) -> io::Result<usize> {
    socket.send_to(search_servers_request().as_bytes(), discovery_address())
}

pub fn receive_notify_message(socket: &UdpSocket) -> io::Result<(String, SocketAddr)> {
    let mut buf = [0; 5000];
    let (_, source) = socket.recv_from(&mut buf)?;

    let reply_str: &str = from_utf8(&buf).unwrap();

    Ok((reply_str.to_owned(), source))
}
