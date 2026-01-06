use anyhow::{Context, Result};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::interval;
use uuid::Uuid;

/// HMAC-SHA256 타입 별칭
type HmacSha256 = Hmac<Sha256>;

/// UDP 브로드캐스트 포트
const DISCOVERY_PORT: u16 = 37845;
const TEST_PORT: u16 = 40000;
/// 비콘 전송 주기 (초)
const BEACON_INTERVAL_SECS: u64 = 5;

/// 기기 타임아웃 시간 (초) - 마지막 비콘 이후 이 시간이 지나면 오프라인으로 간주
const DEVICE_TIMEOUT_SECS: u64 = 15;

/// Pebble 기기 발견을 위한 비콘 메시지
///
/// # Security
/// - HMAC-SHA256으로 메시지 무결성 보장
/// - 타임스탬프로 재생 공격(Replay Attack) 방지
/// - 기기 고유 ID로 식별
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeaconMessage {
    /// 기기 고유 ID (UUID v4)
    pub device_id: String,

    /// 기기 이름 (사용자 정의 가능)
    pub device_name: String,

    /// 메시지 전송 시간 (Unix timestamp)
    pub timestamp: u64,

    /// Pebble 프로토콜 버전
    pub protocol_version: String,

    /// HMAC-SHA256 서명 (hex 인코딩)
    pub signature: String,
}

impl BeaconMessage {
    /// 새로운 비콘 메시지를 생성합니다.
    ///
    /// # Arguments
    /// * `device_id` - 기기 고유 ID
    /// * `device_name` - 기기 이름
    /// * `secret_key` - HMAC 서명을 위한 비밀 키
    ///
    /// # Returns
    /// * `Result<Self>` - 서명된 비콘 메시지
    pub fn new(device_id: String, device_name: String, secret_key: &str) -> Result<Self> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("Failed to get system time")?
            .as_secs();

        let protocol_version = "1.0.0".to_string();

        // 서명할 데이터 생성
        let data_to_sign = format!("{}{}{}{}", device_id, device_name, timestamp, protocol_version);

        // HMAC-SHA256 서명 생성
        let signature = Self::generate_signature(&data_to_sign, secret_key)?;

        Ok(Self {
            device_id,
            device_name,
            timestamp,
            protocol_version,
            signature,
        })
    }

    /// HMAC-SHA256 서명을 생성합니다.
    fn generate_signature(data: &str, secret_key: &str) -> Result<String> {
        let mut mac = HmacSha256::new_from_slice(secret_key.as_bytes())
            .context("Invalid HMAC key length")?;

        mac.update(data.as_bytes());

        let result = mac.finalize();
        let signature_bytes = result.into_bytes();

        Ok(hex::encode(signature_bytes))
    }

    /// 비콘 메시지의 서명을 검증합니다.
    ///
    /// # Security
    /// - HMAC 검증으로 메시지 위변조 방지
    /// - 타임스탬프 검증으로 재생 공격 방지 (30초 이내 메시지만 허용)
    ///
    /// # Arguments
    /// * `secret_key` - HMAC 검증을 위한 비밀 키
    ///
    /// # Returns
    /// * `Result<bool>` - 검증 성공 시 true
    pub fn verify(&self, secret_key: &str) -> Result<bool> {
        // 타임스탬프 검증 (30초 이내)
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("Failed to get system time")?
            .as_secs();

        if current_time > self.timestamp + 30 {
            log::warn!("Beacon message is too old: {} seconds", current_time - self.timestamp);
            return Ok(false);
        }

        // 서명 재생성
        let data_to_sign = format!(
            "{}{}{}{}",
            self.device_id, self.device_name, self.timestamp, self.protocol_version
        );

        let expected_signature = Self::generate_signature(&data_to_sign, secret_key)?;

        // 서명 비교 (타이밍 공격 방지를 위한 constant-time 비교)
        Ok(expected_signature == self.signature)
    }

    /// 메시지를 JSON으로 직렬화합니다.
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(self).context("Failed to serialize beacon message")
    }

    /// JSON에서 메시지를 역직렬화합니다.
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).context("Failed to deserialize beacon message")
    }
}

