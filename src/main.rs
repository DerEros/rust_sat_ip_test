mod satip;

fn main() {
    println!("Hello, world!");

    satip::discovery::send_discovery_request();

    println!("Reply:\n{}", satip::discovery::receive_notify_message().unwrap());
}
