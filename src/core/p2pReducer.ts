import type { Direction } from "./contract";
import { MAX_CONNECTION_LOGS, type P2pAction, type P2pState, type Transfer } from "./p2pState";

function withTransfer(
  transfers: ReadonlyMap<string, Transfer>,
  transferId: string,
  update: (current: Transfer) => Transfer,
): ReadonlyMap<string, Transfer> {
  const current = transfers.get(transferId);
  if (!current) return transfers;
  const next = new Map(transfers);
  next.set(transferId, update(current));
  return next;
}

export function p2pReducer(state: P2pState, action: P2pAction): P2pState {
  switch (action.type) {
    case "log": {
      const line = `[${new Date(action.at).toLocaleTimeString()}] ${action.message}`;
      return { ...state, connectionLogs: [line, ...state.connectionLogs].slice(0, MAX_CONNECTION_LOGS) };
    }
    case "engine/starting":
      return { ...state, isStarting: true };
    case "engine/started":
      return { ...state, isP2pRunning: true };
    case "engine/startFinished":
      return { ...state, isStarting: false };
    case "engine/stopping":
      return { ...state, isStopping: true };
    case "engine/stopFailed":
      return { ...state, isStopping: false };
    case "engine/stopped":
      return {
        ...state,
        isP2pRunning: false,
        isStopping: false,
        connectedPeers: [],
        discoveredPeers: [],
      };
    case "settings/loaded":
      return { ...state, ...action.settings };
    case "settings/deviceName":
      return { ...state, myDeviceName: action.name };
    case "settings/sharedFolder":
      return { ...state, sharedFolderPath: action.path };
    case "settings/sharedFolderDisplayName":
      return { ...state, sharedFolderDisplayName: action.name };
    case "localFiles/loaded":
      return { ...state, localFiles: action.files };
    case "remoteFiles/loading":
      return { ...state, isLoadingFiles: true };
    case "remoteFiles/loadFailed":
      return { ...state, isLoadingFiles: false };
    case "remoteFiles/received":
      return { ...state, remoteFiles: action.data, isLoadingFiles: false };
    case "peers/discovered": {
      const discoveredAt = new Date(action.at);
      const exists = state.discoveredPeers.some((p) => p.peerId === action.peerId);
      const discoveredPeers = exists
        ? state.discoveredPeers.map((p) =>
            p.peerId === action.peerId ? { ...p, address: action.address, discoveredAt } : p,
          )
        : [...state.discoveredPeers, { peerId: action.peerId, address: action.address, discoveredAt }];
      return { ...state, discoveredPeers };
    }
    case "peers/cleared":
      return { ...state, discoveredPeers: [] };
    case "peers/deviceName":
      return {
        ...state,
        connectedPeers: state.connectedPeers.map((p) =>
          p.peerId === action.peerId ? { ...p, deviceName: action.deviceName } : p,
        ),
      };
    case "peers/disconnected":
      return { ...state, connectedPeers: state.connectedPeers.filter((p) => p.peerId !== action.peerId) };
    case "peers/paired": {
      if (state.connectedPeers.some((p) => p.peerId === action.peerId)) {
        return { ...state, sentPairingPin: null };
      }
      return {
        ...state,
        sentPairingPin: null,
        connectedPeers: [
          ...state.connectedPeers,
          {
            peerId: action.peerId,
            deviceName: action.deviceName,
            connectedAt: new Date(action.at),
            sharedFolderName: action.sharedFolderName || undefined,
          },
        ],
      };
    }
    case "transfer/started": {
      const transfers = new Map(state.transfers);
      transfers.set(action.transferId, {
        transferId: action.transferId,
        direction: action.direction,
        peerId: action.peerId,
        fileName: action.fileName,
        progress: 0,
        bytesDone: 0,
        totalBytes: action.totalBytes,
        speedBps: 0,
        status: "active",
      });
      return { ...state, transfers };
    }
    case "transfer/progress":
      return {
        ...state,
        transfers: withTransfer(state.transfers, action.transferId, (t) => ({
          ...t,
          progress: Math.round(action.progress),
          bytesDone: action.bytesDone,
          totalBytes: action.totalBytes,
        })),
      };
    case "transfer/completed":
      return {
        ...state,
        transfers: withTransfer(state.transfers, action.transferId, (t) => ({
          ...t,
          status: "completed",
          progress: 100,
          bytesDone: t.totalBytes,
          speedBps: action.speedBps ?? 0,
        })),
      };
    case "transfer/removed": {
      if (!state.transfers.has(action.transferId)) return state;
      const transfers = new Map(state.transfers);
      transfers.delete(action.transferId);
      return { ...state, transfers };
    }
    case "pairing/requested":
      return { ...state, pairingRequest: action.request };
    case "pairing/requestCleared":
      return { ...state, pairingRequest: null };
    case "pairing/sent":
      return { ...state, sentPairingPin: action.pin };
    case "pairing/sentCleared":
      return { ...state, sentPairingPin: null };
  }
}

export function hasActiveTransfer(state: P2pState, direction: Direction): boolean {
  for (const t of state.transfers.values()) {
    if (t.direction === direction && t.status === "active") return true;
  }
  return false;
}
