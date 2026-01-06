use crate::api::{db, watcher, discovery};
use crate::api::db::FileMetadata;
use crate::api::discovery::DiscoveredDevice;

#[flutter_rust_bridge::frb(sync)]
pub fn greet(name: String) -> String {
    format!("Hello, {name}!")
}

#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    flutter_rust_bridge::setup_default_user_utils();

    // 로깅 초기화
    env_logger::init();

    if let Err(e) = db::init_db() {
        log::error!("Failed to initialize database: {}", e);
    } else {
        log::info!("Database initialized successfully.");
    }
}

/// 파일 변경 사항을 수동으로 기록합니다 (레거시 함수)
///
/// # Note
/// 이 함수는 이전 버전과의 호환성을 위해 유지되며,
/// 실시간 감시를 사용하는 경우 start_file_watcher를 사용하세요.
pub fn record_file_change(path: String, last_modified: i64, file_hash: String) {
    let file_metadata = FileMetadata {
        path,
        last_modified,
        file_hash,
        sync_status: "Pending".to_string(),
    };

    match db::upsert_file(file_metadata) {
        Ok(_) => log::info!("File change recorded successfully."),
        Err(e) => log::error!("Failed to record file change: {}", e),
    }
}

/// 특정 폴더에 대한 실시간 파일 감시를 시작합니다.
///
/// # Arguments
/// * `watch_path` - 감시할 디렉토리의 절대 경로
///
/// # Returns
/// * `Result<String, String>` - 성공 시 성공 메시지, 실패 시 에러 메시지
///
/// # Examples
/// ```dart
/// final result = await api.startFileWatcher(watchPath: "/path/to/sync/folder");
/// if (result.isOk) {
///   print("Watcher started: ${result.ok}");
/// } else {
///   print("Error: ${result.err}");
/// }
/// ```
///
/// # Security
/// - 경로가 존재하고 디렉토리인지 검증
/// - 백그라운드 스레드에서 실행되어 UI를 차단하지 않음
/// - 파일 변경 시 자동으로 blake3 해시 계산 및 DB 업데이트
pub fn start_file_watcher(watch_path: String) -> Result<String, String> {
    log::info!("Starting file watcher for: {}", watch_path);

    // 초기 디렉토리 스캔
    if let Err(e) = db::scan_directory(&watch_path) {
        let error_msg = format!("Failed to perform initial directory scan: {}", e);
        log::error!("{}", error_msg);
        return Err(error_msg);
    }

    // 파일 감시 시작
    match watcher::start_watching(&watch_path) {
        Ok(_) => {
            let success_msg = format!("File watcher started successfully for: {}", watch_path);
            log::info!("{}", success_msg);
            Ok(success_msg)
        }
        Err(e) => {
            let error_msg = format!("Failed to start file watcher: {}", e);
            log::error!("{}", error_msg);
            Err(error_msg)
        }
    }
}

/// 실시간 파일 감시를 중지합니다.
///
/// # Returns
/// * `Result<String, String>` - 성공 시 성공 메시지, 실패 시 에러 메시지
///
/// # Examples
/// ```dart
/// final result = await api.stopFileWatcher();
/// if (result.isOk) {
///   print("Watcher stopped: ${result.ok}");
/// }
/// ```
pub fn stop_file_watcher() -> Result<String, String> {
    match watcher::stop_watching() {
        Ok(_) => {
            let success_msg = "File watcher stopped successfully".to_string();
            log::info!("{}", success_msg);
            Ok(success_msg)
        }
        Err(e) => {
            let error_msg = format!("Failed to stop file watcher: {}", e);
            log::error!("{}", error_msg);
            Err(error_msg)
        }
    }
}

/// 동기화가 필요한 파일 목록을 가져옵니다.
///
/// # Returns
/// * `Result<Vec<String>, String>` - 성공 시 파일 경로 목록, 실패 시 에러 메시지
///
/// # Examples
/// ```dart
/// final result = await api.getPendingFiles();
/// if (result.isOk) {
///   for (final filePath in result.ok) {
///     print("Pending: $filePath");
///   }
/// }
/// ```
pub fn get_pending_files() -> Result<Vec<String>, String> {
    match db::get_pending_files() {
        Ok(files) => {
            log::debug!("Retrieved {} pending files", files.len());
            Ok(files)
        }
        Err(e) => {
            let error_msg = format!("Failed to get pending files: {}", e);
            log::error!("{}", error_msg);
            Err(error_msg)
        }
    }
}

