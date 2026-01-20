pub mod behavior;
pub mod helpers;
pub mod types;

pub use types::{
    CancelTransferMsg, DeviceInfoMsg, FileListRequestMsg, FileTransferRequestMsg,
    FileTransferStreamRequestMsg, IncomingTransferState, OutgoingTransferState, PairingApprovalMsg,
    PairingRequestMsg, CHUNK_SIZE,
};

use behavior::{
    DeviceInfoRequest, FileListRequest, FileListResponse, FileTransferRequest,
    FileTransferResponse, FileTransferStreamMsg, FileTransferStreamRequest,
    FileTransferStreamResponse, MyBehaviour, PairingRequest, PairingResponse,
};
use futures::StreamExt;
use helpers::{get_quic_address, list_files};
use libp2p::{
    request_response, swarm::SwarmEvent, Multiaddr, PeerId, StreamProtocol, SwarmBuilder,
};
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;
use uuid::Uuid;

/// 활동 기록 저장 헬퍼 함수
async fn record_activity(
    state: &Arc<crate::state::P2pState>,
    transfer_type: &str,
    file_name: String,
    peer_id: String,
    size: u64,
    status: &str,
    speed: Option<f64>,
) {
    let activity = crate::commands::activity::ActivityRecord {
        id: Uuid::new_v4().to_string(),
        transfer_type: transfer_type.to_string(),
        file_name,
        peer_id,
        device_name: None,
        size,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
        status: status.to_string(),
        speed,
    };

    let mut history = state.activity_history.lock().await;
    history.push(activity);

    // 최대 100개까지만 유지
    if history.len() > 100 {
        history.remove(0);
    }
}

