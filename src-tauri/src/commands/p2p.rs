// P2P 엔진 관련 명령어
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::State;
use tokio::sync::mpsc;

use crate::error::AppError;
use crate::network::P2pCommand;
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

    // 통합 커맨드 채널 생성 (9개 → 1개)
    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel::<P2pCommand>();

    // 현재 공유 폴더 경로와 기기 이름 가져오기
    let shared_folder = state.shared_folder_path.read().await.clone();
    let device_name = state.device_name.read().await.clone();

    // sender를 state에 저장
    *state.command_sender.lock().await = Some(cmd_tx);

    let state_clone = state.inner().clone();
    let state_for_engine = state.inner().clone();

    tauri::async_runtime::spawn(async move {
        match crate::network::run_p2p_engine(
            app_handle,
            cmd_rx,
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
        *state_clone.command_sender.lock().await = None;
    });

    Ok("P2P 엔진이 시작되었습니다.".to_string())
}

/// P2P 엔진 종료
#[tauri::command]
pub async fn stop_p2p(state: State<'_, Arc<P2pState>>) -> Result<String, AppError> {
    if !state.is_running.load(Ordering::SeqCst) {
        return Err(AppError::p2p_not_running());
    }

    state
        .send_command(P2pCommand::Shutdown)
        .await
        .map_err(|e| AppError::channel_error(format!("종료 신호 전송 실패: {}", e)))?;

    Ok("P2P 엔진 종료 요청을 보냈습니다.".to_string())
}

/// P2P 상태 확인
#[tauri::command]
pub fn get_p2p_status(state: State<'_, Arc<P2pState>>) -> bool {
    state.is_running.load(Ordering::SeqCst)
}
