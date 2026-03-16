// 파일 전송 관련 명령어
use libp2p::PeerId;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::State;

use crate::error::AppError;
use crate::network::{
    CancelTransferMsg, FileListRequestMsg, FileTransferRequestMsg, FileTransferStreamRequestMsg,
    P2pCommand, PairingApprovalMsg, PairingRequestMsg,
};
use crate::state::P2pState;

/// PeerId 파싱 헬퍼
fn parse_peer_id(peer_id: &str) -> Result<PeerId, AppError> {
    peer_id
        .parse()
        .map_err(|e| AppError::invalid_peer_id(format!("{}", e)))
}

/// P2P 실행 상태 확인 헬퍼
fn ensure_running(state: &P2pState) -> Result<(), AppError> {
    if !state.is_running.load(Ordering::SeqCst) {
        Err(AppError::p2p_not_running())
    } else {
        Ok(())
    }
}

/// 피어 연결
#[tauri::command]
pub async fn connect_to_peer(
    addr: String,
    state: State<'_, Arc<P2pState>>,
) -> Result<String, AppError> {
    ensure_running(&state)?;

    state
        .send_command(P2pCommand::Dial(addr.clone()))
        .await
        .map_err(|e| AppError::channel_error(e))?;

    Ok(format!("연결 요청 전송: {}", addr))
}

/// 파일 목록 요청
#[tauri::command]
pub async fn request_file_list(
    peer_id: String,
    path: String,
    state: State<'_, Arc<P2pState>>,
) -> Result<String, AppError> {
    ensure_running(&state)?;
    let peer_id = parse_peer_id(&peer_id)?;

    state
        .send_command(P2pCommand::RequestFileList(FileListRequestMsg {
            peer_id,
            path: path.clone(),
        }))
        .await
        .map_err(|e| AppError::channel_error(e))?;

    Ok(format!("파일 목록 요청 전송: {}", path))
}

/// 파일 다운로드
#[tauri::command]
pub async fn download_file(
    peer_id: String,
    path: String,
    state: State<'_, Arc<P2pState>>,
) -> Result<String, AppError> {
    ensure_running(&state)?;
    let peer_id = parse_peer_id(&peer_id)?;

    state
        .send_command(P2pCommand::RequestFileTransfer(FileTransferRequestMsg {
            peer_id,
            path: path.clone(),
        }))
        .await
        .map_err(|e| AppError::channel_error(e))?;

    Ok(format!("파일 다운로드 요청 전송: {}", path))
}

/// 파일 업로드
#[tauri::command]
pub async fn upload_file(
    peer_id: String,
    file_path: String,
    remote_path: String,
    state: State<'_, Arc<P2pState>>,
) -> Result<String, AppError> {
    ensure_running(&state)?;
    let peer_id = parse_peer_id(&peer_id)?;

    let file_path_buf = PathBuf::from(&file_path);
    if !file_path_buf.exists() {
        return Err(AppError::file_not_found(&file_path));
    }

    let file_name = file_path_buf
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .ok_or_else(|| AppError::io_error("파일 이름을 가져올 수 없습니다."))?;

    state
        .send_command(P2pCommand::RequestFileStream(
            FileTransferStreamRequestMsg {
                peer_id,
                file_path: file_path_buf,
                remote_path: remote_path.clone(),
            },
        ))
        .await
        .map_err(|e| AppError::channel_error(e))?;

    Ok(format!(
        "파일 업로드 요청 전송: {} -> {}",
        file_name, remote_path
    ))
}

/// 페어링 요청
#[tauri::command]
pub async fn request_pairing(
    peer_id: String,
    state: State<'_, Arc<P2pState>>,
) -> Result<String, AppError> {
    ensure_running(&state)?;
    let peer_id = parse_peer_id(&peer_id)?;

    state
        .send_command(P2pCommand::RequestPairing(PairingRequestMsg { peer_id }))
        .await
        .map_err(|e| AppError::channel_error(e))?;

    Ok("페어링 요청을 전송했습니다.".to_string())
}

/// 페어링 응답
#[tauri::command]
pub async fn respond_pairing(
    peer_id: String,
    approved: bool,
    state: State<'_, Arc<P2pState>>,
) -> Result<String, AppError> {
    ensure_running(&state)?;
    let peer_id = parse_peer_id(&peer_id)?;

    state
        .send_command(P2pCommand::ApprovePairing(PairingApprovalMsg {
            peer_id,
            approved,
        }))
        .await
        .map_err(|e| AppError::channel_error(e))?;

    Ok(format!(
        "페어링 {}했습니다.",
        if approved { "승인" } else { "거절" }
    ))
}

/// 전송 취소
#[tauri::command]
pub async fn cancel_transfer(
    transfer_id: String,
    state: State<'_, Arc<P2pState>>,
) -> Result<String, AppError> {
    ensure_running(&state)?;

    state
        .send_command(P2pCommand::CancelTransfer(CancelTransferMsg {
            transfer_id: transfer_id.clone(),
        }))
        .await
        .map_err(|e| AppError::channel_error(e))?;

    Ok(format!("전송 취소 요청 전송: {}", transfer_id))
}
