use serde::Serialize;

use crate::domain::files::FileEntry;
use crate::domain::peer::PeerId;
use crate::domain::transfer::{CancelReason, Direction, TransferId};

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", content = "payload", rename_all = "kebab-case")]
pub enum UiEvent {
    P2pStopped {
        message: String,
    },
    ListeningOn {
        address: String,
    },
    PeerFound {
        peer_id: PeerId,
        address: String,
    },
    PeerExpired {
        peer_id: PeerId,
    },
    PeerInfo {
        peer_id: PeerId,
        agent_version: String,
    },
    PingSuccess {
        peer_id: PeerId,
        rtt_ms: u64,
    },
    DialStarted {
        address: String,
    },
    DialFailed {
        address: String,
        error: String,
    },
    ConnectionSuccess {
        peer_id: PeerId,
    },
    ConnectionClosed {
        peer_id: PeerId,
    },
    DeviceInfoReceived {
        peer_id: PeerId,
        device_name: String,
    },
    FileListRequestSent {
        peer_id: PeerId,
        path: String,
    },
    RemoteFiles {
        peer_id: PeerId,
        path: String,
        files: Vec<FileEntry>,
        error: Option<String>,
    },
    FileListError {
        peer_id: PeerId,
        error: String,
    },
    RemoteDirectoryChanged {
        peer_id: PeerId,
    },
    LocalFilesChanged {
        path: String,
        kind: &'static str,
    },
    SharedFolderChanged {
        path: String,
    },
    TransferStarted {
        transfer_id: TransferId,
        direction: Direction,
        peer_id: PeerId,
        file_name: String,
        total_size: u64,
    },
    TransferProgress {
        transfer_id: TransferId,
        direction: Direction,
        bytes_done: u64,
        total_size: u64,
        progress: f64,
    },
    TransferCompleted {
        transfer_id: TransferId,
        direction: Direction,
        peer_id: PeerId,
        file_name: String,
        total_size: u64,
        saved_path: Option<String>,
        speed_bps: Option<f64>,
    },
    TransferFailed {
        transfer_id: TransferId,
        direction: Direction,
        peer_id: PeerId,
        file_name: String,
        error: String,
    },
    TransferCancelled {
        transfer_id: TransferId,
        direction: Direction,
        peer_id: PeerId,
        file_name: String,
        reason: CancelReason,
    },
    PairingRequest {
        peer_id: PeerId,
        device_name: String,
        pin: String,
    },
    PairingSent {
        peer_id: PeerId,
        pin: String,
    },
    PairingApproved {
        peer_id: PeerId,
        device_name: String,
        shared_folder_name: Option<String>,
    },
    PairingRejected {
        peer_id: PeerId,
        error: Option<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peer() -> PeerId {
        PeerId::parse("12D3KooW").unwrap()
    }

    fn event_name(event: &UiEvent) -> String {
        serde_json::to_value(event).unwrap()["event"]
            .as_str()
            .unwrap()
            .to_string()
    }

    #[test]
    fn event_names_match_contract() {
        let id = TransferId::from_wire("t".into());
        let cases = [
            (UiEvent::P2pStopped { message: String::new() }, "p2p-stopped"),
            (UiEvent::ListeningOn { address: String::new() }, "listening-on"),
            (UiEvent::PeerFound { peer_id: peer(), address: String::new() }, "peer-found"),
            (UiEvent::PingSuccess { peer_id: peer(), rtt_ms: 1 }, "ping-success"),
            (UiEvent::ConnectionSuccess { peer_id: peer() }, "connection-success"),
            (UiEvent::DeviceInfoReceived { peer_id: peer(), device_name: String::new() }, "device-info-received"),
            (UiEvent::FileListRequestSent { peer_id: peer(), path: String::new() }, "file-list-request-sent"),
            (UiEvent::RemoteFiles { peer_id: peer(), path: String::new(), files: vec![], error: None }, "remote-files"),
            (UiEvent::LocalFilesChanged { path: String::new(), kind: "modified" }, "local-files-changed"),
            (
                UiEvent::TransferProgress {
                    transfer_id: id.clone(),
                    direction: Direction::Upload,
                    bytes_done: 0,
                    total_size: 0,
                    progress: 0.0,
                },
                "transfer-progress",
            ),
            (
                UiEvent::TransferCancelled {
                    transfer_id: id,
                    direction: Direction::Download,
                    peer_id: peer(),
                    file_name: String::new(),
                    reason: CancelReason::UserCancelled,
                },
                "transfer-cancelled",
            ),
            (UiEvent::PairingApproved { peer_id: peer(), device_name: String::new(), shared_folder_name: None }, "pairing-approved"),
        ];
        for (event, name) in cases {
            assert_eq!(event_name(&event), name);
        }
    }

    #[test]
    fn payload_is_an_object_with_snake_case_fields() {
        let event = UiEvent::PingSuccess { peer_id: peer(), rtt_ms: 12 };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["payload"]["peer_id"], "12D3KooW");
        assert_eq!(json["payload"]["rtt_ms"], 12);
    }
}
