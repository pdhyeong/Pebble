// P2P 엔진 관련 명령어
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::State;
use tokio::sync::mpsc;

use crate::error::AppError;
use crate::network::{
    self, CancelTransferMsg, FileListRequestMsg, FileTransferRequestMsg,
    FileTransferStreamRequestMsg, PairingApprovalMsg, PairingRequestMsg,
};
use crate::state::P2pState;

/// P2P 엔진 시작
#[tauri::command]
pub async fn start_p2p(
    app_handle: tauri::AppHandle,
    state: State<'_, Arc<P2pState>>,
) -> Result<String, AppError> {
    if state.is_running.load(Ordering::SeqCst) {
        return Err(AppError::p2p_already_running());
    }

    state.is_running.store(true, Ordering::SeqCst);

    // 채널 생성
    let (dial_tx, dial_rx) = mpsc::unbounded_channel::<String>();
    let (file_list_tx, file_list_rx) = mpsc::unbounded_channel::<FileListRequestMsg>();
    let (file_transfer_tx, file_transfer_rx) = mpsc::unbounded_channel::<FileTransferRequestMsg>();
    let (file_stream_tx, file_stream_rx) =
        mpsc::unbounded_channel::<FileTransferStreamRequestMsg>();
    let (shared_folder_tx, shared_folder_rx) = mpsc::unbounded_channel::<PathBuf>();
    let (pairing_request_tx, pairing_request_rx) = mpsc::unbounded_channel::<PairingRequestMsg>();
    let (pairing_approval_tx, pairing_approval_rx) =
        mpsc::unbounded_channel::<PairingApprovalMsg>();
    let (cancel_transfer_tx, cancel_transfer_rx) = mpsc::unbounded_channel::<CancelTransferMsg>();
    let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>(1);

    // 현재 공유 폴더 경로와 기기 이름 가져오기
    let shared_folder = state.shared_folder_path.read().await.clone();
    let device_name = state.device_name.read().await.clone();

    // sender들을 state에 저장
    *state.dial_sender.lock().await = Some(dial_tx);
    *state.file_list_sender.lock().await = Some(file_list_tx);
    *state.file_transfer_sender.lock().await = Some(file_transfer_tx);
    *state.file_stream_sender.lock().await = Some(file_stream_tx);
    *state.shared_folder_sender.lock().await = Some(shared_folder_tx);
    *state.pairing_request_sender.lock().await = Some(pairing_request_tx);
    *state.pairing_approval_sender.lock().await = Some(pairing_approval_tx);
    *state.cancel_transfer_sender.lock().await = Some(cancel_transfer_tx);
    *state.shutdown_sender.lock().await = Some(shutdown_tx);

    let state_clone = state.inner().clone();
    let state_for_engine = state.inner().clone();

    tauri::async_runtime::spawn(async move {
        match network::run_p2p_engine(
            app_handle,
            dial_rx,
            file_list_rx,
            file_transfer_rx,
            file_stream_rx,
            shared_folder_rx,
            pairing_request_rx,
            pairing_approval_rx,
            cancel_transfer_rx,
            shutdown_rx,
            shared_folder,
            device_name,
            state_for_engine,
        )
        .await
        {
            Ok(_) => println!("P2P 엔진 종료"),
            Err(e) => println!("P2P 엔진 에러: {}", e),
        }

        // 엔진 종료 시 정리
        state_clone.is_running.store(false, Ordering::SeqCst);
        *state_clone.dial_sender.lock().await = None;
        *state_clone.file_list_sender.lock().await = None;
        *state_clone.file_transfer_sender.lock().await = None;
        *state_clone.file_stream_sender.lock().await = None;
        *state_clone.shared_folder_sender.lock().await = None;
        *state_clone.pairing_request_sender.lock().await = None;
        *state_clone.pairing_approval_sender.lock().await = None;
        *state_clone.shutdown_sender.lock().await = None;
    });

    Ok("P2P 엔진이 시작되었습니다.".to_string())
}

/// P2P 엔진 종료
#[tauri::command]
pub async fn stop_p2p(state: State<'_, Arc<P2pState>>) -> Result<String, AppError> {
    if !state.is_running.load(Ordering::SeqCst) {
        return Err(AppError::p2p_not_running());
    }

    let sender = state.shutdown_sender.lock().await;
    if let Some(tx) = sender.as_ref() {
        tx.send(())
            .await
            .map_err(|e| AppError::channel_error(format!("종료 신호 전송 실패: {}", e)))?;
        Ok("P2P 엔진 종료 요청을 보냈습니다.".to_string())
    } else {
        Err(AppError::channel_error("종료 채널이 없습니다."))
    }
}

/// P2P 상태 확인
#[tauri::command]
pub fn get_p2p_status(state: State<'_, Arc<P2pState>>) -> bool {
    state.is_running.load(Ordering::SeqCst)
}