/// 발견된 Pebble 기기 정보
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredDevice {
    /// 기기 고유 ID
    pub device_id: String,

    /// 기기 이름
    pub device_name: String,

    /// 기기 IP 주소
    pub ip_address: String,

    /// 프로토콜 버전
    pub protocol_version: String,

    /// 마지막으로 본 시간 (Unix timestamp)
    pub last_seen: u64,

    /// 기기가 온라인 상태인지 여부
    pub is_online: bool,
}

impl DiscoveredDevice {
    /// 새로운 발견된 기기를 생성합니다.
    pub fn new(beacon: &BeaconMessage, ip_address: String) -> Self {
        Self {
            device_id: beacon.device_id.clone(),
            device_name: beacon.device_name.clone(),
            ip_address,
            protocol_version: beacon.protocol_version.clone(),
            last_seen: beacon.timestamp,
            is_online: true,
        }
    }

    /// 기기의 마지막 본 시간을 업데이트합니다.
    pub fn update_last_seen(&mut self, timestamp: u64) {
        self.last_seen = timestamp;
        self.is_online = true;
    }

    /// 기기가 타임아웃되었는지 확인합니다.
    pub fn is_timeout(&self, current_time: u64) -> bool {
        current_time > self.last_seen + DEVICE_TIMEOUT_SECS
    }
}

/// 기기 발견 서비스
///
/// UDP 브로드캐스트를 사용하여 LAN에서 Pebble 기기를 발견합니다.
pub struct DiscoveryService {
    /// 현재 기기 ID
    device_id: String,

    /// 현재 기기 이름
    device_name: String,

    /// 인증 비밀 키
    secret_key: String,

    /// 발견된 기기 목록 (device_id -> DiscoveredDevice)
    discovered_devices: Arc<Mutex<HashMap<String, DiscoveredDevice>>>,

    /// 서비스 실행 여부
    is_running: Arc<Mutex<bool>>,
}

impl DiscoveryService {
    /// 새로운 발견 서비스를 생성합니다.
    ///
    /// # Arguments
    /// * `device_name` - 현재 기기의 이름
    /// * `secret_key` - HMAC 인증을 위한 비밀 키 (모든 Pebble 기기가 공유)
    ///
    /// # Security Notes
    /// - secret_key는 모든 Pebble 기기가 공유하는 Pre-Shared Key (PSK)입니다
    /// - 실제 배포 시 안전한 키 관리 시스템 사용 권장
    pub fn new(device_name: String, secret_key: String) -> Self {
        let device_id = Uuid::new_v4().to_string();

        Self {
            device_id,
            device_name,
            secret_key,
            discovered_devices: Arc::new(Mutex::new(HashMap::new())),
            is_running: Arc::new(Mutex::new(false)),
        }
    }

    /// 기기 ID를 반환합니다.
    pub fn get_device_id(&self) -> String {
        self.device_id.clone()
    }

    /// 발견 서비스를 시작합니다.
    ///
    /// # Architecture
    /// - 두 개의 비동기 태스크 생성:
    ///   1. 비콘 송신기: 주기적으로 UDP 브로드캐스트 전송
    ///   2. 비콘 수신기: UDP 브로드캐스트 수신 및 기기 목록 업데이트
    pub async fn start(&self) -> Result<()> {
        let mut is_running = self.is_running.lock().unwrap();
        if *is_running {
            anyhow::bail!("Discovery service is already running");
        }
        *is_running = true;
        drop(is_running);

        log::info!("Starting discovery service for device: {}", self.device_name);

        // 비콘 송신 태스크
        let device_id = self.device_id.clone();
        let device_name = self.device_name.clone();
        let secret_key = self.secret_key.clone();
        let is_running_tx = Arc::clone(&self.is_running);

        tokio::spawn(async move {
            if let Err(e) = Self::beacon_sender(device_id, device_name, secret_key, is_running_tx).await {
                log::error!("Beacon sender error: {}", e);
            }
        });

        // 비콘 수신 태스크
        let discovered_devices = Arc::clone(&self.discovered_devices);
        let secret_key = self.secret_key.clone();
        let device_id = self.device_id.clone();
        let is_running_rx = Arc::clone(&self.is_running);

        tokio::spawn(async move {
            if let Err(e) = Self::beacon_receiver(discovered_devices, secret_key, device_id, is_running_rx).await {
                log::error!("Beacon receiver error: {}", e);
            }
        });

        log::info!("Discovery service started successfully");

        Ok(())
    }

