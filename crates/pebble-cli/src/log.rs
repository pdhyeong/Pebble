use pebble_core::application::events::UiEvent;
use pebble_core::application::ports::UiEvents;
use pebble_core::domain::transfer::Direction;

pub struct LogPrinter;

impl UiEvents for LogPrinter {
    fn emit(&self, event: UiEvent) {
        if let Some(line) = describe(&event) {
            println!("[{}] {line}", utc_clock());
        }
    }
}

fn describe(event: &UiEvent) -> Option<String> {
    let line = match event {
        UiEvent::ListeningOn { address } => format!("대기 주소 {address}"),
        UiEvent::PeerFound { peer_id, address } => format!("기기 발견 {peer_id} ({address})"),
        UiEvent::ConnectionSuccess { peer_id } => format!("연결됨 {peer_id}"),
        UiEvent::ConnectionClosed { peer_id } => format!("연결 끊김 {peer_id}"),
        UiEvent::DeviceInfoReceived { peer_id, device_name } => format!("기기 이름 {device_name} = {peer_id}"),
        UiEvent::PairingRequest { peer_id, device_name, pin } => format!(
            "페어링 요청: {device_name} ({peer_id}) PIN {pin} — 상대 화면의 PIN 과 같으면 `pebble-cli accept {device_name}`"
        ),
        UiEvent::PairingApproved { peer_id, device_name, .. } => format!("페어링됨 {device_name} ({peer_id})"),
        UiEvent::PairingRejected { peer_id, error } => {
            format!("페어링 거절 {peer_id}: {}", error.as_deref().unwrap_or("-"))
        }
        UiEvent::TransferStarted { transfer_id, direction, file_name, total_size, .. } if *total_size > 0 => {
            format!("전송 시작 [{transfer_id}] {} {file_name} ({total_size} B)", arrow(*direction))
        }
        UiEvent::TransferCompleted { transfer_id, file_name, saved_path, .. } => format!(
            "전송 완료 [{transfer_id}] {file_name}{}",
            saved_path.as_deref().map(|p| format!(" → {p}")).unwrap_or_default()
        ),
        UiEvent::TransferFailed { transfer_id, file_name, error, .. } => {
            format!("전송 실패 [{transfer_id}] {file_name}: {error}")
        }
        UiEvent::TransferCancelled { transfer_id, file_name, reason, .. } => {
            format!("전송 취소 [{transfer_id}] {file_name} ({reason:?})")
        }
        UiEvent::P2pStopped { message } => message.clone(),
        _ => return None,
    };
    Some(line)
}

fn arrow(direction: Direction) -> &'static str {
    match direction {
        Direction::Upload => "보냄",
        Direction::Download => "받음",
    }
}

fn utc_clock() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default();
    let day = secs % 86_400;
    format!("{:02}:{:02}:{:02}Z", day / 3600, day % 3600 / 60, day % 60)
}
