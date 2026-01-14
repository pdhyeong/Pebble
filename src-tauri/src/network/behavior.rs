use libp2p::{mdns, swarm::NetworkBehaviour};

#[derive(NetworkBehaviour)]
pub struct MyBehaviour {
    pub mdns: mdns::tokio::Behaviour,
}