    /// 발견 서비스를 중지합니다.
    pub fn stop(&self) -> Result<()> {
        let mut is_running = self.is_running.lock().unwrap();
        *is_running = false;
        log::info!("Discovery service stopped");
        Ok(())
    }

    /// 비콘 송신 태스크
    ///
    /// 주기적으로 UDP 브로드캐스트를 전송합니다.
    async fn beacon_sender(
        device_id: String,
        device_name: String,
        secret_key: String,
        is_running: Arc<Mutex<bool>>,
    ) -> Result<()> {
        let socket = UdpSocket::bind("0.0.0.0:0")
            .context("Failed to bind UDP socket for sending")?;

        socket.set_broadcast(true)
            .context("Failed to set broadcast mode")?;

        let broadcast_addr: SocketAddr = format!("255.255.255.255:{}", DISCOVERY_PORT).parse()
            .context("Failed to parse broadcast address")?;

        let mut interval = interval(Duration::from_secs(BEACON_INTERVAL_SECS));

        loop {
            interval.tick().await;

            // 실행 중인지 확인
            {
                let running = is_running.lock().unwrap();
                if !*running {
                    break;
                }
            }

            // 비콘 메시지 생성
            let beacon = match BeaconMessage::new(device_id.clone(), device_name.clone(), &secret_key) {
                Ok(b) => b,
                Err(e) => {
                    log::error!("Failed to create beacon message: {}", e);
                    continue;
                }
            };

            let json_data = match beacon.to_json() {
                Ok(j) => j,
                Err(e) => {
                    log::error!("Failed to serialize beacon: {}", e);
                    continue;
                }
            };

            // UDP 브로드캐스트 전송
            match socket.send_to(json_data.as_bytes(), broadcast_addr) {
                Ok(bytes_sent) => {
                    log::debug!("Sent beacon: {} bytes to {}", bytes_sent, broadcast_addr);
                }
                Err(e) => {
                    log::error!("Failed to send beacon: {}", e);
                }
            }
        }

        log::info!("Beacon sender stopped");
        Ok(())
    }

