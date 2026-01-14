pub mod behavior;

use behavior::MyBehaviour;
use futures::StreamExt;
use libp2p::{swarm::SwarmEvent, SwarmBuilder};
use std::error::Error;
use tauri::{AppHandle, Emitter}; 

pub async fn run_p2p_engine(app: AppHandle) -> Result<(), Box<dyn Error>> {
    // 1. Swarm 생성 (Identity -> Runtime -> Transport -> Behaviour -> Build)
    let mut swarm = SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(
            libp2p::tcp::Config::default(),
            libp2p::noise::Config::new,
            libp2p::yamux::Config::default,
        )?
        .with_behaviour(|key| {
            let mdns = libp2p::mdns::tokio::Behaviour::new(
                libp2p::mdns::Config::default(),
                key.public().to_peer_id(),
            ).expect("mDNS 초기화 실패");
            
            MyBehaviour { mdns }
        })?
        .build();

    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    println!("내 기기 PeerId: {}", swarm.local_peer_id());

    loop {

        if let SwarmEvent::Behaviour(behavior::MyBehaviourEvent::Mdns(libp2p::mdns::Event::Discovered(list))) = swarm.select_next_some().await {
            for (peer_id, addr) in list {
                println!("✅ 기기 발견! PeerId: {}, Address: {}", peer_id, addr); // 터미널 출력 추가
                app.emit("peer-found", format!("{}:{}", peer_id, addr)).ok();
            }
        }

        match swarm.select_next_some().await {
            SwarmEvent::Behaviour(behavior::MyBehaviourEvent::Mdns(libp2p::mdns::Event::Discovered(list))) => {
                for (peer_id, addr) in list {
                    println!("상대방 발견! ID: {}, 주소: {}", peer_id, addr);
                    
                    let payload = format!("{}:{}", peer_id, addr);
                    app.emit("peer-found", payload).ok();
                }
            }
            SwarmEvent::NewListenAddr { address, .. } => {
                println!("현재 리스닝 중인 주소: {}", address);
            }
            _ => {}
        }
    }
}