/// 특정 파일의 동기화 상태를 업데이트합니다.
///
/// # Arguments
/// * `file_path` - 파일 경로
/// * `status` - 새로운 상태 ("Pending", "Synced", "Failed", "Deleted")
///
/// # Returns
/// * `Result<String, String>` - 성공 시 성공 메시지, 실패 시 에러 메시지
pub fn update_file_status(file_path: String, status: String) -> Result<String, String> {
    match db::update_sync_status(&file_path, &status) {
        Ok(_) => {
            let success_msg = format!("Updated {} to status: {}", file_path, status);
            log::info!("{}", success_msg);
            Ok(success_msg)
        }
        Err(e) => {
            let error_msg = format!("Failed to update file status: {}", e);
            log::error!("{}", error_msg);
            Err(error_msg)
        }
    }
}

// ============================================================================
// Phase 2: 기기 탐색 (Discovery) API
// ============================================================================

/// LAN에서 Pebble 기기 탐색을 시작합니다.
///
/// # Arguments
/// * `device_name` - 현재 기기의 이름 (예: "John's MacBook")
/// * `secret_key` - HMAC 인증을 위한 비밀 키 (모든 Pebble 기기가 공유)
///
/// # Returns
/// * `Result<String, String>` - 성공 시 기기 ID, 실패 시 에러 메시지
///
/// # Examples
/// ```dart
/// final result = await api.startDeviceDiscovery(
///   deviceName: "My Device",
///   secretKey: "my-secret-psk-key-2024"
/// );
/// if (result.isOk) {
///   print("Device ID: ${result.ok}");
/// }
/// ```
///
/// # Security
/// - UDP 브로드캐스트로 LAN 내 기기 탐색
/// - HMAC-SHA256으로 메시지 서명 및 검증
/// - 타임스탬프로 재생 공격(Replay Attack) 방지
/// - Pre-Shared Key (PSK) 방식의 인증
pub async fn start_device_discovery(device_name: String, secret_key: String) -> Result<String, String> {
    log::info!("Starting device discovery: {}", device_name);

    match discovery::start_discovery(device_name, secret_key).await {
        Ok(device_id) => {
            let success_msg = format!("Device discovery started. Device ID: {}", device_id);
            log::info!("{}", success_msg);
            Ok(device_id)
        }
        Err(e) => {
            let error_msg = format!("Failed to start device discovery: {}", e);
            log::error!("{}", error_msg);
            Err(error_msg)
        }
    }
}

/// 기기 탐색을 중지합니다.
///
/// # Returns
/// * `Result<String, String>` - 성공 시 성공 메시지, 실패 시 에러 메시지
///
/// # Examples
/// ```dart
/// final result = await api.stopDeviceDiscovery();
/// ```
pub fn stop_device_discovery() -> Result<String, String> {
    match discovery::stop_discovery() {
        Ok(_) => {
            let success_msg = "Device discovery stopped successfully".to_string();
            log::info!("{}", success_msg);
            Ok(success_msg)
        }
        Err(e) => {
            let error_msg = format!("Failed to stop device discovery: {}", e);
            log::error!("{}", error_msg);
            Err(error_msg)
        }
    }
}

/// 발견된 Pebble 기기 목록을 가져옵니다.
///
/// # Returns
/// * `Result<Vec<DiscoveredDevice>, String>` - 성공 시 기기 목록, 실패 시 에러 메시지
///
/// # Examples
/// ```dart
/// final result = await api.getDiscoveredDevices();
/// if (result.isOk) {
///   for (final device in result.ok) {
///     print("Device: ${device.deviceName} (${device.ipAddress})");
///   }
/// }
/// ```
pub fn get_discovered_devices() -> Result<Vec<DiscoveredDevice>, String> {
    match discovery::get_discovered_devices() {
        Ok(devices) => {
            log::debug!("Retrieved {} discovered devices", devices.len());
            Ok(devices)
        }
        Err(e) => {
            let error_msg = format!("Failed to get discovered devices: {}", e);
            log::error!("{}", error_msg);
            Err(error_msg)
        }
    }
}

// ============================================================================
// Phase 3: 암호화된 파일 전송 (Secure File Transfer) API
// ============================================================================

