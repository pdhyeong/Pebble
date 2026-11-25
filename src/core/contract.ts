export type ErrorCode =
  | "P2P_NOT_RUNNING"
  | "P2P_ALREADY_RUNNING"
  | "INVALID_PEER_ID"
  | "INVALID_PATH"
  | "ACCESS_DENIED"
  | "NOT_FOUND"
  | "IO"
  | "NETWORK"
  | "ENGINE_UNAVAILABLE"
  | "INTERNAL";

export interface AppError {
  code: ErrorCode;
  message: string;
}

export interface LocalFileInfo {
  name: string;
  path: string;
  is_dir: boolean;
  size: number;
}

export interface ActivityRecord {
  id: string;
  transfer_type: "upload" | "download";
  file_name: string;
  peer_id: string;
  device_name: string | null;
  size: number;
  timestamp: number;
  status: "completed" | "failed" | "cancelled";
  speed: number | null;
}

export interface SharedFolderStats {
  total_size: number;
  file_count: number;
}

export type Direction = "upload" | "download";

export interface RemoteFilesPayload {
  peer_id: string;
  path: string;
  files: LocalFileInfo[];
  error: string | null;
}

export interface TransferStartedPayload {
  transfer_id: string;
  direction: Direction;
  peer_id: string;
  file_name: string;
  total_size: number;
}

export interface TransferProgressPayload {
  transfer_id: string;
  direction: Direction;
  bytes_done: number;
  total_size: number;
  progress: number;
}

export interface TransferCompletedPayload {
  transfer_id: string;
  direction: Direction;
  peer_id: string;
  file_name: string;
  total_size: number;
  saved_path: string | null;
  speed_bps: number | null;
}

export interface TransferFailedPayload {
  transfer_id: string;
  direction: Direction;
  peer_id: string;
  file_name: string;
  error: string;
}

export interface TransferCancelledPayload {
  transfer_id: string;
  direction: Direction;
  peer_id: string;
  file_name: string;
  reason: "user_cancelled" | "peer_cancelled";
}

export interface EventPayloads {
  "p2p-stopped": { message: string };
  "listening-on": { address: string };
  "peer-found": { peer_id: string; address: string };
  "peer-expired": { peer_id: string };
  "peer-info": { peer_id: string; agent_version: string };
  "ping-success": { peer_id: string; rtt_ms: number };
  "dial-started": { address: string };
  "dial-failed": { address: string; error: string };
  "connection-success": { peer_id: string };
  "connection-closed": { peer_id: string };
  "device-info-received": { peer_id: string; device_name: string };
  "file-list-request-sent": { peer_id: string; path: string };
  "remote-files": RemoteFilesPayload;
  "file-list-error": { peer_id: string; error: string };
  "remote-directory-changed": { peer_id: string };
  "local-files-changed": { path: string; kind: "modified" };
  "shared-folder-changed": { path: string };
  "transfer-started": TransferStartedPayload;
  "transfer-progress": TransferProgressPayload;
  "transfer-completed": TransferCompletedPayload;
  "transfer-failed": TransferFailedPayload;
  "transfer-cancelled": TransferCancelledPayload;
  "pairing-request": { peer_id: string; device_name: string; pin: string };
  "pairing-sent": { peer_id: string; pin: string };
  "pairing-approved": { peer_id: string; device_name: string; shared_folder_name: string | null };
  "pairing-rejected": { peer_id: string; error: string | null };
}

export type EventName = keyof EventPayloads;

export type ContractEvent = {
  [K in EventName]: { name: K; payload: EventPayloads[K] };
}[EventName];

const EVENT_NAME_SET: Record<EventName, true> = {
  "p2p-stopped": true,
  "listening-on": true,
  "peer-found": true,
  "peer-expired": true,
  "peer-info": true,
  "ping-success": true,
  "dial-started": true,
  "dial-failed": true,
  "connection-success": true,
  "connection-closed": true,
  "device-info-received": true,
  "file-list-request-sent": true,
  "remote-files": true,
  "file-list-error": true,
  "remote-directory-changed": true,
  "local-files-changed": true,
  "shared-folder-changed": true,
  "transfer-started": true,
  "transfer-progress": true,
  "transfer-completed": true,
  "transfer-failed": true,
  "transfer-cancelled": true,
  "pairing-request": true,
  "pairing-sent": true,
  "pairing-approved": true,
  "pairing-rejected": true,
};

export const EVENT_NAMES = Object.keys(EVENT_NAME_SET) as EventName[];
