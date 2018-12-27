mod satip;

const BIND_ADDRESS: &str = "192.168.178.42:31222";

fn main() {
    println!("Hello, world!");

    println!("Reply:\n{}",
             satip::discovery::discover_servers(BIND_ADDRESS).unwrap());
}
