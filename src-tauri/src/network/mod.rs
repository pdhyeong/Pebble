pub mod behavior;

use behavior::{MyBehaviour, FileListRequest, FileListResponse, FileInfo};
use futures::StreamExt;
use libp2p::{swarm::SwarmEvent, request_response, Multiaddr, PeerId, StreamProtocol, SwarmBuilder};
use std::error::Error;
use std::time::Duration;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;

// 공유 폴더 경로 (나중에 설정으로 변경 가능)
fn get_shared_folder() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Pebble")
        .join("Shared")
}

// 폴더 내 파일 목록 조회
fn list_files(base_path: &PathBuf, relative_path: &str) -> FileListResponse {
    let full_path = if relative_path.is_empty() || relative_path == "/" {
        base_path.clone()
    } else {
        base_path.join(relative_path.trim_start_matches('/'))
    };

    // 공유 폴더가 없으면 생성
    if !base_path.exists() {
        if let Err(e) = std::fs::create_dir_all(base_path) {
            return FileListResponse {
                path: relative_path.to_string(),
                files: vec![],
                error: Some(format!("공유 폴더 생성 실패: {}", e)),
            };
        }
    }

    match std::fs::read_dir(&full_path) {
        Ok(entries) => {
            let files: Vec<FileInfo> = entries
                .filter_map(|entry| entry.ok())
                .map(|entry| {
                    let metadata = entry.metadata().ok();
                    let is_dir = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);
                    let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);

                    FileInfo {
                        name: entry.file_name().to_string_lossy().to_string(),
                        path: entry.path().strip_prefix(base_path)
                            .map(|p| format!("/{}", p.display()))
                            .unwrap_or_else(|_| entry.path().display().to_string()),
                        is_dir,
                        size,
                    }
                })
                .collect();

            FileListResponse {
                path: relative_path.to_string(),
                files,
                error: None,
            }
        }
        Err(e) => FileListResponse {
            path: relative_path.to_string(),
            files: vec![],
            error: Some(format!("폴더 읽기 실패: {}", e)),
        },
    }
}

// 파일 목록 요청 메시지
pub struct FileListRequestMsg {
    pub peer_id: PeerId,
    pub path: String,
}

pub async fn run_p2p_engine(
    app: AppHandle,
    mut dial_rx: mpsc::UnboundedReceiver<String>,
    mut file_list_rx: mpsc::UnboundedReceiver<FileListRequestMsg>,
) -> Result<(), Box<dyn Error>> {
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

            // 파일 목록 프로토콜
            let file_list = request_response::Behaviour::new(
                [(StreamProtocol::new("/pebble/file-list/1.0.0"), request_response::ProtocolSupport::Full)],
                request_response::Config::default(),
            );

            Ok(MyBehaviour { mdns, identify, ping, file_list })
        })?
        .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(Duration::from_secs(60 * 60)))
        .build();

    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    let local_peer_id = swarm.local_peer_id().to_string();
    println!("내 기기 PeerId: {}", local_peer_id);
    println!("공유 폴더: {:?}", get_shared_folder());

    loop {
        tokio::select! {
            // Dial 요청 처리
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

            // 파일 목록 요청 처리
            Some(req) = file_list_rx.recv() => {
                println!("📂 파일 목록 요청: peer={}, path={}", req.peer_id, req.path);
                let request = FileListRequest { path: req.path.clone() };
                swarm.behaviour_mut().file_list.send_request(&req.peer_id, request);
                app.emit("file-list-request-sent", format!("{}:{}", req.peer_id, req.path)).ok();
            }

            // Swarm 이벤트 처리
            event = swarm.select_next_some() => {
                match event {
                    // mDNS 피어 발견
                    SwarmEvent::Behaviour(behavior::MyBehaviourEvent::Mdns(libp2p::mdns::Event::Discovered(list))) => {
                        for (peer_id, addr) in list {
                            println!("peer-found ID: {}, 주소: {}", peer_id, addr);
                            let payload = format!("{}:{}", peer_id, addr);
                            app.emit("peer-found", payload).ok();
                        }
                    }

                    // mDNS 피어 만료
                    SwarmEvent::Behaviour(behavior::MyBehaviourEvent::Mdns(libp2p::mdns::Event::Expired(list))) => {
                        for (peer_id, addr) in list {
                            println!("peer-expired: {} at {}", peer_id, addr);
                            app.emit("peer-expired", peer_id.to_string()).ok();
                        }
                    }

                    // Identify 정보 수신
                    SwarmEvent::Behaviour(behavior::MyBehaviourEvent::Identify(libp2p::identify::Event::Received { info, .. })) => {
                        println!("ℹ️ 피어 정보: {}", info.agent_version);
                        app.emit("peer-info", info.agent_version).ok();
                    }

                    // Ping 결과
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

                    // 파일 목록 요청/응답 처리
                    SwarmEvent::Behaviour(behavior::MyBehaviourEvent::FileList(event)) => {
                        match event {
                            request_response::Event::Message { peer, message } => {
                                match message {
                                    // 요청 수신 - 파일 목록 응답
                                    request_response::Message::Request { request, channel, .. } => {
                                        println!("📥 파일 목록 요청 수신: {} -> {}", peer, request.path);
                                        let shared_folder = get_shared_folder();
                                        let response = list_files(&shared_folder, &request.path);

                                        if let Err(e) = swarm.behaviour_mut().file_list.send_response(channel, response) {
                                            println!("응답 전송 실패: {:?}", e);
                                        }
                                    }

                                    // 응답 수신 - 프론트엔드로 전달
                                    request_response::Message::Response { response, .. } => {
                                        println!("📤 파일 목록 응답 수신: {} 파일", response.files.len());
                                        let payload = serde_json::json!({
                                            "peer_id": peer.to_string(),
                                            "path": response.path,
                                            "files": response.files,
                                            "error": response.error,
                                        });
                                        app.emit("remote-files", payload.to_string()).ok();
                                    }
                                }
                            }
                            request_response::Event::OutboundFailure { peer, error, .. } => {
                                println!("❌ 요청 실패: {} - {:?}", peer, error);
                                app.emit("file-list-error", format!("{}:{:?}", peer, error)).ok();
                            }
                            request_response::Event::InboundFailure { peer, error, .. } => {
                                println!("❌ 응답 실패: {} - {:?}", peer, error);
                            }
                            request_response::Event::ResponseSent { peer, .. } => {
                                println!("✅ 응답 전송 완료: {}", peer);
                            }
                        }
                    }

                    // 연결 수립
                    SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
                        println!("connection-success PeerId: {}, Endpoint: {:?}", peer_id, endpoint);
                        app.emit("connection-success", peer_id.to_string()).ok();
                    }

                    // 연결 종료
                    SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
                        println!("Connection Closed: {}, 원인: {:?}", peer_id, cause);
                        app.emit("connection-closed", peer_id.to_string()).ok();
                    }

                    // 리스닝 주소
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
