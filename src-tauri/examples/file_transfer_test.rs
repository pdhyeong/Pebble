// 파일 전송 테스트 예제
//
// 사용법:
// 1. 터미널 A: cargo run --example file_transfer_test -- --shared-folder ./test_shared_a --name "노드A"
// 2. 터미널 B: cargo run --example file_transfer_test -- --shared-folder ./test_shared_b --name "노드B"
//
// 두 노드가 mDNS로 발견되면 자동으로 연결되고, 파일 목록 및 파일 전송을 테스트합니다.

use futures::prelude::*;
use libp2p::{
    identify, mdns, ping, request_response, swarm::SwarmEvent, PeerId, StreamProtocol, SwarmBuilder,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::io::{self, Write as IoWrite};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileListRequest {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileListResponse {
    pub path: String,
    pub files: Vec<FileInfo>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTransferRequest {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTransferResponse {
    pub path: String,
    pub data: Option<Vec<u8>>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileUploadRequest {
    pub file_name: String,
    pub remote_path: String,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileUploadResponse {
    pub file_name: String,
    pub saved_path: String,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfoRequest {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfoResponse {
    pub device_name: String,
}

// ===== Codec 구현 =====

#[derive(Debug, Clone, Default)]
pub struct FileListCodec;

#[async_trait::async_trait]
impl request_response::Codec for FileListCodec {
    type Protocol = StreamProtocol;
    type Request = FileListRequest;
    type Response = FileListResponse;

    async fn read_request<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut len_buf = [0u8; 4];
        io.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;
        let mut buf = vec![0u8; len];
        io.read_exact(&mut buf).await?;
        serde_json::from_slice(&buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    async fn read_response<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut len_buf = [0u8; 4];
        io.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;
        let mut buf = vec![0u8; len];
        io.read_exact(&mut buf).await?;
        serde_json::from_slice(&buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    async fn write_request<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
        req: Self::Request,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let data =
            serde_json::to_vec(&req).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let len = (data.len() as u32).to_be_bytes();
        io.write_all(&len).await?;
        io.write_all(&data).await?;
        io.flush().await?;
        Ok(())
    }

    async fn write_response<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
        res: Self::Response,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let data =
            serde_json::to_vec(&res).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let len = (data.len() as u32).to_be_bytes();
        io.write_all(&len).await?;
        io.write_all(&data).await?;
        io.flush().await?;
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct FileTransferCodec;

#[async_trait::async_trait]
impl request_response::Codec for FileTransferCodec {
    type Protocol = StreamProtocol;
    type Request = FileTransferRequest;
    type Response = FileTransferResponse;

    async fn read_request<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut len_buf = [0u8; 4];
        io.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;
        let mut buf = vec![0u8; len];
        io.read_exact(&mut buf).await?;
        serde_json::from_slice(&buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    async fn read_response<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut len_buf = [0u8; 4];
        io.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;
        let mut buf = vec![0u8; len];
        io.read_exact(&mut buf).await?;
        serde_json::from_slice(&buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    async fn write_request<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
        req: Self::Request,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let data =
            serde_json::to_vec(&req).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let len = (data.len() as u32).to_be_bytes();
        io.write_all(&len).await?;
        io.write_all(&data).await?;
        io.flush().await?;
        Ok(())
    }

    async fn write_response<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
        res: Self::Response,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let data =
            serde_json::to_vec(&res).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let len = (data.len() as u32).to_be_bytes();
        io.write_all(&len).await?;
        io.write_all(&data).await?;
        io.flush().await?;
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct FileUploadCodec;

#[async_trait::async_trait]
impl request_response::Codec for FileUploadCodec {
    type Protocol = StreamProtocol;
    type Request = FileUploadRequest;
    type Response = FileUploadResponse;

    async fn read_request<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut len_buf = [0u8; 4];
        io.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;
        let mut buf = vec![0u8; len];
        io.read_exact(&mut buf).await?;
        serde_json::from_slice(&buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    async fn read_response<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut len_buf = [0u8; 4];
        io.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;
        let mut buf = vec![0u8; len];
        io.read_exact(&mut buf).await?;
        serde_json::from_slice(&buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    async fn write_request<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
        req: Self::Request,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let data =
            serde_json::to_vec(&req).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let len = (data.len() as u32).to_be_bytes();
        io.write_all(&len).await?;
        io.write_all(&data).await?;
        io.flush().await?;
        Ok(())
    }

    async fn write_response<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
        res: Self::Response,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let data =
            serde_json::to_vec(&res).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let len = (data.len() as u32).to_be_bytes();
        io.write_all(&len).await?;
        io.write_all(&data).await?;
        io.flush().await?;
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct DeviceInfoCodec;

#[async_trait::async_trait]
impl request_response::Codec for DeviceInfoCodec {
    type Protocol = StreamProtocol;
    type Request = DeviceInfoRequest;
    type Response = DeviceInfoResponse;

    async fn read_request<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut len_buf = [0u8; 4];
        io.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;
        let mut buf = vec![0u8; len];
        io.read_exact(&mut buf).await?;
        serde_json::from_slice(&buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    async fn read_response<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut len_buf = [0u8; 4];
        io.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;
        let mut buf = vec![0u8; len];
        io.read_exact(&mut buf).await?;
        serde_json::from_slice(&buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    async fn write_request<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
        req: Self::Request,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let data =
            serde_json::to_vec(&req).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let len = (data.len() as u32).to_be_bytes();
        io.write_all(&len).await?;
        io.write_all(&data).await?;
        io.flush().await?;
        Ok(())
    }

    async fn write_response<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
        res: Self::Response,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let data =
            serde_json::to_vec(&res).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let len = (data.len() as u32).to_be_bytes();
        io.write_all(&len).await?;
        io.write_all(&data).await?;
        io.flush().await?;
        Ok(())
    }
}

// ===== NetworkBehaviour =====

#[derive(libp2p::swarm::NetworkBehaviour)]
struct MyBehaviour {
    mdns: mdns::tokio::Behaviour,
    identify: identify::Behaviour,
    ping: ping::Behaviour,
    file_list: request_response::Behaviour<FileListCodec>,
    file_transfer: request_response::Behaviour<FileTransferCodec>,
    file_upload: request_response::Behaviour<FileUploadCodec>,
    device_info: request_response::Behaviour<DeviceInfoCodec>,
}

// ===== 파일 목록 조회 함수 =====

fn list_files(base_path: &PathBuf, relative_path: &str) -> FileListResponse {
    let full_path = if relative_path.is_empty() || relative_path == "/" {
        base_path.clone()
    } else {
        base_path.join(relative_path.trim_start_matches('/'))
    };

    if !base_path.exists() {
        if let Err(e) = std::fs::create_dir_all(base_path) {
            return FileListResponse {
                path: relative_path.to_string(),
                files: vec![],
                error: Some(format!("폴더 생성 실패: {}", e)),
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
                        path: entry
                            .path()
                            .strip_prefix(base_path)
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

// ===== 명령어 처리 =====

fn print_help() {
    println!("\n=== 사용 가능한 명령어 ===");
    println!("  list <peer_id>              - 상대 피어의 파일 목록 조회");
    println!("  list <peer_id> <path>       - 상대 피어의 특정 경로 파일 목록 조회");
    println!("  download <peer_id> <path>   - 상대 피어에서 파일 다운로드");
    println!("  upload <peer_id> <filename> - 상대 피어에게 파일 업로드 (공유폴더 내 파일)");
    println!("  peers                       - 연결된 피어 목록 보기");
    println!("  local                       - 내 공유 폴더 파일 목록 보기");
    println!("  help                        - 도움말 표시");
    println!("  quit                        - 프로그램 종료");
    println!("");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // 커맨드라인 인자 파싱
    let args: Vec<String> = std::env::args().collect();
    let mut shared_folder = PathBuf::from("./test_shared");
    let mut device_name = format!("테스트노드-{}", std::process::id());

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--shared-folder" | "-s" => {
                if i + 1 < args.len() {
                    shared_folder = PathBuf::from(&args[i + 1]);
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "--name" | "-n" => {
                if i + 1 < args.len() {
                    device_name = args[i + 1].clone();
                    i += 2;
                } else {
                    i += 1;
                }
            }
            _ => i += 1,
        }
    }

    // 공유 폴더 생성
    std::fs::create_dir_all(&shared_folder)?;
    let shared_folder = std::fs::canonicalize(&shared_folder)?;

    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║        P2P 파일 전송 테스트 노드                           ║");
    println!("╠════════════════════════════════════════════════════════════╣");
    println!("║ 기기 이름: {:<47} ║", device_name);
    println!("║ 공유 폴더: {:<47} ║", shared_folder.display());
    println!("╚════════════════════════════════════════════════════════════╝");
    println!("");

    // 테스트 파일 생성 (폴더에 파일이 없으면)
    let test_file = shared_folder.join("test_file.txt");
    if !test_file.exists() {
        std::fs::write(
            &test_file,
            format!(
                "안녕하세요! {}에서 보내는 테스트 파일입니다.\n생성 시간: {:?}",
                device_name,
                std::time::SystemTime::now()
            ),
        )?;
        println!("📝 테스트 파일 생성: {:?}", test_file);
    }

    let test_subdir = shared_folder.join("test_subdir");
    if !test_subdir.exists() {
        std::fs::create_dir_all(&test_subdir)?;
        std::fs::write(
            test_subdir.join("nested_file.txt"),
            "하위 폴더의 파일입니다.",
        )?;
        println!("📁 테스트 하위 폴더 생성: {:?}", test_subdir);
    }

    let mut swarm = SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(
            libp2p::tcp::Config::default(),
            libp2p::noise::Config::new,
            libp2p::yamux::Config::default,
        )?
        .with_behaviour(|key| {
            let mdns = mdns::tokio::Behaviour::new(Default::default(), key.public().to_peer_id())?;
            let identify = identify::Behaviour::new(identify::Config::new(
                "/pebble/1.0.0".into(),
                key.public(),
            ));
            let ping = ping::Behaviour::new(ping::Config::new());

            let file_list = request_response::Behaviour::new(
                [(
                    StreamProtocol::new("/pebble/file-list/1.0.0"),
                    request_response::ProtocolSupport::Full,
                )],
                request_response::Config::default(),
            );

            let file_transfer = request_response::Behaviour::new(
                [(
                    StreamProtocol::new("/pebble/file-transfer/1.0.0"),
                    request_response::ProtocolSupport::Full,
                )],
                request_response::Config::default(),
            );

            let file_upload = request_response::Behaviour::new(
                [(
                    StreamProtocol::new("/pebble/file-upload/1.0.0"),
                    request_response::ProtocolSupport::Full,
                )],
                request_response::Config::default(),
            );

            let device_info = request_response::Behaviour::new(
                [(
                    StreamProtocol::new("/pebble/device-info/1.0.0"),
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
                file_upload,
                device_info,
            })
        })?
        .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(Duration::from_secs(60 * 60)))
        .build();

    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    let local_peer_id = *swarm.local_peer_id();
    println!("🆔 내 PeerId: {}", local_peer_id);
    println!("\n⏳ 다른 피어를 찾는 중...");
    println!("💡 다른 터미널에서 이 예제를 실행하면 자동으로 연결됩니다.\n");
    print_help();

    // 연결된 피어 정보 저장
    let mut connected_peers: HashMap<PeerId, String> = HashMap::new();

    // stdin을 비동기로 읽기 위한 설정
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();

    // 별도 스레드에서 stdin 읽기
    std::thread::spawn(move || {
        let stdin = io::stdin();
        loop {
            print!("> ");
            io::stdout().flush().unwrap();
            let mut line = String::new();
            if stdin.read_line(&mut line).is_ok() {
                if tx.send(line.trim().to_string()).is_err() {
                    break;
                }
            }
        }
    });

    loop {
        tokio::select! {
            // 사용자 입력 처리
            Some(input) = rx.recv() => {
                let parts: Vec<&str> = input.split_whitespace().collect();
                if parts.is_empty() {
                    continue;
                }

                match parts[0] {
                    "help" | "h" => {
                        print_help();
                    }
                    "quit" | "q" | "exit" => {
                        println!("👋 종료합니다.");
                        break;
                    }
                    "peers" | "p" => {
                        if connected_peers.is_empty() {
                            println!("❌ 연결된 피어가 없습니다.");
                        } else {
                            println!("\n=== 연결된 피어 목록 ===");
                            for (peer_id, name) in &connected_peers {
                                println!("  {} ({})", name, peer_id);
                            }
                            println!("");
                        }
                    }
                    "local" | "l" => {
                        let response = list_files(&shared_folder, "/");
                        println!("\n=== 내 공유 폴더 ({}) ===", shared_folder.display());
                        if let Some(error) = response.error {
                            println!("❌ 오류: {}", error);
                        } else {
                            for file in response.files {
                                let file_type = if file.is_dir { "📁" } else { "📄" };
                                println!("  {} {} ({} bytes)", file_type, file.name, file.size);
                            }
                        }
                        println!("");
                    }
                    "list" => {
                        if parts.len() < 2 {
                            println!("❌ 사용법: list <peer_id> [path]");
                            println!("   예: list 12D3KooW... /");
                            continue;
                        }

                        // peer_id 찾기 (부분 매칭 지원)
                        let peer_query = parts[1];
                        let path = if parts.len() > 2 { parts[2] } else { "/" };

                        let target_peer = connected_peers.keys()
                            .find(|p| p.to_string().starts_with(peer_query) || p.to_string().contains(peer_query))
                            .cloned();

                        if let Some(peer_id) = target_peer {
                            println!("📂 {} 에게 파일 목록 요청 중... (경로: {})", peer_id, path);
                            let request = FileListRequest { path: path.to_string() };
                            swarm.behaviour_mut().file_list.send_request(&peer_id, request);
                        } else {
                            println!("❌ 피어를 찾을 수 없습니다: {}", peer_query);
                            println!("   'peers' 명령으로 연결된 피어 목록을 확인하세요.");
                        }
                    }
                    "download" | "d" => {
                        if parts.len() < 3 {
                            println!("❌ 사용법: download <peer_id> <file_path>");
                            println!("   예: download 12D3KooW... /test_file.txt");
                            continue;
                        }

                        let peer_query = parts[1];
                        let file_path = parts[2];

                        let target_peer = connected_peers.keys()
                            .find(|p| p.to_string().starts_with(peer_query) || p.to_string().contains(peer_query))
                            .cloned();

                        if let Some(peer_id) = target_peer {
                            println!("📥 {} 에서 파일 다운로드 요청 중... (파일: {})", peer_id, file_path);
                            let request = FileTransferRequest { path: file_path.to_string() };
                            swarm.behaviour_mut().file_transfer.send_request(&peer_id, request);
                        } else {
                            println!("❌ 피어를 찾을 수 없습니다: {}", peer_query);
                        }
                    }
                    "upload" | "u" => {
                        if parts.len() < 3 {
                            println!("❌ 사용법: upload <peer_id> <filename>");
                            println!("   예: upload 12D3KooW... test_file.txt");
                            continue;
                        }

                        let peer_query = parts[1];
                        let filename = parts[2];

                        let target_peer = connected_peers.keys()
                            .find(|p| p.to_string().starts_with(peer_query) || p.to_string().contains(peer_query))
                            .cloned();

                        if let Some(peer_id) = target_peer {
                            // 공유 폴더에서 파일 읽기
                            let file_path = shared_folder.join(filename);
                            match std::fs::read(&file_path) {
                                Ok(data) => {
                                    println!("📤 {} 에게 파일 업로드 중... (파일: {}, {} bytes)", peer_id, filename, data.len());
                                    let request = FileUploadRequest {
                                        file_name: filename.to_string(),
                                        remote_path: "/".to_string(),
                                        data,
                                    };
                                    swarm.behaviour_mut().file_upload.send_request(&peer_id, request);
                                }
                                Err(e) => {
                                    println!("❌ 파일 읽기 실패: {} - {}", filename, e);
                                }
                            }
                        } else {
                            println!("❌ 피어를 찾을 수 없습니다: {}", peer_query);
                        }
                    }
                    _ => {
                        println!("❌ 알 수 없는 명령어: {}", parts[0]);
                        println!("   'help'를 입력하면 사용 가능한 명령어를 볼 수 있습니다.");
                    }
                }
            }

            // Swarm 이벤트 처리
            event = swarm.select_next_some() => {
                match event {
                    // mDNS 피어 발견
                    SwarmEvent::Behaviour(MyBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                        for (peer_id, addr) in list {
                            if peer_id == local_peer_id {
                                continue;
                            }
                            println!("\n🔍 피어 발견: {} at {}", peer_id, addr);
                            // 자동으로 연결 시도
                            if !connected_peers.contains_key(&peer_id) {
                                if let Err(e) = swarm.dial(addr.clone()) {
                                    println!("❌ 연결 시도 실패: {}", e);
                                }
                            }
                        }
                    }

                    // mDNS 피어 만료
                    SwarmEvent::Behaviour(MyBehaviourEvent::Mdns(mdns::Event::Expired(list))) => {
                        for (peer_id, _) in list {
                            println!("\n⏰ 피어 만료: {}", peer_id);
                        }
                    }

                    // 연결 수립
                    SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
                        println!("\n✅ 연결 성공: {} ({:?})", peer_id, endpoint.get_remote_address());
                        connected_peers.insert(peer_id, format!("알 수 없음"));

                        // 기기 정보 요청
                        let request = DeviceInfoRequest {};
                        swarm.behaviour_mut().device_info.send_request(&peer_id, request);
                    }

                    // 연결 종료
                    SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
                        println!("\n🔌 연결 종료: {} (원인: {:?})", peer_id, cause);
                        connected_peers.remove(&peer_id);
                    }

                    // Identify 정보 수신
                    SwarmEvent::Behaviour(MyBehaviourEvent::Identify(identify::Event::Received { info, .. })) => {
                        println!("ℹ️  피어 정보: 프로토콜={}, 에이전트={}", info.protocol_version, info.agent_version);
                    }

                    // Ping 결과
                    SwarmEvent::Behaviour(MyBehaviourEvent::Ping(ping::Event { peer, result, .. })) => {
                        match result {
                            Ok(rtt) => {
                                println!("🏓 Ping 성공: {} (RTT: {:?})", peer, rtt);
                            }
                            Err(e) => {
                                println!("❌ Ping 실패: {} - {}", peer, e);
                            }
                        }
                    }

                    // 기기 정보 프로토콜
                    SwarmEvent::Behaviour(MyBehaviourEvent::DeviceInfo(event)) => {
                        match event {
                            request_response::Event::Message { peer, message } => {
                                match message {
                                    request_response::Message::Request { channel, .. } => {
                                        println!("📱 기기 정보 요청 수신: {}", peer);
                                        let response = DeviceInfoResponse {
                                            device_name: device_name.clone(),
                                        };
                                        if let Err(e) = swarm.behaviour_mut().device_info.send_response(channel, response) {
                                            println!("❌ 기기 정보 응답 실패: {:?}", e);
                                        }
                                    }
                                    request_response::Message::Response { response, .. } => {
                                        println!("📱 기기 정보 수신: {} -> \"{}\"", peer, response.device_name);
                                        connected_peers.insert(peer, response.device_name);
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

                    // 파일 목록 프로토콜
                    SwarmEvent::Behaviour(MyBehaviourEvent::FileList(event)) => {
                        match event {
                            request_response::Event::Message { peer, message } => {
                                match message {
                                    request_response::Message::Request { request, channel, .. } => {
                                        println!("\n📥 파일 목록 요청 수신: {} -> 경로: {}", peer, request.path);
                                        let response = list_files(&shared_folder, &request.path);
                                        println!("📤 응답 전송: {} 개의 파일/폴더", response.files.len());

                                        if let Err(e) = swarm.behaviour_mut().file_list.send_response(channel, response) {
                                            println!("❌ 파일 목록 응답 실패: {:?}", e);
                                        }
                                    }
                                    request_response::Message::Response { response, .. } => {
                                        println!("\n📂 파일 목록 응답 수신 (경로: {})", response.path);
                                        if let Some(error) = response.error {
                                            println!("❌ 오류: {}", error);
                                        } else {
                                            println!("=== {} 의 파일 목록 ===", peer);
                                            if response.files.is_empty() {
                                                println!("  (비어있음)");
                                            } else {
                                                for file in response.files {
                                                    let file_type = if file.is_dir { "📁" } else { "📄" };
                                                    println!("  {} {} ({} bytes) - 경로: {}", file_type, file.name, file.size, file.path);
                                                }
                                            }
                                        }
                                        println!("");
                                    }
                                }
                            }
                            request_response::Event::OutboundFailure { peer, error, .. } => {
                                println!("❌ 파일 목록 요청 실패: {} - {:?}", peer, error);
                            }
                            request_response::Event::InboundFailure { peer, error, .. } => {
                                println!("❌ 파일 목록 응답 실패: {} - {:?}", peer, error);
                            }
                            request_response::Event::ResponseSent { peer, .. } => {
                                println!("✅ 파일 목록 응답 전송 완료: {}", peer);
                            }
                        }
                    }

                    // 파일 전송 프로토콜
                    SwarmEvent::Behaviour(MyBehaviourEvent::FileTransfer(event)) => {
                        match event {
                            request_response::Event::Message { peer, message } => {
                                match message {
                                    request_response::Message::Request { request, channel, .. } => {
                                        println!("\n📥 파일 전송 요청 수신: {} -> {}", peer, request.path);
                                        let file_path = shared_folder.join(request.path.trim_start_matches('/'));

                                        let response = match std::fs::read(&file_path) {
                                            Ok(data) => {
                                                println!("📤 파일 전송 중: {} bytes", data.len());
                                                FileTransferResponse {
                                                    path: request.path,
                                                    data: Some(data),
                                                    error: None,
                                                }
                                            }
                                            Err(e) => {
                                                println!("❌ 파일 읽기 실패: {}", e);
                                                FileTransferResponse {
                                                    path: request.path,
                                                    data: None,
                                                    error: Some(format!("파일 읽기 실패: {}", e)),
                                                }
                                            }
                                        };

                                        if let Err(e) = swarm.behaviour_mut().file_transfer.send_response(channel, response) {
                                            println!("❌ 파일 전송 응답 실패: {:?}", e);
                                        }
                                    }
                                    request_response::Message::Response { response, .. } => {
                                        println!("\n📥 파일 전송 응답 수신: {}", response.path);

                                        if let Some(error) = &response.error {
                                            println!("❌ 오류: {}", error);
                                        } else if let Some(data) = &response.data {
                                            println!("✅ 파일 수신 완료: {} bytes", data.len());

                                            // 다운로드 폴더에 저장
                                            let download_folder = shared_folder.join("downloads");
                                            std::fs::create_dir_all(&download_folder).ok();

                                            let file_name = std::path::Path::new(&response.path)
                                                .file_name()
                                                .map(|n| n.to_string_lossy().to_string())
                                                .unwrap_or_else(|| "downloaded_file".to_string());
                                            let save_path = download_folder.join(&file_name);

                                            match std::fs::write(&save_path, data) {
                                                Ok(_) => {
                                                    println!("💾 파일 저장 완료: {:?}", save_path);

                                                    // 파일 내용 미리보기 (텍스트 파일인 경우)
                                                    if let Ok(content) = String::from_utf8(data.clone()) {
                                                        println!("\n--- 파일 내용 미리보기 ---");
                                                        let preview: String = content.chars().take(500).collect();
                                                        println!("{}", preview);
                                                        if content.len() > 500 {
                                                            println!("... (이하 생략)");
                                                        }
                                                        println!("--------------------------");
                                                    }
                                                }
                                                Err(e) => {
                                                    println!("❌ 파일 저장 실패: {}", e);
                                                }
                                            }
                                        }
                                        println!("");
                                    }
                                }
                            }
                            request_response::Event::OutboundFailure { peer, error, .. } => {
                                println!("❌ 파일 전송 요청 실패: {} - {:?}", peer, error);
                            }
                            request_response::Event::InboundFailure { peer, error, .. } => {
                                println!("❌ 파일 전송 응답 실패: {} - {:?}", peer, error);
                            }
                            request_response::Event::ResponseSent { peer, .. } => {
                                println!("✅ 파일 전송 완료: {}", peer);
                            }
                        }
                    }

                    // 파일 업로드 프로토콜
                    SwarmEvent::Behaviour(MyBehaviourEvent::FileUpload(event)) => {
                        match event {
                            request_response::Event::Message { peer, message } => {
                                match message {
                                    request_response::Message::Request { request, channel, .. } => {
                                        println!("\n📥 파일 업로드 요청 수신: {} -> {}/{}", peer, request.remote_path, request.file_name);

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

                                                    // 텍스트 파일이면 내용 미리보기
                                                    if let Ok(content) = String::from_utf8(request.data.clone()) {
                                                        println!("\n--- 받은 파일 내용 미리보기 ---");
                                                        let preview: String = content.chars().take(300).collect();
                                                        println!("{}", preview);
                                                        if content.len() > 300 {
                                                            println!("... (이하 생략)");
                                                        }
                                                        println!("-------------------------------");
                                                    }

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

                                        if let Err(e) = swarm.behaviour_mut().file_upload.send_response(channel, response) {
                                            println!("❌ 업로드 응답 전송 실패: {:?}", e);
                                        }
                                    }
                                    request_response::Message::Response { response, .. } => {
                                        println!("\n📤 파일 업로드 응답 수신: {} - {}", response.file_name, if response.success { "성공" } else { "실패" });
                                        if let Some(error) = response.error {
                                            println!("❌ 오류: {}", error);
                                        } else {
                                            println!("✅ 저장 경로: {}", response.saved_path);
                                        }
                                        println!("");
                                    }
                                }
                            }
                            request_response::Event::OutboundFailure { peer, error, .. } => {
                                println!("❌ 파일 업로드 요청 실패: {} - {:?}", peer, error);
                            }
                            request_response::Event::InboundFailure { peer, error, .. } => {
                                println!("❌ 파일 업로드 응답 실패: {} - {:?}", peer, error);
                            }
                            request_response::Event::ResponseSent { peer, .. } => {
                                println!("✅ 파일 업로드 응답 완료: {}", peer);
                            }
                        }
                    }

                    // 리스닝 주소
                    SwarmEvent::NewListenAddr { address, .. } => {
                        println!("👂 리스닝: {}", address);
                    }

                    _ => {}
                }
            }
        }
    }

    Ok(())
}