pub async fn run_p2p_engine(
    app: AppHandle,
    mut dial_rx: mpsc::UnboundedReceiver<String>,
    mut file_list_rx: mpsc::UnboundedReceiver<FileListRequestMsg>,
    mut file_transfer_rx: mpsc::UnboundedReceiver<FileTransferRequestMsg>,
    mut file_stream_rx: mpsc::UnboundedReceiver<FileTransferStreamRequestMsg>,
    mut shared_folder_rx: mpsc::UnboundedReceiver<PathBuf>,
    mut pairing_request_rx: mpsc::UnboundedReceiver<PairingRequestMsg>,
    mut pairing_approval_rx: mpsc::UnboundedReceiver<PairingApprovalMsg>,
    mut cancel_transfer_rx: mpsc::UnboundedReceiver<CancelTransferMsg>,
    mut shutdown_rx: mpsc::Receiver<()>,
    initial_shared_folder: PathBuf,
    device_name: String,
    state: Arc<crate::state::P2pState>,
) -> Result<(), Box<dyn Error>> {
    // 공유 폴더 경로를 변경 가능하게
    let mut shared_folder = initial_shared_folder;

    // 전송 상태 관리
    let mut outgoing_transfers: HashMap<String, OutgoingTransferState> = HashMap::new();
    let mut incoming_transfers: HashMap<String, IncomingTransferState> = HashMap::new();
    let mut req_id_to_transfer_id: HashMap<request_response::RequestId, String> = HashMap::new();

    // 피어 주소 추적 (QUIC 우선 사용을 위해)
    let mut peer_addresses: HashMap<PeerId, Vec<Multiaddr>> = HashMap::new();

    // 신뢰된 피어 목록 (인증 완료된 피어)
    let mut trusted_peers: std::collections::HashSet<PeerId> = std::collections::HashSet::new();

    // 대기 중인 페어링 요청 (request_id -> (peer_id, channel, pin))
    let mut pending_pairings: HashMap<
        String,
        (
            PeerId,
            request_response::ResponseChannel<PairingResponse>,
            String,
        ),
    > = HashMap::new();

    let mut swarm = SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(
            libp2p::tcp::Config::default(),
            libp2p::noise::Config::new,
            libp2p::yamux::Config::default,
        )?
        .with_quic() // QUIC 지원 추가
        .with_behaviour(|key| {
            let mdns =
                libp2p::mdns::tokio::Behaviour::new(Default::default(), key.public().to_peer_id())?;
            let identify = libp2p::identify::Behaviour::new(libp2p::identify::Config::new(
                "/pebble/1.0.0".into(),
                key.public(),
            ));
            let ping = libp2p::ping::Behaviour::new(libp2p::ping::Config::new());

            // 파일 목록 프로토콜
            let file_list = request_response::Behaviour::new(
                [(
                    StreamProtocol::new("/pebble/file-list/1.0.0"),
                    request_response::ProtocolSupport::Full,
                )],
                request_response::Config::default(),
            );

            // 파일 전송 프로토콜 (다운로드)
            let file_transfer = request_response::Behaviour::new(
                [(
                    StreamProtocol::new("/pebble/file-transfer/1.0.0"),
                    request_response::ProtocolSupport::Full,
                )],
                request_response::Config::default(),
            );

            // 파일 업로드 프로토콜 (전송)
            let file_stream = request_response::Behaviour::new(
                [(
                    StreamProtocol::new("/pebble/file-upload/1.0.0"),
                    request_response::ProtocolSupport::Full,
                )],
                request_response::Config::default(),
            );

            // 기기 정보 프로토콜
            let device_info = request_response::Behaviour::new(
                [(
                    StreamProtocol::new("/pebble/device-info/1.0.0"),
                    request_response::ProtocolSupport::Full,
                )],
                request_response::Config::default(),
            );

            // 페어링 프로토콜
            let pairing = request_response::Behaviour::new(
                [(
                    StreamProtocol::new("/pebble/pairing/1.0.0"),
                    request_response::ProtocolSupport::Full,
                )],
                request_response::Config::default(),
            );

            Ok(MyBehaviour {
                mdns,
                identify,
                ping,
                file_list,
                file_transfer,
                file_stream,
                device_info,
                pairing,
            })
        })?
        .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(Duration::from_secs(60 * 60)))
        .build();

    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;
    // QUIC 리스너 (UDP)
    swarm.listen_on("/ip4/0.0.0.0/udp/0/quic-v1".parse()?)?;

    let local_peer_id = swarm.local_peer_id().to_string();
    println!("내 기기 PeerId: {}", local_peer_id);
    println!("기기 이름: {}", device_name);
    println!("공유 폴더: {:?}", shared_folder);

    loop {
        tokio::select! {
            // Shutdown 신호 처리
            _ = shutdown_rx.recv() => {
                println!("🛑 P2P 엔진 종료 신호 수신");
                app.emit("p2p-stopped", "P2P 엔진이 종료되었습니다.").ok();
                break;
            }

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

            // 파일 전송 요청 처리 (다운로드)
            Some(req) = file_transfer_rx.recv() => {
                println!("📥 파일 다운로드 요청: peer={}, path={}", req.peer_id, req.path);
                let request = FileTransferRequest { path: req.path.clone() };
                swarm.behaviour_mut().file_transfer.send_request(&req.peer_id, request);
                app.emit("file-download-started", format!("{}:{}", req.peer_id, req.path)).ok();
            }

            // 파일 업로드 요청 처리 (전송 시작)
            Some(req) = file_stream_rx.recv() => {
                let file_name = req.file_path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "unknown".to_string());

                println!("📤 파일 업로드 시작 요청: peer={}, file={}", req.peer_id, file_name);

                // QUIC 주소로 먼저 연결 시도 (대용량 파일에 효율적)
                if let Some(addrs) = peer_addresses.get(&req.peer_id) {
                    if let Some(quic_addr) = get_quic_address(addrs) {
                        println!("🚀 QUIC 연결 시도: {}", quic_addr);
                        if let Err(e) = swarm.dial(quic_addr.clone()) {
                            println!("⚠️ QUIC dial 실패 (기존 연결 사용): {}", e);
                        }
                    }
                }

                match File::open(&req.file_path) {
                    Ok(file) => {
                        let metadata = file.metadata().ok();
                        let total_size = metadata.map(|m| m.len()).unwrap_or(0);
                        let total_chunks = (total_size as f64 / CHUNK_SIZE as f64).ceil() as u32;
                        let transfer_id = Uuid::new_v4().to_string();

                        // Init 메시지 전송
                        let msg = FileTransferStreamMsg::Init {
                            file_name: file_name.clone(),
                            total_size,
                            total_chunks,
                            transfer_id: transfer_id.clone(),
                            remote_path: req.remote_path.clone(),
                        };

                        let request = FileTransferStreamRequest { msg };
                        let req_id = swarm.behaviour_mut().file_stream.send_request(&req.peer_id, request);

                        // 상태 저장
                        let state = OutgoingTransferState {
                            file,
                            file_name: file_name.clone(),
                            total_size,
                            total_chunks,
                            current_chunk: 0,
                            peer_id: req.peer_id,
                            transfer_id: transfer_id.clone(),
                            start_time: std::time::Instant::now(),
                        };

                        outgoing_transfers.insert(transfer_id.clone(), state);
                        req_id_to_transfer_id.insert(req_id, transfer_id.clone());

                        app.emit("file-upload-started", serde_json::json!({
                            "peer_id": req.peer_id.to_string(),
                            "file_name": file_name,
                            "transfer_id": transfer_id,
                            "total_chunks": total_chunks,
                            "total_size": total_size,
                        }).to_string()).ok();
                    }
                    Err(e) => {
                        println!("❌ 파일 열기 실패: {}", e);
                        app.emit("file-upload-error", serde_json::json!({
                            "error": format!("파일 열기 실패: {}", e)
                        }).to_string()).ok();
                    }
                }
            }

            // 공유 폴더 경로 변경 처리
            Some(new_path) = shared_folder_rx.recv() => {
                println!("📁 공유 폴더 경로 변경: {:?}", new_path);
                shared_folder = new_path;
                app.emit("shared-folder-changed", shared_folder.display().to_string()).ok();
            }

            // 페어링 요청 전송 (사용자가 상대방에게 연결 요청)
            Some(req) = pairing_request_rx.recv() => {
                // 6자리 랜덤 PIN 생성
                let pin: String = (0..6).map(|_| rand::random::<u8>() % 10).map(|n| char::from_digit(n as u32, 10).unwrap()).collect();

                println!("🔐 페어링 요청 전송: peer={}, PIN={}", req.peer_id, pin);

                let request = PairingRequest {
                    device_name: device_name.clone(),
                    pin: pin.clone(),
                };
                swarm.behaviour_mut().pairing.send_request(&req.peer_id, request);

                // 프론트엔드에 PIN 표시
                app.emit("pairing-sent", serde_json::json!({
                    "peer_id": req.peer_id.to_string(),
                    "pin": pin,
                }).to_string()).ok();
            }

            // 페어링 승인/거절 처리
            Some(approval) = pairing_approval_rx.recv() => {
                let peer_key = approval.peer_id.to_string();
                if let Some((peer_id, channel, _pin)) = pending_pairings.remove(&peer_key) {
                    // 공유 폴더 표시 이름 가져오기
                    let shared_folder_name = if approval.approved && state.show_folder_name.load(std::sync::atomic::Ordering::SeqCst) {
                        Some(state.shared_folder_display_name.read().await.clone())
                    } else {
                        None
                    };

                    let response = PairingResponse {
                        approved: approval.approved,
                        device_name: device_name.clone(),
                        shared_folder_name,
                        error: if approval.approved { None } else { Some("사용자가 거절했습니다.".to_string()) },
                    };

                    if approval.approved {
                        trusted_peers.insert(peer_id);
                        println!("✅ 페어링 승인: {}", peer_id);
                    } else {
                        println!("❌ 페어링 거절: {}", peer_id);
                    }

                    swarm.behaviour_mut().pairing.send_response(channel, response).ok();
                }
            }

            // 전송 취소 요청 처리
            Some(cancel_req) = cancel_transfer_rx.recv() => {
                let transfer_id = cancel_req.transfer_id.clone();
                println!("🛑 전송 취소 요청: {}", transfer_id);

                // 발신 전송 취소 (Sender가 취소)
                if let Some(state) = outgoing_transfers.remove(&transfer_id) {
                    let req = FileTransferStreamRequest {
                        msg: FileTransferStreamMsg::Cancel {
                            transfer_id: transfer_id.clone(),
                        }
                    };
                    swarm.behaviour_mut().file_stream.send_request(&state.peer_id, req);

                    app.emit("transfer-cancelled", serde_json::json!({
                        "transfer_id": transfer_id,
                        "peer_id": state.peer_id.to_string(),
                        "file_name": state.file_name,
                        "reason": "user_cancelled"
                    }).to_string()).ok();
                }

                // 수신 전송 취소 (Receiver가 취소)
                if let Some(state) = incoming_transfers.remove(&transfer_id) {
                    // 부분 파일 삭제
                    if let Err(e) = std::fs::remove_file(&state.saved_path) {
                        println!("⚠️ 부분 파일 삭제 실패: {}", e);
                    }

                    // Sender에게 Cancel 메시지 전송
                    let req = FileTransferStreamRequest {
                        msg: FileTransferStreamMsg::Cancel {
                            transfer_id: transfer_id.clone(),
                        }
                    };
                    swarm.behaviour_mut().file_stream.send_request(&state.peer_id, req);

                    app.emit("transfer-cancelled", serde_json::json!({
                        "transfer_id": transfer_id,
                        "peer_id": state.peer_id.to_string(),
                        "file_name": state.file_name,
                        "reason": "user_cancelled"
                    }).to_string()).ok();
                }
            }

            // Swarm 이벤트 처리
            event = swarm.select_next_some() => {
                match event {
                    // ... (이전 이벤트들은 유지)
                    SwarmEvent::Behaviour(behavior::MyBehaviourEvent::Mdns(libp2p::mdns::Event::Discovered(list))) => {
                         for (peer_id, addr) in list {
                            let addr_str = addr.to_string();

                            // 피어 주소 저장 (QUIC 우선을 위해 모든 주소 저장)
                            peer_addresses.entry(peer_id).or_insert_with(Vec::new).push(addr.clone());

                            // TCP 주소만 프론트에 표시 (QUIC은 파일 전송 시에만 내부적으로 사용)
                            if !addr_str.contains("quic") {
                                println!("peer-found ID: {}, 주소: {}", peer_id, addr);
                                let payload = format!("{}:{}", peer_id, addr);
                                app.emit("peer-found", payload).ok();
                            } else {
                                println!("📡 QUIC 주소 저장 (파일 전송용): {} -> {}", peer_id, addr);
                            }
                        }
                    }
                    SwarmEvent::Behaviour(behavior::MyBehaviourEvent::Mdns(libp2p::mdns::Event::Expired(list))) => {
                         for (peer_id, _addr) in list {
                            app.emit("peer-expired", peer_id.to_string()).ok();
                        }
                    }
                    SwarmEvent::Behaviour(behavior::MyBehaviourEvent::Identify(libp2p::identify::Event::Received { info, .. })) => {
                        app.emit("peer-info", info.agent_version).ok();
                    }
                    SwarmEvent::Behaviour(behavior::MyBehaviourEvent::Ping(libp2p::ping::Event { peer, result, .. })) => {
                        match result {
                            Ok(rtt) => {
                                let payload = format!("{}:{}", peer, rtt.as_millis());
                                app.emit("ping-success", payload).ok();
                            }
                            Err(_) => {}
                        }
                    }
                    SwarmEvent::Behaviour(behavior::MyBehaviourEvent::FileList(event)) => {
                        match event {
                            request_response::Event::Message { peer, message } => {
                                match message {
                                    request_response::Message::Request { request, channel, .. } => {
                                        // 페어링되지 않은 피어는 차단
                                        if !trusted_peers.contains(&peer) {
                                            println!("⛔ 페어링되지 않은 피어의 파일 목록 요청 거부: {}", peer);
                                            let response = FileListResponse {
                                                path: request.path,
                                                files: vec![],
                                                error: Some("페어링이 필요합니다.".to_string()),
                                            };
                                            swarm.behaviour_mut().file_list.send_response(channel, response).ok();
                                        } else {
                                            let response = list_files(&shared_folder, &request.path);
                                            swarm.behaviour_mut().file_list.send_response(channel, response).ok();
                                        }
                                    }
                                    request_response::Message::Response { response, .. } => {
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
                                app.emit("file-list-error", format!("{}:{:?}", peer, error)).ok();
                            }
                            _ => {}
                        }
                    }
                    // ... (FileTransfer는 기존 유지 - 다운로드 기능)
                    SwarmEvent::Behaviour(behavior::MyBehaviourEvent::FileTransfer(event)) => {
                         match event {
                            request_response::Event::Message { peer, message } => {
                                match message {
                                    request_response::Message::Request { request, channel, .. } => {
                                        // 페어링되지 않은 피어는 차단
                                        if !trusted_peers.contains(&peer) {
                                            println!("⛔ 페어링되지 않은 피어의 파일 전송 요청 거부: {}", peer);
                                            let response = FileTransferResponse {
                                                path: request.path,
                                                data: None,
                                                error: Some("페어링이 필요합니다.".to_string()),
                                            };
                                            swarm.behaviour_mut().file_transfer.send_response(channel, response).ok();
                                        } else {
                                            let file_path = shared_folder.join(request.path.trim_start_matches('/'));
                                            let response = match std::fs::read(&file_path) {
                                                Ok(data) => FileTransferResponse { path: request.path, data: Some(data), error: None },
                                                Err(e) => FileTransferResponse { path: request.path, data: None, error: Some(format!("{}", e)) },
                                            };
                                            swarm.behaviour_mut().file_transfer.send_response(channel, response).ok();
                                        }
                                    }
                                    request_response::Message::Response { response, .. } => {
                                        println!("📦 파일 전송 응답 수신: peer={}, path={}", peer, response.path);
                                        if let Some(error) = response.error {
                                            println!("❌ 파일 다운로드 에러: {}", error);
                                            app.emit("file-download-error", serde_json::json!({"peer_id": peer.to_string(), "error": error}).to_string()).ok();
                                        } else if let Some(data) = response.data {
                                            println!("✅ 파일 데이터 수신: {} bytes", data.len());
                                            let download_folder = dirs::download_dir().unwrap_or_else(|| dirs::home_dir().unwrap_or(PathBuf::from(".")));
                                            let file_name = std::path::Path::new(&response.path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or("file".to_string());
                                            let save_path = download_folder.join(&file_name);
                                            match std::fs::write(&save_path, data) {
                                                Ok(_) => {
                                                    println!("💾 파일 저장 완료: {:?}", save_path);
                                                    app.emit("file-download-complete", serde_json::json!({"peer_id": peer.to_string(), "saved_path": save_path, "size": 0}).to_string()).ok();
                                                }
                                                Err(e) => {
                                                    println!("❌ 파일 저장 실패: {}", e);
                                                    app.emit("file-download-error", serde_json::json!({"peer_id": peer.to_string(), "error": format!("{}", e)}).to_string()).ok();
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }

                    // ====== File Stream (Chunked) 처리 ======
                    SwarmEvent::Behaviour(behavior::MyBehaviourEvent::FileStream(event)) => {
                        match event {
                            request_response::Event::Message { peer, message } => {
                                match message {
                                    // [Receiver] 요청 수신 (Init 또는 Chunk)
                                    request_response::Message::Request { request, channel, .. } => {
                                        // 페어링되지 않은 피어는 차단
                                        if !trusted_peers.contains(&peer) {
                                            println!("⛔ 페어링되지 않은 피어의 파일 업로드 거부: {}", peer);
                                            let transfer_id = match &request.msg {
                                                FileTransferStreamMsg::Init { transfer_id, .. } => transfer_id.clone(),
                                                FileTransferStreamMsg::Chunk { transfer_id, .. } => transfer_id.clone(),
                                                FileTransferStreamMsg::Cancel { transfer_id } => transfer_id.clone(),
                                            };
                                            let res = FileTransferStreamResponse {
                                                transfer_id,
                                                success: false,
                                                error: Some("페어링이 필요합니다.".to_string()),
                                            };
                                            swarm.behaviour_mut().file_stream.send_response(channel, res).ok();
                                            continue;
                                        }

                                        match request.msg {
                                            FileTransferStreamMsg::Init { file_name, total_size, total_chunks, transfer_id, remote_path } => {
                                                println!("📥 파일 업로드 init: {}", file_name);

                                                // 저장 경로 결정
                                                let save_dir = if remote_path.is_empty() || remote_path == "/" {
                                                    shared_folder.clone()
                                                } else {
                                                    shared_folder.join(remote_path.trim_start_matches('/'))
                                                };

                                                match std::fs::create_dir_all(&save_dir) {
                                                    Ok(_) => {
                                                        let save_path = save_dir.join(&file_name);
                                                        match File::create(&save_path) {
                                                            Ok(file) => {
                                                                let state = IncomingTransferState {
                                                                    file,
                                                                    file_name: file_name.clone(),
                                                                    saved_path: save_path.clone(),
                                                                    total_chunks,
                                                                    received_chunks: 0,
                                                                    peer_id: peer,
                                                                    start_time: std::time::Instant::now(),
                                                                    total_size,
                                                                };
                                                                incoming_transfers.insert(transfer_id.clone(), state);

                                                                let res = FileTransferStreamResponse {
                                                                    transfer_id,
                                                                    success: true,
                                                                    error: None,
                                                                };
                                                                swarm.behaviour_mut().file_stream.send_response(channel, res).ok();
                                                            }
                                                            Err(e) => {
                                                                let res = FileTransferStreamResponse {
                                                                    transfer_id,
                                                                    success: false,
                                                                    error: Some(format!("파일 생성 실패: {}", e)),
                                                                };
                                                                swarm.behaviour_mut().file_stream.send_response(channel, res).ok();
                                                            }
                                                        }
                                                    }
                                                    Err(e) => {
                                                        let res = FileTransferStreamResponse {
                                                            transfer_id,
                                                            success: false,
                                                            error: Some(format!("폴더 생성 실패: {}", e)),
                                                        };
                                                        swarm.behaviour_mut().file_stream.send_response(channel, res).ok();
                                                    }
                                                }
                                            }

                                            FileTransferStreamMsg::Chunk { transfer_id, chunk_index: _, data } => {
                                                if let Some(transfer_state) = incoming_transfers.get_mut(&transfer_id) {
                                                    if let Err(e) = transfer_state.file.write_all(&data) {
                                                        println!("❌ 파일 쓰기 실패: {}", e);
                                                        // 에러 응답
                                                        let res = FileTransferStreamResponse { transfer_id: transfer_id.clone(), success: false, error: Some(e.to_string()) };
                                                        swarm.behaviour_mut().file_stream.send_response(channel, res).ok();
                                                        continue;
                                                    }
                                                    transfer_state.received_chunks += 1;

                                                    // 진행률 이벤트 발송
                                                    let progress = (transfer_state.received_chunks as f64 / transfer_state.total_chunks as f64) * 100.0;
                                                    app.emit("file-download-progress", serde_json::json!({
                                                        "transfer_id": transfer_id,
                                                        "peer_id": peer.to_string(),
                                                        "file_name": transfer_state.file_name,
                                                        "progress": progress,
                                                        "current": transfer_state.received_chunks,
                                                        "total": transfer_state.total_chunks
                                                    }).to_string()).ok();

                                                    // 완료 체크
                                                    if transfer_state.received_chunks >= transfer_state.total_chunks {
                                                        println!("✅ 파일 수신 완료: {:?}", transfer_state.saved_path);
                                                        let res = FileTransferStreamResponse { transfer_id: transfer_id.clone(), success: true, error: None };
                                                        swarm.behaviour_mut().file_stream.send_response(channel, res).ok();

                                                        // 완료 이벤트
                                                        app.emit("file-received", serde_json::json!({
                                                            "peer_id": peer.to_string(),
                                                            "file_name": transfer_state.file_name,
                                                            "saved_path": transfer_state.saved_path,
                                                        }).to_string()).ok();

                                                        // 활동 기록
                                                        let duration = transfer_state.start_time.elapsed().as_secs_f64();
                                                        let speed = if duration > 0.0 { Some(transfer_state.total_size as f64 / duration) } else { None };
                                                        let file_name = transfer_state.file_name.clone();
                                                        let total_size = transfer_state.total_size;

                                                        // 상태 제거
                                                        incoming_transfers.remove(&transfer_id);

                                                        // 활동 기록 저장
                                                        record_activity(
                                                            &state,
                                                            "download",
                                                            file_name,
                                                            peer.to_string(),
                                                            total_size,
                                                            "completed",
                                                            speed,
                                                        ).await;
                                                    } else {
                                                        // 계속 진행
                                                        let res = FileTransferStreamResponse { transfer_id: transfer_id.clone(), success: true, error: None };
                                                        swarm.behaviour_mut().file_stream.send_response(channel, res).ok();
                                                    }
                                                } else {
                                                    // 상태 없음 (Init 누락 등)
                                                    let res = FileTransferStreamResponse { transfer_id, success: false, error: Some("Transfer Session Not Found".to_string()) };
                                                    swarm.behaviour_mut().file_stream.send_response(channel, res).ok();
                                                }
                                            }

                                            FileTransferStreamMsg::Cancel { transfer_id } => {
                                                println!("🛑 전송 취소 수신: {}", transfer_id);

                                                // Sender가 취소한 경우: 수신 중인 전송 정리
                                                if let Some(state) = incoming_transfers.remove(&transfer_id) {
                                                    // 부분 파일 삭제
                                                    if let Err(e) = std::fs::remove_file(&state.saved_path) {
                                                        println!("⚠️ 부분 파일 삭제 실패: {}", e);
                                                    }

                                                    app.emit("transfer-cancelled", serde_json::json!({
                                                        "transfer_id": transfer_id,
                                                        "peer_id": peer.to_string(),
                                                        "file_name": state.file_name,
                                                        "reason": "sender_cancelled"
                                                    }).to_string()).ok();
                                                }

                                                // Receiver가 취소한 경우: 송신 중인 전송 정리
                                                if outgoing_transfers.remove(&transfer_id).is_some() {
                                                    app.emit("transfer-cancelled", serde_json::json!({
                                                        "transfer_id": transfer_id,
                                                        "peer_id": peer.to_string(),
                                                        "reason": "receiver_cancelled"
                                                    }).to_string()).ok();
                                                }

                                                let res = FileTransferStreamResponse {
                                                    transfer_id,
                                                    success: true,
                                                    error: None,
                                                };
                                                swarm.behaviour_mut().file_stream.send_response(channel, res).ok();
                                            }
                                        }
                                    }

                                    // [Sender] 응답 수신
                                    request_response::Message::Response { response, request_id } => {
                                        // RequestId로 TransferId 찾기
                                        if let Some(transfer_id) = req_id_to_transfer_id.remove(&request_id) {
                                            if !response.success {
                                                 println!("❌ 전송 중단 (상대방 오류): {:?}", response.error);
                                                 app.emit("file-upload-error", serde_json::json!({"error": response.error, "transfer_id": transfer_id}).to_string()).ok();
                                                 outgoing_transfers.remove(&transfer_id);
                                                 continue;
                                            }

                                            // 다음 청크 보내기
                                            let mut finished = false;
                                            let mut file_name = String::new();
                                            let mut total_size = 0u64;
                                            let mut peer_id_str = String::new();
                                            let mut speed: Option<f64> = None;

                                            if let Some(transfer_state) = outgoing_transfers.get_mut(&transfer_id) {
                                                if transfer_state.current_chunk < transfer_state.total_chunks {
                                                     let mut buffer = vec![0u8; CHUNK_SIZE];
                                                     match transfer_state.file.read(&mut buffer) {
                                                        Ok(n) => {
                                                            if n == 0 {
                                                                finished = true; // 읽을 데이터 없음
                                                            } else {
                                                                buffer.truncate(n);
                                                                let req = FileTransferStreamRequest {
                                                                    msg: FileTransferStreamMsg::Chunk {
                                                                        transfer_id: transfer_id.clone(),
                                                                        chunk_index: transfer_state.current_chunk,
                                                                        data: buffer,
                                                                    }
                                                                };
                                                                let new_req_id = swarm.behaviour_mut().file_stream.send_request(&transfer_state.peer_id, req);
                                                                req_id_to_transfer_id.insert(new_req_id, transfer_id.clone());

                                                                // 진행도 업데이트
                                                                transfer_state.current_chunk += 1;
                                                                let progress = (transfer_state.current_chunk as f64 / transfer_state.total_chunks as f64) * 100.0;
                                                                app.emit("file-upload-progress", serde_json::json!({
                                                                    "transfer_id": transfer_id,
                                                                    "progress": progress,
                                                                    "current": transfer_state.current_chunk,
                                                                    "total": transfer_state.total_chunks
                                                                }).to_string()).ok();
                                                            }
                                                        }
                                                        Err(e) => {
                                                            println!("❌ 파일 읽기 에러: {}", e);
                                                            finished = true; // 에러로 중단
                                                        }
                                                     }
                                                } else {
                                                    finished = true;
                                                }

                                                if finished {
                                                    println!("✅ 파일 전송 완료: {}", transfer_state.file_name);
                                                    app.emit("file-upload-complete", serde_json::json!({
                                                        "transfer_id": transfer_id,
                                                        "file_name": transfer_state.file_name
                                                    }).to_string()).ok();

                                                    // 활동 기록 데이터 추출
                                                    let duration = transfer_state.start_time.elapsed().as_secs_f64();
                                                    speed = if duration > 0.0 { Some(transfer_state.total_size as f64 / duration) } else { None };
                                                    file_name = transfer_state.file_name.clone();
                                                    total_size = transfer_state.total_size;
                                                    peer_id_str = transfer_state.peer_id.to_string();
                                                }
                                            }

                                            // 상태 제거 및 활동 기록 (if let 블록 밖에서)
                                            if finished {
                                                outgoing_transfers.remove(&transfer_id);

                                                // 활동 기록 저장
                                                record_activity(
                                                    &state,
                                                    "upload",
                                                    file_name,
                                                    peer_id_str,
                                                    total_size,
                                                    "completed",
                                                    speed,
                                                ).await;
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }

                    // ====== 페어링 프로토콜 처리 ======
                    SwarmEvent::Behaviour(behavior::MyBehaviourEvent::Pairing(event)) => {
                        match event {
                            request_response::Event::Message { peer, message } => {
                                match message {
                                    // 페어링 요청 수신 (상대방이 나에게 연결 요청)
                                    request_response::Message::Request { request, channel, .. } => {
                                        println!("🔐 페어링 요청 수신: {} (PIN: {})", request.device_name, request.pin);

                                        // 대기 목록에 저장
                                        let peer_key = peer.to_string();
                                        pending_pairings.insert(peer_key.clone(), (peer, channel, request.pin.clone()));

                                        // 프론트엔드에 알림 (사용자 승인 필요)
                                        app.emit("pairing-request", serde_json::json!({
                                            "peer_id": peer.to_string(),
                                            "device_name": request.device_name,
                                            "pin": request.pin,
                                        }).to_string()).ok();
                                    }

                                    // 페어링 응답 수신 (상대방이 승인/거절)
                                    request_response::Message::Response { response, .. } => {
                                        if response.approved {
                                            trusted_peers.insert(peer);
                                            println!("✅ 페어링 승인됨: {}", response.device_name);
                                            app.emit("pairing-approved", serde_json::json!({
                                                "peer_id": peer.to_string(),
                                                "device_name": response.device_name,
                                                "shared_folder_name": response.shared_folder_name,
                                            }).to_string()).ok();
                                        } else {
                                            println!("❌ 페어링 거절됨: {:?}", response.error);
                                            app.emit("pairing-rejected", serde_json::json!({
                                                "peer_id": peer.to_string(),
                                                "error": response.error,
                                            }).to_string()).ok();
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }

                    // Disconnection handling might need to cleanup active transfers?
                    // We can do that later for optimization.

                    SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                        println!("connection-success PeerId: {}", peer_id);
                        let request = DeviceInfoRequest {};
                        swarm.behaviour_mut().device_info.send_request(&peer_id, request);
                        app.emit("connection-success", peer_id.to_string()).ok();
                    }
                    SwarmEvent::ConnectionClosed { peer_id, .. } => {
                        app.emit("connection-closed", peer_id.to_string()).ok();
                    }
                    SwarmEvent::NewListenAddr { address, .. } => {
                        // localhost는 로그 제외 (실제 네트워크 IP만 표시)
                        if !address.to_string().contains("127.0.0.1") {
                            println!("listening address: {}", address);
                            app.emit("listening-on", address.to_string()).ok();
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(())
}
