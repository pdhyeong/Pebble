use futures::StreamExt;
use libp2p::{mdns, identify, ping, swarm::SwarmEvent, SwarmBuilder};
use std::error::Error;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut swarm = SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(Default::default(), libp2p::noise::Config::new, libp2p::yamux::Config::default)?
        .with_behaviour(|key| {
            let mdns = mdns::tokio::Behaviour::new(Default::default(), key.public().to_peer_id())?;
            let identify = identify::Behaviour::new(identify::Config::new(
                "/pebble/1.0.0".into(),
                key.public(),
            ));
            let ping = ping::Behaviour::new(ping::Config::new());
            Ok(MyBehaviour { mdns, identify, ping })
        })?
        .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(Duration::from_secs(60 * 60)))
        .build();

    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;
    println!("🚀 [테스트 노드] 실행 중... 상대를 찾는 중입니다. {}", "/ip4/0.0.0.0/tcp/0");

    loop {
        match swarm.select_next_some().await {
            SwarmEvent::Behaviour(MyBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                for (peer_id, addr) in list {
                    println!("🔍 기기 발견! PeerId: {}, 주소: {}", peer_id, addr);
                    // 자동 연결하지 않음 - 상대가 연결 버튼을 눌러야 연결됨
                }
            }
            SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
                println!("✅ 연결 성공! 상대 PeerId: {}", peer_id);
                println!("📍 연결된 주소: {:?}", endpoint.get_remote_address());
            }
            SwarmEvent::Behaviour(MyBehaviourEvent::Identify(identify::Event::Received { info, .. })) => {
                println!("상대 정보 확인: 프로토콜 버전 - {}", info.protocol_version);
                println!("상대 에이전트: {}", info.agent_version);
            }
            _ => {}
        }
    }
}

#[derive(libp2p::swarm::NetworkBehaviour)]
struct MyBehaviour {
    mdns: mdns::tokio::Behaviour,
    identify: identify::Behaviour,
    ping: ping::Behaviour,
}