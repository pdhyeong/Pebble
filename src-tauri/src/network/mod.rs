pub mod behavior;

use behavior::{
    MyBehaviour, FileListRequest, FileListResponse, FileInfo,
    FileTransferRequest, FileTransferResponse,
    FileUploadRequest, FileUploadResponse,
    DeviceInfoRequest, DeviceInfoResponse,
};
use futures::StreamExt;
use libp2p::{swarm::SwarmEvent, request_response, Multiaddr, PeerId, StreamProtocol, SwarmBuilder};
use std::error::Error;
use std::time::Duration;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;

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

pub struct FileListRequestMsg {
    pub peer_id: PeerId,
    pub path: String,
}

// 파일 전송 요청 메시지
pub struct FileTransferRequestMsg {
    pub peer_id: PeerId,
    pub path: String,
}

// 파일 업로드 요청 메시지
pub struct FileUploadRequestMsg {
    pub peer_id: PeerId,
    pub file_name: String,
    pub remote_path: String,
    pub data: Vec<u8>,
}

// 기기 정보 메시지 (lib.rs에서 사용하지 않지만 export 필요)
pub struct DeviceInfoMsg {
    pub peer_id: PeerId,
}

pub async fn run_p2p_engine(
    app: AppHandle,
    mut dial_rx: mpsc::UnboundedReceiver<String>,
    mut file_list_rx: mpsc::UnboundedReceiver<FileListRequestMsg>,
    mut file_transfer_rx: mpsc::UnboundedReceiver<FileTransferRequestMsg>,
    mut file_upload_rx: mpsc::UnboundedReceiver<FileUploadRequestMsg>,
    mut shared_folder_rx: mpsc::UnboundedReceiver<PathBuf>,
    mut shutdown_rx: mpsc::Receiver<()>,
    initial_shared_folder: PathBuf,
    device_name: String,
) -> Result<(), Box<dyn Error>> {
    // 공유 폴더 경로를 변경 가능하게
    let mut shared_folder = initial_shared_folder;
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

            // 파일 전송 프로토콜 (다운로드)
            let file_transfer = request_response::Behaviour::new(
                [(StreamProtocol::new("/pebble/file-transfer/1.0.0"), request_response::ProtocolSupport::Full)],
                request_response::Config::default(),
            );

            // 파일 업로드 프로토콜 (전송)
            let file_upload = request_response::Behaviour::new(
                [(StreamProtocol::new("/pebble/file-upload/1.0.0"), request_response::ProtocolSupport::Full)],
                request_response::Config::default(),
            );

            // 기기 정보 프로토콜
            let device_info = request_response::Behaviour::new(
                [(StreamProtocol::new("/pebble/device-info/1.0.0"), request_response::ProtocolSupport::Full)],
                request_response::Config::default(),
            );

            Ok(MyBehaviour { mdns, identify, ping, file_list, file_transfer, file_upload, device_info })
        })?
        .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(Duration::from_secs(60 * 60)))
        .build();

    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

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

            // 파일 업로드 요청 처리 (전송)
            Some(req) = file_upload_rx.recv() => {
                println!("📤 파일 업로드 요청: peer={}, file={}, remote_path={}", req.peer_id, req.file_name, req.remote_path);
                let request = FileUploadRequest {
                    file_name: req.file_name.clone(),
                    remote_path: req.remote_path.clone(),
                    data: req.data,
                };
                swarm.behaviour_mut().file_upload.send_request(&req.peer_id, request);
                app.emit("file-upload-started", serde_json::json!({
                    "peer_id": req.peer_id.to_string(),
                    "file_name": req.file_name,
                    "remote_path": req.remote_path,
                }).to_string()).ok();
            }

            // 공유 폴더 경로 변경 처리
            Some(new_path) = shared_folder_rx.recv() => {
                println!("📁 공유 폴더 경로 변경: {:?}", new_path);
                shared_folder = new_path;
                app.emit("shared-folder-changed", shared_folder.display().to_string()).ok();
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

                    // 파일 전송 요청/응답 처리
                    SwarmEvent::Behaviour(behavior::MyBehaviourEvent::FileTransfer(event)) => {
                        match event {
                            request_response::Event::Message { peer, message } => {
                                match message {
                                    // 요청 수신 - 파일 읽어서 전송
                                    request_response::Message::Request { request, channel, .. } => {
                                        println!("📥 파일 전송 요청 수신: {} -> {}", peer, request.path);
                                        let file_path = shared_folder.join(request.path.trim_start_matches('/'));

                                        let response = match std::fs::read(&file_path) {
                                            Ok(data) => FileTransferResponse {
                                                path: request.path,
                                                data: Some(data),
                                                error: None,
                                            },
                                            Err(e) => FileTransferResponse {
                                                path: request.path,
                                                data: None,
                                                error: Some(format!("파일 읽기 실패: {}", e)),
                                            },
                                        };

                                        if let Err(e) = swarm.behaviour_mut().file_transfer.send_response(channel, response) {
                                            println!("파일 전송 응답 실패: {:?}", e);
                                        }
                                    }

                                    // 응답 수신 - 파일 저장 및 프론트엔드 알림
                                    request_response::Message::Response { response, .. } => {
                                        if let Some(error) = &response.error {
                                            println!("❌ 파일 전송 실패: {}", error);
                                            let payload = serde_json::json!({
                                                "peer_id": peer.to_string(),
                                                "path": response.path,
                                                "error": error,
                                            });
                                            app.emit("file-download-error", payload.to_string()).ok();
                                        } else if let Some(data) = &response.data {
                                            println!("📤 파일 수신 완료: {} ({} bytes)", response.path, data.len());

                                            // 다운로드 폴더에 저장
                                            let download_folder = dirs::download_dir()
                                                .unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from(".")));
                                            let file_name = std::path::Path::new(&response.path)
                                                .file_name()
                                                .map(|n| n.to_string_lossy().to_string())
                                                .unwrap_or_else(|| "downloaded_file".to_string());
                                            let save_path = download_folder.join(&file_name);

                                            match std::fs::write(&save_path, data) {
                                                Ok(_) => {
                                                    println!("✅ 파일 저장 완료: {:?}", save_path);
                                                    let payload = serde_json::json!({
                                                        "peer_id": peer.to_string(),
                                                        "path": response.path,
                                                        "saved_path": save_path.display().to_string(),
                                                        "size": data.len(),
                                                    });
                                                    app.emit("file-download-complete", payload.to_string()).ok();
                                                }
                                                Err(e) => {
                                                    println!("❌ 파일 저장 실패: {}", e);
                                                    let payload = serde_json::json!({
                                                        "peer_id": peer.to_string(),
                                                        "path": response.path,
                                                        "error": format!("파일 저장 실패: {}", e),
                                                    });
                                                    app.emit("file-download-error", payload.to_string()).ok();
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            request_response::Event::OutboundFailure { peer, error, .. } => {
                                println!("❌ 파일 전송 요청 실패: {} - {:?}", peer, error);
                                app.emit("file-download-error", format!("{}:{:?}", peer, error)).ok();
                            }
                            request_response::Event::InboundFailure { peer, error, .. } => {
                                println!("❌ 파일 전송 응답 실패: {} - {:?}", peer, error);
                            }
                            request_response::Event::ResponseSent { peer, .. } => {
                                println!("✅ 파일 전송 완료: {}", peer);
                            }
                        }
                    }

                    // 파일 업로드 요청/응답 처리
                    SwarmEvent::Behaviour(behavior::MyBehaviourEvent::FileUpload(event)) => {
                        match event {
                            request_response::Event::Message { peer, message } => {
                                match message {
                                    // 요청 수신 - 파일 저장
                                    request_response::Message::Request { request, channel, .. } => {
                                        println!("📥 파일 업로드 요청 수신: {} -> {}/{}", peer, request.remote_path, request.file_name);

                                        // 저장 경로 결정
                                        let save_dir = if request.remote_path.is_empty() || request.remote_path == "/" {
                                            shared_folder.clone()
                                        } else {
                                            shared_folder.join(request.remote_path.trim_start_matches('/'))
                                        };

                                        // 폴더 생성
                                        let response = if let Err(e) = std::fs::create_dir_all(&save_dir) {
                                            FileUploadResponse {
                                                file_name: request.file_name.clone(),
                                                saved_path: String::new(),
                                                success: false,
                                                error: Some(format!("폴더 생성 실패: {}", e)),
                                            }
                                        } else {
                                            let save_path = save_dir.join(&request.file_name);

                                            match std::fs::write(&save_path, &request.data) {
                                                Ok(_) => {
                                                    println!("✅ 파일 저장 완료: {:?} ({} bytes)", save_path, request.data.len());
                                                    FileUploadResponse {
                                                        file_name: request.file_name.clone(),
                                                        saved_path: save_path.display().to_string(),
                                                        success: true,
                                                        error: None,
                                                    }
                                                }
                                                Err(e) => {
                                                    println!("❌ 파일 저장 실패: {}", e);
                                                    FileUploadResponse {
                                                        file_name: request.file_name.clone(),
                                                        saved_path: String::new(),
                                                        success: false,
                                                        error: Some(format!("파일 저장 실패: {}", e)),
                                                    }
                                                }
                                            }
                                        };

                                        // 파일 수신 완료 이벤트 (내가 파일을 받았을 때)
                                        let notify_payload = serde_json::json!({
                                            "peer_id": peer.to_string(),
                                            "file_name": request.file_name,
                                            "saved_path": response.saved_path.clone(),
                                            "success": response.success,
                                            "error": response.error.clone(),
                                        });
                                        app.emit("file-received", notify_payload.to_string()).ok();

                                        if let Err(e) = swarm.behaviour_mut().file_upload.send_response(channel, response) {
                                            println!("❌ 업로드 응답 전송 실패: {:?}", e);
                                        }
                                    }

                                    // 응답 수신 - 업로드 결과 확인
                                    request_response::Message::Response { response, .. } => {
                                        println!("📤 파일 업로드 응답 수신: {} - {}", response.file_name, if response.success { "성공" } else { "실패" });
                                        let payload = serde_json::json!({
                                            "peer_id": peer.to_string(),
                                            "file_name": response.file_name,
                                            "saved_path": response.saved_path,
                                            "success": response.success,
                                            "error": response.error,
                                        });

                                        if response.success {
                                            app.emit("file-upload-complete", payload.to_string()).ok();
                                        } else {
                                            app.emit("file-upload-error", payload.to_string()).ok();
                                        }
                                    }
                                }
                            }
                            request_response::Event::OutboundFailure { peer, error, .. } => {
                                println!("❌ 파일 업로드 요청 실패: {} - {:?}", peer, error);
                                app.emit("file-upload-error", serde_json::json!({
                                    "peer_id": peer.to_string(),
                                    "error": format!("{:?}", error),
                                }).to_string()).ok();
                            }
                            request_response::Event::InboundFailure { peer, error, .. } => {
                                println!("❌ 파일 업로드 응답 실패: {} - {:?}", peer, error);
                            }
                            request_response::Event::ResponseSent { peer, .. } => {
                                println!("✅ 파일 업로드 응답 완료: {}", peer);
                            }
                        }
                    }

                    // 기기 정보 요청/응답 처리
                    SwarmEvent::Behaviour(behavior::MyBehaviourEvent::DeviceInfo(event)) => {
                        match event {
                            request_response::Event::Message { peer, message } => {
                                match message {
                                    // 요청 수신 - 내 기기 이름 응답
                                    request_response::Message::Request { channel, .. } => {
                                        println!("📱 기기 정보 요청 수신: {}", peer);
                                        let response = DeviceInfoResponse {
                                            device_name: device_name.clone(),
                                        };
                                        if let Err(e) = swarm.behaviour_mut().device_info.send_response(channel, response) {
                                            println!("기기 정보 응답 실패: {:?}", e);
                                        }
                                    }

                                    // 응답 수신 - 상대방 기기 이름 저장
                                    request_response::Message::Response { response, .. } => {
                                        println!("📱 기기 정보 수신: {} -> {}", peer, response.device_name);
                                        let payload = serde_json::json!({
                                            "peer_id": peer.to_string(),
                                            "device_name": response.device_name,
                                        });
                                        app.emit("device-info-received", payload.to_string()).ok();
                                    }
                                }
                            }
                            request_response::Event::OutboundFailure { peer, error, .. } => {
                                println!("❌ 기기 정보 요청 실패: {} - {:?}", peer, error);
                            }
                            request_response::Event::InboundFailure { peer, error, .. } => {
                                println!("❌ 기기 정보 응답 실패: {} - {:?}", peer, error);
                            }
                            request_response::Event::ResponseSent { peer, .. } => {
                                println!("✅ 기기 정보 응답 완료: {}", peer);
                            }
                        }
                    }

                    // 연결 수립
                    SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
                        println!("connection-success PeerId: {}, Endpoint: {:?}", peer_id, endpoint);

                        // 연결이 수립되면 기기 정보 요청
                        let request = DeviceInfoRequest {};
                        swarm.behaviour_mut().device_info.send_request(&peer_id, request);

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
    Ok(())
}