/// TLS 인증서를 생성하거나 로드합니다.
///
/// # Arguments
/// * `device_id` - 기기 고유 ID
/// * `device_name` - 기기 이름
/// * `cert_dir` - 인증서 저장 디렉토리
///
/// # Returns
/// * `Result<String, String>` - 성공 시 인증서 핑거프린트, 실패 시 에러 메시지
///
/// # Security
/// - RSA 2048비트 자기 서명 인증서 생성
/// - SHA-256 핑거프린트로 Certificate Pinning 지원
pub fn init_tls_certificate(
    device_id: String,
    device_name: String,
    cert_dir: String,
) -> Result<String, String> {
    use crate::api::certificate::CertificateManager;

    let manager = CertificateManager::new(cert_dir);

    match manager.get_or_create_certificate(&device_id, &device_name) {
        Ok(cert) => {
            log::info!("TLS certificate initialized. Fingerprint: {}", cert.fingerprint);
            Ok(cert.fingerprint)
        }
        Err(e) => {
            let error_msg = format!("Failed to initialize TLS certificate: {}", e);
            log::error!("{}", error_msg);
            Err(error_msg)
        }
    }
}

/// 파일 전송 서버를 시작합니다.
///
/// # Arguments
/// * `device_id` - 기기 고유 ID
/// * `device_name` - 기기 이름
/// * `cert_dir` - 인증서 디렉토리
/// * `bind_port` - 바인딩할 포트 (기본값: 37846)
///
/// # Returns
/// * `Result<String, String>` - 성공 시 성공 메시지, 실패 시 에러 메시지
///
/// # Security
/// - TLS 1.3 암호화 연결
/// - 자기 서명 인증서 사용
/// - Certificate Pinning으로 MITM 공격 방지
pub async fn start_transfer_server(
    device_id: String,
    device_name: String,
    cert_dir: String,
    bind_port: Option<u16>,
) -> Result<String, String> {
    use crate::api::certificate::CertificateManager;
    use crate::api::transfer::{TransferServer, TRANSFER_PORT};
    use std::net::SocketAddr;

    let manager = CertificateManager::new(cert_dir);
    let cert = manager.get_or_create_certificate(&device_id, &device_name)
        .map_err(|e| format!("Failed to load certificate: {}", e))?;

    let port = bind_port.unwrap_or(TRANSFER_PORT);
    let bind_addr: SocketAddr = format!("0.0.0.0:{}", port).parse()
        .map_err(|e| format!("Invalid bind address: {}", e))?;

    let server = TransferServer::new(cert);

    // 백그라운드에서 서버 실행
    tokio::spawn(async move {
        if let Err(e) = server.start(bind_addr).await {
            log::error!("Transfer server error: {}", e);
        }
    });

    let success_msg = format!("Transfer server started on port {}", port);
    log::info!("{}", success_msg);
    Ok(success_msg)
}

/// 파일을 다른 기기로 전송합니다.
///
/// # Arguments
/// * `server_ip` - 수신 기기의 IP 주소
/// * `server_port` - 수신 기기의 포트 (기본값: 37846)
/// * `file_path` - 전송할 파일 경로
/// * `server_fingerprint` - 수신 기기 인증서의 핑거프린트 (Certificate Pinning용, Optional)
///
/// # Returns
/// * `Result<String, String>` - 성공 시 전송 ID, 실패 시 에러 메시지
///
/// # Examples
/// ```dart
/// final result = await api.sendFile(
///   serverIp: "192.168.1.100",
///   serverPort: 37846,
///   filePath: "/path/to/file.pdf",
///   serverFingerprint: "a8f5f167f44f4964e6c998dee827110c...",
/// );
/// ```
pub async fn send_file(
    server_ip: String,
    server_port: Option<u16>,
    file_path: String,
    server_fingerprint: Option<String>,
) -> Result<String, String> {
    use crate::api::transfer::{TransferClient, TRANSFER_PORT};
    use std::net::SocketAddr;

    let port = server_port.unwrap_or(TRANSFER_PORT);
    let server_addr: SocketAddr = format!("{}:{}", server_ip, port).parse()
        .map_err(|e| format!("Invalid server address: {}", e))?;

    let client = TransferClient::new(server_fingerprint);

    // 파일 전송
    match client.send_file(server_addr, &file_path).await {
        Ok(_) => {
            let success_msg = format!("File sent successfully: {}", file_path);
            log::info!("{}", success_msg);
            Ok(success_msg)
        }
        Err(e) => {
            let error_msg = format!("Failed to send file: {}", e);
            log::error!("{}", error_msg);
            Err(error_msg)
        }
    }
}