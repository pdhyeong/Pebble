use libp2p::{mdns, identify, ping, swarm::NetworkBehaviour};

#[derive(NetworkBehaviour)]
pub struct MyBehaviour {
    pub mdns: mdns::tokio::Behaviour,
    pub identify: identify::Behaviour,
    pub ping: ping::Behaviour,
}