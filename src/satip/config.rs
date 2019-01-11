use std::time::Duration;

#[derive(Debug, Copy, Clone)]
pub struct Config {
    pub bind_address: &'static str,
    pub discovery_broadcast_address: &'static str,
    pub discovery_wait_time: Duration,
    pub user_agent: &'static str,
}

pub fn default_config() -> Config {
    Config {
        bind_address : "0.0.0.0:0",
//        discovery_broadcast_address : "127.0.0.1:1337",
        discovery_broadcast_address : "239.255.255.250:1900",
        discovery_wait_time: Duration::from_secs(5),
        user_agent: "Linux/1.0 UPnP/1.1 ernasatip/1.0",
    }
}