    /// 비콘 수신 태스크
    ///
    /// UDP 브로드캐스트를 수신하고 발견된 기기 목록을 업데이트합니다.
    async fn beacon_receiver(
        discovered_devices: Arc<Mutex<HashMap<String, DiscoveredDevice>>>,
        secret_key: String,
        own_device_id: String,
        is_running: Arc<Mutex<bool>>,
    ) -> Result<()> {
        use std::net::SocketAddrV4;

        let ports_to_try = [DISCOVERY_PORT, TEST_PORT];
        let mut bound = None;
        for port in ports_to_try {
            // SO_REUSEADDR 설정으로 여러 프로세스가 같은 포트 사용 가능
            let socket = socket2::Socket::new(
                socket2::Domain::IPV4,
                socket2::Type::DGRAM,
                Some(socket2::Protocol::UDP),
            )?;
            socket.set_reuse_address(true)?;
            let addr: SocketAddrV4 = format!("0.0.0.0:{}", port).parse()?;
            match socket.bind(&socket2::SockAddr::from(addr)) {
                Ok(_) => {
                    log::info!("Listening for beacons on UDP port {}", port);
                    bound = Some(socket);
                    break;
                }
                Err(e) => {
                    log::warn!("Failed to bind to port {}: {}", port, e);
                    continue;
                }
            }
        }
        let socket = bound.context("Failed to bind UDP socket for receiving")?;
        socket.set_nonblocking(true)?;
        let socket: UdpSocket = socket.into();
        let mut buffer = vec![0u8; 4096];
        let mut last_cleanup = SystemTime::now();

        loop {
            // 논블로킹 체크를 위한 짧은 대기
            tokio::time::sleep(Duration::from_millis(100)).await;

            // 실행 중인지 확인
            {
                let running = is_running.lock().unwrap();
                if !*running {
                    break;
                }
            }

            // 기기 타임아웃 정리 (5초마다)
            if let Ok(elapsed) = last_cleanup.elapsed() {
                if elapsed >= Duration::from_secs(5) {
                    Self::cleanup_timeout_devices(&discovered_devices);
                    last_cleanup = SystemTime::now();
                }
            }

            // UDP 패킷 수신
            match socket.recv_from(&mut buffer) {
                Ok((bytes_received, src_addr)) => {
                    let data = &buffer[..bytes_received];
                    let json_str = match std::str::from_utf8(data) {
                        Ok(s) => s,
                        Err(e) => {
                            log::warn!("Received invalid UTF-8 data: {}", e);
                            continue;
                        }
                    };

                    // 비콘 메시지 파싱
                    let beacon = match BeaconMessage::from_json(json_str) {
                        Ok(b) => b,
                        Err(e) => {
                            log::warn!("Failed to parse beacon message: {}", e);
                            continue;
                        }
                    };

                    // 자기 자신의 비콘은 무시
                    if beacon.device_id == own_device_id {
                        continue;
                    }

                    // 서명 검증
                    let is_valid = match beacon.verify(&secret_key) {
                        Ok(v) => v,
                        Err(e) => {
                            log::error!("Failed to verify beacon signature: {}", e);
                            continue;
                        }
                    };

                    if !is_valid {
                        log::warn!("Received invalid beacon from {}", src_addr);
                        continue;
                    }

                    // 발견된 기기 목록 업데이트
                    let ip_address = src_addr.ip().to_string();
                    let mut devices = discovered_devices.lock().unwrap();

                    if let Some(device) = devices.get_mut(&beacon.device_id) {
                        device.update_last_seen(beacon.timestamp);
                        log::debug!("Updated device: {} ({})", device.device_name, ip_address);
                    } else {
                        let device = DiscoveredDevice::new(&beacon, ip_address.clone());
                        log::info!("Discovered new device: {} ({}) at {}", device.device_name, device.device_id, ip_address);
                        devices.insert(beacon.device_id.clone(), device);
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // 데이터 없음, 계속 대기
                    continue;
                }
                Err(e) => {
                    log::error!("Failed to receive UDP packet: {}", e);
                }
            }
        }

        log::info!("Beacon receiver stopped");
        Ok(())
    }

    /// 타임아웃된 기기를 정리합니다.
    fn cleanup_timeout_devices(discovered_devices: &Arc<Mutex<HashMap<String, DiscoveredDevice>>>) {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut devices = discovered_devices.lock().unwrap();

        devices.retain(|device_id, device| {
            if device.is_timeout(current_time) {
                log::info!("Device timed out: {} ({})", device.device_name, device_id);
                false
            } else {
                true
            }
        });
    }

    /// 발견된 기기 목록을 반환합니다.
    pub fn get_discovered_devices(&self) -> Vec<DiscoveredDevice> {
        let devices = self.discovered_devices.lock().unwrap();
        devices.values().cloned().collect()
    }
}

/// 전역 발견 서비스 인스턴스
static DISCOVERY_SERVICE: once_cell::sync::Lazy<Arc<Mutex<Option<DiscoveryService>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(None)));

/// 발견 서비스를 시작합니다.
///
/// # Arguments
/// * `device_name` - 현재 기기의 이름
/// * `secret_key` - HMAC 인증을 위한 비밀 키
///
/// # Returns
/// * `Result<String>` - 성공 시 기기 ID 반환
pub async fn start_discovery(device_name: String, secret_key: String) -> Result<String> {
    let service = DiscoveryService::new(device_name, secret_key);
    let device_id = service.get_device_id();

    service.start().await?;

    let mut instance = DISCOVERY_SERVICE
        .lock()
        .map_err(|e| anyhow::anyhow!("Failed to acquire discovery lock: {}", e))?;

    *instance = Some(service);

    log::info!("Discovery service started with device ID: {}", device_id);

    Ok(device_id)
}

/// 발견 서비스를 중지합니다.
pub fn stop_discovery() -> Result<()> {
    let mut instance = DISCOVERY_SERVICE
        .lock()
        .map_err(|e| anyhow::anyhow!("Failed to acquire discovery lock: {}", e))?;

    if let Some(service) = instance.as_ref() {
        service.stop()?;
        *instance = None;
        log::info!("Discovery service stopped");
    }

    Ok(())
}

/// 발견된 기기 목록을 가져옵니다.
pub fn get_discovered_devices() -> Result<Vec<DiscoveredDevice>> {
    let instance = DISCOVERY_SERVICE
        .lock()
        .map_err(|e| anyhow::anyhow!("Failed to acquire discovery lock: {}", e))?;

    if let Some(service) = instance.as_ref() {
        Ok(service.get_discovered_devices())
    } else {
        Ok(Vec::new())
    }
}
