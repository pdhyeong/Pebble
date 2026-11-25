import { invoke } from "@tauri-apps/api/core";
import type { ActivityRecord, LocalFileInfo, SharedFolderStats } from "../../core/contract";
import { DEFAULT_SHOW_FOLDER_NAME, type AppSettings } from "../../core/settings";

export function startP2p(): Promise<null> {
  return invoke<null>("start_p2p");
}

export function stopP2p(): Promise<null> {
  return invoke<null>("stop_p2p");
}

export function getP2pStatus(): Promise<boolean> {
  return invoke<boolean>("get_p2p_status");
}

export function connectToPeer(addr: string): Promise<null> {
  return invoke<null>("connect_to_peer", { addr });
}

export function requestFileList(peerId: string, path: string): Promise<null> {
  return invoke<null>("request_file_list", { peerId, path });
}

export function downloadFile(peerId: string, path: string): Promise<string> {
  return invoke<string>("download_file", { peerId, path });
}

export function uploadFile(peerId: string, filePath: string, remotePath: string): Promise<string> {
  return invoke<string>("upload_file", { peerId, filePath, remotePath });
}

export function cancelTransfer(transferId: string): Promise<null> {
  return invoke<null>("cancel_transfer", { transferId });
}

export function requestPairing(peerId: string): Promise<string> {
  return invoke<string>("request_pairing", { peerId });
}

export function respondPairing(peerId: string, approved: boolean): Promise<null> {
  return invoke<null>("respond_pairing", { peerId, approved });
}

export function setDeviceName(name: string): Promise<null> {
  return invoke<null>("set_device_name", { name });
}

export function getDeviceName(): Promise<string> {
  return invoke<string>("get_device_name");
}

export function setSharedFolder(path: string): Promise<null> {
  return invoke<null>("set_shared_folder", { path });
}

export function getSharedFolder(): Promise<string> {
  return invoke<string>("get_shared_folder");
}

export function setSharedFolderDisplayName(name: string): Promise<null> {
  return invoke<null>("set_shared_folder_display_name", { name });
}

export function getSharedFolderDisplayName(): Promise<string> {
  return invoke<string>("get_shared_folder_display_name");
}

export function setShowFolderName(show: boolean): Promise<null> {
  return invoke<null>("set_show_folder_name", { show });
}

export function getLocalSharedFiles(relativePath: string): Promise<LocalFileInfo[]> {
  return invoke<LocalFileInfo[]>("get_local_shared_files", { relativePath });
}

export function getLocalIp(): Promise<string> {
  return invoke<string>("get_local_ip");
}

export function getActivityHistory(): Promise<ActivityRecord[]> {
  return invoke<ActivityRecord[]>("get_activity_history");
}

export function getSharedFolderStats(): Promise<SharedFolderStats> {
  return invoke<SharedFolderStats>("get_shared_folder_stats");
}

export async function applySettingsToBackend(settings: AppSettings): Promise<void> {
  await setDeviceName(settings.deviceName);
  await setSharedFolderDisplayName(settings.sharedFolderDisplayName);
  await setShowFolderName(DEFAULT_SHOW_FOLDER_NAME);
  await setSharedFolder(settings.sharedFolderPath);
}
