// 파일 전송 관련 명령어
use libp2p::PeerId;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::State;

use crate::error::AppError;
use crate::network::{
    FileListRequestMsg, FileTransferRequestMsg, FileTransferStreamRequestMsg, PairingApprovalMsg,
    PairingRequestMsg,
};
use crate::state::P2pState;

/// 피어 연결
#[tauri::command]
pub async fn connect_to_peer(
    addr: String,
    state: State<'_, Arc<P2pState>>,
) -> Result<String, AppError> {
    if !state.is_running.load(Ordering::SeqCst) {
        return Err(AppError::p2p_not_running());
    }

    let sender = state.dial_sender.lock().await;
    if let Some(tx) = sender.as_ref() {
        tx.send(addr.clone())
            .map_err(|e| AppError::channel_error(format!("연결 요청 전송 실패: {}", e)))?;
        Ok(format!("연결 요청 전송: {}", addr))
    } else {
        Err(AppError::channel_error("P2P 엔진 채널이 없습니다."))
    }
}

/// 파일 목록 요청
#[tauri::command]
pub async fn request_file_list(
    peer_id: String,
    path: String,
    state: State<'_, Arc<P2pState>>,
) -> Result<String, AppError> {
    if !state.is_running.load(Ordering::SeqCst) {
        return Err(AppError::p2p_not_running());
    }

    let peer_id: PeerId = peer_id
        .parse()
        .map_err(|e| AppError::invalid_peer_id(format!("{}", e)))?;

    let sender = state.file_list_sender.lock().await;
    if let Some(tx) = sender.as_ref() {
        tx.send(FileListRequestMsg {
            peer_id,
            path: path.clone(),
        })
        .map_err(|e| AppError::channel_error(format!("파일 목록 요청 전송 실패: {}", e)))?;
        Ok(format!("파일 목록 요청 전송: {}", path))
    } else {
        Err(AppError::channel_error("P2P 엔진 채널이 없습니다."))
    }
}

/// 파일 다운로드
#[tauri::command]
pub async fn download_file(
    peer_id: String,
    path: String,
    state: State<'_, Arc<P2pState>>,
) -> Result<String, AppError> {
    if !state.is_running.load(Ordering::SeqCst) {
        return Err(AppError::p2p_not_running());
    }

    let peer_id: PeerId = peer_id
        .parse()
        .map_err(|e| AppError::invalid_peer_id(format!("{}", e)))?;

    let sender = state.file_transfer_sender.lock().await;
    if let Some(tx) = sender.as_ref() {
        tx.send(FileTransferRequestMsg {
            peer_id,
            path: path.clone(),
        })
        .map_err(|e| AppError::channel_error(format!("파일 다운로드 요청 전송 실패: {}", e)))?;
        Ok(format!("파일 다운로드 요청 전송: {}", path))
    } else {
        Err(AppError::channel_error("P2P 엔진 채널이 없습니다."))
    }
}

/// 파일 업로드
#[tauri::command]
pub async fn upload_file(
    peer_id: String,
    file_path: String,
    remote_path: String,
    state: State<'_, Arc<P2pState>>,
) -> Result<String, AppError> {
    if !state.is_running.load(Ordering::SeqCst) {
        return Err(AppError::p2p_not_running());
    }

    let peer_id: PeerId = peer_id
        .parse()
        .map_err(|e| AppError::invalid_peer_id(format!("{}", e)))?;

    let file_path_buf = PathBuf::from(&file_path);
    if !file_path_buf.exists() {
        return Err(AppError::file_not_found(&file_path));
    }

    let file_name = file_path_buf
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .ok_or_else(|| AppError::io_error("파일 이름을 가져올 수 없습니다."))?;

    let sender = state.file_stream_sender.lock().await;
    if let Some(tx) = sender.as_ref() {
        tx.send(FileTransferStreamRequestMsg {
            peer_id,
            file_path: file_path_buf,
            remote_path: remote_path.clone(),
        })
        .map_err(|e| AppError::channel_error(format!("파일 업로드 요청 전송 실패: {}", e)))?;
        Ok(format!(
            "파일 업로드 요청 전송: {} -> {}",
            file_name, remote_path
        ))
    } else {
        Err(AppError::channel_error("P2P 엔진 채널이 없습니다."))
    }
}

/// 페어링 요청
#[tauri::command]
pub async fn request_pairing(
    peer_id: String,
    state: State<'_, Arc<P2pState>>,
) -> Result<String, AppError> {
    if !state.is_running.load(Ordering::SeqCst) {
        return Err(AppError::p2p_not_running());
    }

    let peer_id: PeerId = peer_id
        .parse()
        .map_err(|e| AppError::invalid_peer_id(format!("{}", e)))?;

    let sender = state.pairing_request_sender.lock().await;
    if let Some(tx) = sender.as_ref() {
        tx.send(PairingRequestMsg { peer_id })
            .map_err(|e| AppError::channel_error(format!("페어링 요청 전송 실패: {}", e)))?;
        Ok("페어링 요청을 전송했습니다.".to_string())
    } else {
        Err(AppError::channel_error("P2P 엔진 채널이 없습니다."))
    }
}

/// 페어링 응답
#[tauri::command]
pub async fn respond_pairing(
    peer_id: String,
    approved: bool,
    state: State<'_, Arc<P2pState>>,
) -> Result<String, AppError> {
    if !state.is_running.load(Ordering::SeqCst) {
        return Err(AppError::p2p_not_running());
    }

    let peer_id: PeerId = peer_id
        .parse()
        .map_err(|e| AppError::invalid_peer_id(format!("{}", e)))?;

    let sender = state.pairing_approval_sender.lock().await;
    if let Some(tx) = sender.as_ref() {
        tx.send(PairingApprovalMsg { peer_id, approved })
            .map_err(|e| AppError::channel_error(format!("페어링 응답 전송 실패: {}", e)))?;
        Ok(format!(
            "페어링 {}했습니다.",
            if approved { "승인" } else { "거절" }
        ))
    } else {
        Err(AppError::channel_error("P2P 엔진 채널이 없습니다."))
    }
}

/// 전송 취소
#[tauri::command]
pub async fn cancel_transfer(
    transfer_id: String,
    state: State<'_, Arc<P2pState>>,
) -> Result<String, AppError> {
    if !state.is_running.load(Ordering::SeqCst) {
        return Err(AppError::p2p_not_running());
    }

    let sender = state.cancel_transfer_sender.lock().await;
    if let Some(tx) = sender.as_ref() {
        tx.send(crate::network::CancelTransferMsg {
            transfer_id: transfer_id.clone(),
        })
        .map_err(|e| AppError::channel_error(format!("전송 취소 요청 전송 실패: {}", e)))?;
        Ok(format!("전송 취소 요청 전송: {}", transfer_id))
    } else {
        Err(AppError::channel_error("P2P 엔진 채널이 없습니다."))
    }
}
