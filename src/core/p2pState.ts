import type { Direction, LocalFileInfo, RemoteFilesPayload } from "./contract";

export interface DiscoveredPeer {
  peerId: string;
  address: string;
  discoveredAt: Date;
}

export interface ConnectedPeer {
  peerId: string;
  deviceName: string;
  connectedAt: Date;
  sharedFolderName?: string;
}

export type TransferStatus = "active" | "completed";

export interface Transfer {
  transferId: string;
  direction: Direction;
  peerId: string;
  fileName: string;
  progress: number;
  bytesDone: number;
  totalBytes: number;
  speedBps: number;
  status: TransferStatus;
}

export interface PairingRequest {
  peerId: string;
  deviceName: string;
  pin: string;
}

export interface SentPairingPin {
  peerId: string;
  pin: string;
}

export interface P2pState {
  isP2pRunning: boolean;
  isStarting: boolean;
  isStopping: boolean;
  discoveredPeers: DiscoveredPeer[];
  connectedPeers: ConnectedPeer[];
  connectionLogs: string[];
  remoteFiles: RemoteFilesPayload | null;
  localFiles: LocalFileInfo[];
  isLoadingFiles: boolean;
  transfers: ReadonlyMap<string, Transfer>;
  pairingRequest: PairingRequest | null;
  sentPairingPin: SentPairingPin | null;
  myDeviceName: string;
  sharedFolderPath: string;
  sharedFolderDisplayName: string;
}

export const MAX_CONNECTION_LOGS = 50;

export const initialP2pState: P2pState = {
  isP2pRunning: false,
  isStarting: false,
  isStopping: false,
  discoveredPeers: [],
  connectedPeers: [],
  connectionLogs: [],
  remoteFiles: null,
  localFiles: [],
  isLoadingFiles: false,
  transfers: new Map(),
  pairingRequest: null,
  sentPairingPin: null,
  myDeviceName: "내 기기",
  sharedFolderPath: "",
  sharedFolderDisplayName: "",
};

export interface LoadedSettings {
  isP2pRunning: boolean;
  myDeviceName: string;
  sharedFolderPath: string;
  sharedFolderDisplayName: string;
}

export type P2pAction =
  | { type: "log"; message: string; at: number }
  | { type: "engine/starting" }
  | { type: "engine/started" }
  | { type: "engine/startFinished" }
  | { type: "engine/stopping" }
  | { type: "engine/stopFailed" }
  | { type: "engine/stopped" }
  | { type: "settings/loaded"; settings: LoadedSettings }
  | { type: "settings/deviceName"; name: string }
  | { type: "settings/sharedFolder"; path: string }
  | { type: "settings/sharedFolderDisplayName"; name: string }
  | { type: "localFiles/loaded"; files: LocalFileInfo[] }
  | { type: "remoteFiles/loading" }
  | { type: "remoteFiles/loadFailed" }
  | { type: "remoteFiles/received"; data: RemoteFilesPayload }
  | { type: "peers/discovered"; peerId: string; address: string; at: number }
  | { type: "peers/cleared" }
  | { type: "peers/deviceName"; peerId: string; deviceName: string }
  | { type: "peers/disconnected"; peerId: string }
  | { type: "peers/paired"; peerId: string; deviceName: string; sharedFolderName: string | null; at: number }
  | { type: "transfer/started"; transferId: string; direction: Direction; peerId: string; fileName: string; totalBytes: number }
  | { type: "transfer/progress"; transferId: string; bytesDone: number; totalBytes: number; progress: number }
  | { type: "transfer/completed"; transferId: string; speedBps: number | null }
  | { type: "transfer/removed"; transferId: string }
  | { type: "pairing/requested"; request: PairingRequest }
  | { type: "pairing/requestCleared" }
  | { type: "pairing/sent"; pin: SentPairingPin }
  | { type: "pairing/sentCleared" };
