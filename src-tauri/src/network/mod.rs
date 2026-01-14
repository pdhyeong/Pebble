pub mod behavior;

use behavior::MyBehaviour;
use futures::StreamExt;
use libp2p::{swarm::SwarmEvent, Multiaddr, SwarmBuilder};
use std::error::Error;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;

pub async fn run_p2p_engine(app: AppHandle, mut dial_rx: mpsc::UnboundedReceiver<String>) -> Result<(), Box<dyn Error>> {
    let mut swarm = SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(
            libp2p::tcp::Config::default(),
            libp2p::noise::Config::new,
            libp2p::yamux::Config::default,
        )?
        .with_behaviour(|key| {
            let mdns = libp2p::mdns::tokio::Behaviour::new(Default::default(), key.public().to_peer_id())?;
            let identify = libp2p::identify::Behaviour::new(
                libp2p::identify::Config::new("/pebble/1.0.0".into(), key.public())
            );
            let ping = libp2p::ping::Behaviour::new(libp2p::ping::Config::new());
            Ok(MyBehaviour { mdns, identify, ping })
        })?
        .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(Duration::from_secs(60 * 60))) // 1시간 유지
        .build();

    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    println!("내 기기 PeerId: {}", swarm.local_peer_id());
    println!("내 ip 주소 확인 중...");

    loop {
        tokio::select! {
            Some(addr_str) = dial_rx.recv() => {
                println!("📞 Dial 요청 수신: {}", addr_str);
                match addr_str.parse::<Multiaddr>() {
                    Ok(addr) => {
                        match swarm.dial(addr.clone()) {
                            Ok(_) => {
                                println!("Dial connecting~: {}", addr);
                                app.emit("dial-started", addr.to_string()).ok();
                            }
                            Err(e) => {
                                println!("Dial failed: {}", e);
                                app.emit("dial-failed", format!("{}", e)).ok();
                            }
                        }
                    }
                    Err(e) => {
                        println!("dial failed: {}", e);
                        app.emit("dial-failed", format!("주소 파싱 실패: {}", e)).ok();
                    }
                }
            }
            event = swarm.select_next_some() => {
                match event {
                    SwarmEvent::Behaviour(behavior::MyBehaviourEvent::Mdns(libp2p::mdns::Event::Discovered(list))) => {
                        for (peer_id, addr) in list {
                            println!("peer-found ID: {}, 주소: {}", peer_id, addr);
                            let payload = format!("{}:{}", peer_id, addr);
                            app.emit("peer-found", payload).ok();
                        }
                    }
                    SwarmEvent::Behaviour(behavior::MyBehaviourEvent::Mdns(libp2p::mdns::Event::Expired(list))) => {
                        for (peer_id, addr) in list {
                            println!("peer-expired: {} at {}", peer_id, addr);
                            app.emit("peer-expired", peer_id.to_string()).ok();
                        }
                    }
                    SwarmEvent::Behaviour(behavior::MyBehaviourEvent::Identify(libp2p::identify::Event::Received { info, .. })) => {
                        println!("ℹ️ 피어 정보: {}", info.agent_version);
                        app.emit("peer-info", info.agent_version).ok();
                    }
                    SwarmEvent::Behaviour(behavior::MyBehaviourEvent::Ping(libp2p::ping::Event { peer, result, .. })) => {
                        match result {
                            Ok(rtt) => {
                                println!("Ping Success: {} (RTT: {:?})", peer, rtt);
                                let payload = format!("{}:{}", peer, rtt.as_millis());
                                app.emit("ping-success", payload).ok();
                            }
                            Err(e) => {
                                println!("Ping Failed: {} - {}", peer, e);
                            }
                        }
                    }
                    SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
                        println!("connection-success PeerId: {}, Endpoint: {:?}", peer_id, endpoint);
                        app.emit("connection-success", peer_id.to_string()).ok();
                    }
                    SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
                        println!("Connection Closed: {}, 원인: {:?}", peer_id, cause);
                        app.emit("connection-closed", peer_id.to_string()).ok();
                    }
                    SwarmEvent::NewListenAddr { address, .. } => {
                        println!("listening address: {}", address);
                        app.emit("listening-on", address.to_string()).ok();
                    }
                    _ => {}
                }
            }
        }
    }
}