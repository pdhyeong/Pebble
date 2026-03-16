// 네트워크 관련 타입 정의
/// 네트워크 관련 메시지 타입 정의
use libp2p::PeerId;
use std::fs::File;
use std::path::PathBuf;

/// 파일 목록 요청 메시지
#[derive(Debug, Clone)]
pub struct FileListRequestMsg {
    pub peer_id: PeerId,
    pub path: String,
}

/// 파일 전송 요청 메시지 (기존 방식)
pub struct FileTransferRequestMsg {
    pub peer_id: PeerId,
    pub path: String,
}

/// 파일 스트림 전송 요청 메시지
pub struct FileTransferStreamRequestMsg {
    pub peer_id: PeerId,
    pub file_path: PathBuf,
    pub remote_path: String,
}

/// 기기 정보 메시지
#[allow(dead_code)]
pub struct DeviceInfoMsg {
    pub peer_id: PeerId,
}

/// 페어링 요청 메시지
pub struct PairingRequestMsg {
    pub peer_id: PeerId,
}

/// 페어링 응답 메시지
pub struct PairingApprovalMsg {
    pub peer_id: PeerId,
    pub approved: bool,
}

/// 전송 취소 메시지
pub struct CancelTransferMsg {
    pub transfer_id: String,
}

/// 발신 전송 상태
pub struct OutgoingTransferState {
    pub file: File,
    pub file_name: String,
    pub total_size: u64,
    pub total_chunks: u32,
    pub current_chunk: u32,
    pub peer_id: PeerId,
    pub transfer_id: String,
    pub start_time: std::time::Instant,
}

/// 수신 전송 상태
pub struct IncomingTransferState {
    pub file: File,
    pub file_name: String,
    pub saved_path: PathBuf,
    pub total_chunks: u32,
    pub received_chunks: u32,
    pub peer_id: PeerId,
    pub start_time: std::time::Instant,
    pub total_size: u64,
}

/// 청크 크기 (1MB)
pub const CHUNK_SIZE: usize = 1024 * 1024;

/// 통합 P2P 커맨드 enum
/// 프론트엔드 → 엔진으로의 모든 명령을 단일 채널로 전달
pub enum P2pCommand {
    /// 피어에 연결 (Multiaddr 문자열)
    Dial(String),
    /// 원격 피어의 파일 목록 요청
    RequestFileList(FileListRequestMsg),
    /// 원격 피어에서 파일 다운로드
    RequestFileTransfer(FileTransferRequestMsg),
    /// 원격 피어에 파일 업로드 (청크 스트림)
    RequestFileStream(FileTransferStreamRequestMsg),
    /// 공유 폴더 경로 업데이트
    UpdateSharedFolder(PathBuf),
    /// 원격 피어에 페어링 요청
    RequestPairing(PairingRequestMsg),
    /// 페어링 승인/거절
    ApprovePairing(PairingApprovalMsg),
    /// 연결된 모든 피어에게 디렉토리 변경 신호 브로드캐스트
    NotifyDirectoryChanged,
    /// 진행 중인 전송 취소
    CancelTransfer(CancelTransferMsg),
    /// 엔진 종료
    Shutdown,
}
