// P2P Tauri 명령어 래퍼 함수들
import { invoke } from "@tauri-apps/api/core";
import type { LocalFileInfo } from "./types";

/** P2P 엔진 시작 */
export async function startP2pEngine(): Promise<string> {
    return invoke<string>("start_p2p");
}

/** P2P 엔진 종료 */
export async function stopP2pEngine(): Promise<string> {
    return invoke<string>("stop_p2p");
}

/** P2P 상태 확인 */
export async function getP2pStatus(): Promise<boolean> {
    return invoke<boolean>("get_p2p_status");
}

/** 피어에 연결 */
export async function connectToPeer(addr: string): Promise<string> {
    return invoke<string>("connect_to_peer", { addr });
}

/** 파일 목록 요청 */
export async function requestFileList(peerId: string, path: string): Promise<string> {
    return invoke<string>("request_file_list", { peerId, path });
}

/** 파일 다운로드 */
export async function downloadFile(peerId: string, path: string): Promise<string> {
    return invoke<string>("download_file", { peerId, path });
}

/** 파일 업로드 */
export async function uploadFile(
    peerId: string,
    filePath: string,
    remotePath: string
): Promise<string> {
    return invoke<string>("upload_file", { peerId, filePath, remotePath });
}

/** 페어링 요청 */
export async function requestPairing(peerId: string): Promise<string> {
    return invoke<string>("request_pairing", { peerId });
}

/** 페어링 응답 */
export async function respondPairing(peerId: string, approved: boolean): Promise<string> {
    return invoke<string>("respond_pairing", { peerId, approved });
}

/** 기기 이름 설정 */
export async function setDeviceName(name: string): Promise<string> {
    return invoke<string>("set_device_name", { name });
}

/** 기기 이름 조회 */
export async function getDeviceName(): Promise<string> {
    return invoke<string>("get_device_name");
}

/** 공유 폴더 설정 */
export async function setSharedFolder(path: string): Promise<string> {
    return invoke<string>("set_shared_folder", { path });
}

/** 공유 폴더 조회 */
export async function getSharedFolder(): Promise<string> {
    return invoke<string>("get_shared_folder");
}

/** 로컬 파일 목록 조회 */
export async function getLocalSharedFiles(relativePath: string): Promise<LocalFileInfo[]> {
    return invoke<LocalFileInfo[]>("get_local_shared_files", { relativePath });
}

/** 공유 폴더 표시 이름 설정 */
export async function setSharedFolderDisplayName(name: string): Promise<string> {
    return invoke<string>("set_shared_folder_display_name", { name });
}

/** 공유 폴더 표시 이름 조회 */
export async function getSharedFolderDisplayName(): Promise<string> {
    return invoke<string>("get_shared_folder_display_name");
}

/** 폴더 이름 공개 여부 설정 */
export async function setShowFolderName(show: boolean): Promise<string> {
    return invoke<string>("set_show_folder_name", { show });
}

/** 폴더 이름 공개 여부 조회 */
export async function getShowFolderName(): Promise<boolean> {
    return invoke<boolean>("get_show_folder_name");
}

export async function cancelTransfer(transferId: string): Promise<string> {
  return await invoke<string>('cancel_transfer', { transferId });
}
