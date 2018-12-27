use std::net::UdpSocket;
use std::str::from_utf8;

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

pub fn send_discovery_request() -> std::io::Result<()> {
    let socket = UdpSocket::bind("192.168.178.42:31556")?;

    socket.send_to(search_servers_request().as_bytes(), discovery_address());

    Ok(())
}

pub fn receive_notify_message() -> std::io::Result<String> {
    let socket = UdpSocket::bind("192.168.178.42:31556")?;

    let mut buf = [0; 5000];
    let (size, source) = socket.recv_from(&mut buf)?;

    let reply_str = from_utf8(&buf).unwrap();

    Ok(reply_str.to_string())
}
