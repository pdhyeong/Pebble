use std::io::Write;
use tokio::sync::broadcast::{error::RecvError, Receiver};

use pebble_core::application::events::UiEvent;
use pebble_core::domain::transfer::Direction;

pub async fn print_events(mut events: Receiver<UiEvent>) {
    loop {
        let line = match events.recv().await {
            Ok(event) => describe(&event),
            Err(RecvError::Lagged(missed)) => Some(format!("로그 {missed}줄을 건너뛰었습니다")),
            Err(RecvError::Closed) => return,
        };
        if let Some(line) = line {
            let line = printable(&line);
            let written = writeln!(std::io::stdout().lock(), "[{}] {line}", utc_clock());
            if written.is_err() {
                return;
            }
        }
    }
}

pub fn printable(raw: &str) -> String {
    raw.chars()
        .map(|c| if c.is_control() { '?' } else { c })
        .collect()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_characters_cannot_forge_lines_or_drive_the_terminal() {
        assert_eq!(
            printable("a\n[00:00:00Z] 페어링됨\u{1b}[2J"),
            "a?[00:00:00Z] 페어링됨?[2J"
        );
        assert_eq!(printable("보통 이름"), "보통 이름");
    }

    fn peer() -> pebble_core::domain::peer::PeerId {
        pebble_core::domain::peer::PeerId::parse("12D3KooW").unwrap()
    }

    fn tid() -> pebble_core::domain::transfer::TransferId {
        pebble_core::domain::transfer::TransferId::from_wire("t1".into())
    }

    #[test]
    fn describes_connection_and_pairing_events() {
        use pebble_core::application::events::UiEvent as E;
        let p = peer();
        let cases = [
            (E::ListeningOn { address: "/a".into() }, "대기 주소 /a".to_string()),
            (
                E::PeerFound { peer_id: p.clone(), address: "/a".into() },
                "기기 발견 12D3KooW (/a)".to_string(),
            ),
            (E::ConnectionSuccess { peer_id: p.clone() }, "연결됨 12D3KooW".to_string()),
            (E::ConnectionClosed { peer_id: p.clone() }, "연결 끊김 12D3KooW".to_string()),
            (
                E::DeviceInfoReceived { peer_id: p.clone(), device_name: "n".into() },
                "기기 이름 n = 12D3KooW".to_string(),
            ),
            (
                E::PairingRequest { peer_id: p.clone(), device_name: "n".into(), pin: "123".into() },
                "페어링 요청: n (12D3KooW) PIN 123 — 상대 화면의 PIN 과 같으면 `pebble-cli accept n`"
                    .to_string(),
            ),
            (
                E::PairingApproved { peer_id: p.clone(), device_name: "n".into(), shared_folder_name: None },
                "페어링됨 n (12D3KooW)".to_string(),
            ),
            (
                E::PairingRejected { peer_id: p.clone(), error: Some("bad".into()) },
                "페어링 거절 12D3KooW: bad".to_string(),
            ),
            (
                E::PairingRejected { peer_id: p.clone(), error: None },
                "페어링 거절 12D3KooW: -".to_string(),
            ),
            (E::P2pStopped { message: "멈춤".into() }, "멈춤".to_string()),
        ];
        for (event, expected) in cases {
            assert_eq!(describe(&event), Some(expected));
        }
        assert_eq!(describe(&E::PeerExpired { peer_id: p }), None);
    }

    #[test]
    fn describes_transfer_events() {
        use pebble_core::application::events::UiEvent as E;
        use pebble_core::domain::transfer::CancelReason;
        let started = |direction, total_size| E::TransferStarted {
            transfer_id: tid(),
            direction,
            peer_id: peer(),
            file_name: "f.txt".into(),
            total_size,
        };
        assert_eq!(
            describe(&started(Direction::Upload, 5)),
            Some("전송 시작 [t1] 보냄 f.txt (5 B)".to_string())
        );
        assert_eq!(
            describe(&started(Direction::Download, 1)),
            Some("전송 시작 [t1] 받음 f.txt (1 B)".to_string())
        );
        assert_eq!(describe(&started(Direction::Upload, 0)), None);
        let completed = |saved_path| E::TransferCompleted {
            transfer_id: tid(),
            direction: Direction::Download,
            peer_id: peer(),
            file_name: "f.txt".into(),
            total_size: 3,
            saved_path,
            speed_bps: None,
        };
        assert_eq!(
            describe(&completed(Some("/d/f.txt".into()))),
            Some("전송 완료 [t1] f.txt → /d/f.txt".to_string())
        );
        assert_eq!(
            describe(&completed(None)),
            Some("전송 완료 [t1] f.txt".to_string())
        );
        assert_eq!(
            describe(&E::TransferFailed {
                transfer_id: tid(),
                direction: Direction::Upload,
                peer_id: peer(),
                file_name: "f.txt".into(),
                error: "boom".into(),
            }),
            Some("전송 실패 [t1] f.txt: boom".to_string())
        );
        assert_eq!(
            describe(&E::TransferCancelled {
                transfer_id: tid(),
                direction: Direction::Upload,
                peer_id: peer(),
                file_name: "f.txt".into(),
                reason: CancelReason::UserCancelled,
            }),
            Some("전송 취소 [t1] f.txt (UserCancelled)".to_string())
        );
    }

    #[test]
    fn utc_clock_is_a_valid_time_of_day() {
        let clock = utc_clock();
        assert_eq!(clock.len(), 9, "{clock}");
        assert!(clock.ends_with('Z'));
        let parts: Vec<u64> = clock[..8].split(':').map(|p| p.parse().unwrap()).collect();
        assert_eq!(parts.len(), 3);
        assert!(parts[0] < 24 && parts[1] < 60 && parts[2] < 60, "{clock}");
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            % 86_400;
        let now = parts[0] * 3600 + parts[1] * 60 + parts[2];
        assert!(now.abs_diff(secs) < 5 || now.abs_diff(secs) > 86_395);
    }

    #[tokio::test]
    async fn print_events_stops_when_channel_closes() {
        let (tx, rx) = tokio::sync::broadcast::channel(4);
        tx.send(pebble_core::application::events::UiEvent::ListeningOn {
            address: "/a".into(),
        })
        .unwrap();
        drop(tx);
        tokio::time::timeout(std::time::Duration::from_secs(5), print_events(rx))
            .await
            .unwrap();
    }
}
