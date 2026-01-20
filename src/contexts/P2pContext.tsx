import { createContext, useContext, useState, useEffect, useCallback, type ReactNode } from "react";

import type {
  DiscoveredPeer,
  ConnectedPeer,
  LocalFileInfo,
  RemoteFilesData,
  ActiveTransfer,
  PairingRequest,
  SentPairingPin,
} from "./p2p/types";

import { useP2pEventListeners } from "./p2p/useP2pEventListeners";
import { useP2pActions } from "./p2p/useP2pActions";
import {
  getP2pStatus,
  getDeviceName,
  getSharedFolder,
  getLocalSharedFiles,
  getSharedFolderDisplayName,
  getShowFolderName,
} from "./p2p/api";

// 타입 re-export (하위 호환성)
export type {
  DiscoveredPeer,
  ConnectedPeer,
  LocalFileInfo,
  RemoteFilesData,
  ActiveTransfer,
} from "./p2p/types";


interface P2pContextValue {
  isP2pRunning: boolean;
  isStarting: boolean;
  isStopping: boolean;
  discoveredPeers: DiscoveredPeer[];
  connectedPeers: ConnectedPeer[];
  connectionLogs: string[];
  remoteFiles: RemoteFilesData | null;
  localFiles: LocalFileInfo[];
  isLoadingFiles: boolean;
  isDownloading: boolean;
  isUploading: boolean;
  uploadProgress: number;
  downloadProgress: number;
  activeTransfers: Map<string, ActiveTransfer>;
  pairingRequest: PairingRequest | null;
  sentPairingPin: SentPairingPin | null;
  myDeviceName: string;
  sharedFolderPath: string;
  sharedFolderDisplayName: string;
  showFolderName: boolean;
  startP2p: () => Promise<void>;
  stopP2p: () => Promise<void>;
  connectToPeer: (addr: string) => Promise<void>;
  requestFileList: (peerId: string, path?: string) => Promise<void>;
  downloadFile: (peerId: string, path: string) => Promise<void>;
  uploadFile: (peerId: string, filePath: string, remotePath?: string) => Promise<void>;
  requestPairing: (peerId: string) => Promise<void>;
  respondPairing: (peerId: string, approved: boolean) => Promise<void>;
  clearAll: () => void;
  setMyDeviceName: (name: string) => Promise<void>;
  setSharedFolder: (path: string) => Promise<void>;
  setSharedFolderDisplayName: (name: string) => Promise<void>;
  setShowFolderName: (show: boolean) => Promise<void>;
  refreshLocalFiles: (path?: string) => Promise<void>;
}


const P2pContext = createContext<P2pContextValue | null>(null);

export function P2pProvider({ children }: { children: ReactNode }) {
  // ===== State =====
  const [isP2pRunning, setIsP2pRunning] = useState(false);
  const [isStarting, setIsStarting] = useState(false);
  const [isStopping, setIsStopping] = useState(false);
  const [discoveredPeers, setDiscoveredPeers] = useState<DiscoveredPeer[]>([]);
  const [connectedPeers, setConnectedPeers] = useState<ConnectedPeer[]>([]);
  const [connectionLogs, setConnectionLogs] = useState<string[]>([]);
  const [remoteFiles, setRemoteFiles] = useState<RemoteFilesData | null>(null);
  const [localFiles, setLocalFiles] = useState<LocalFileInfo[]>([]);
  const [isLoadingFiles, setIsLoadingFiles] = useState(false);
  const [isDownloading, setIsDownloading] = useState(false);
  const [downloadProgress, setDownloadProgress] = useState(0);
  const [isUploading, setIsUploading] = useState(false);
  const [uploadProgress, setUploadProgress] = useState(0);
  const [activeTransfers, setActiveTransfers] = useState<Map<string, ActiveTransfer>>(new Map());
  const [pairingRequest, setPairingRequest] = useState<PairingRequest | null>(null);
  const [sentPairingPin, setSentPairingPin] = useState<SentPairingPin | null>(null);
  const [myDeviceName, setMyDeviceNameState] = useState<string>("내 기기");
  const [sharedFolderPath, setSharedFolderPathState] = useState<string>("");
  const [sharedFolderDisplayName, setSharedFolderDisplayNameState] = useState<string>("");
  const [showFolderName, setShowFolderNameState] = useState<boolean>(true);

  // ===== Logging Helper =====
  const addLog = useCallback((message: string) => {
    const log = `[${new Date().toLocaleTimeString()}] ${message}`;
    setConnectionLogs(prev => [log, ...prev].slice(0, 50));
  }, []);

  // ===== Event Listeners (delegated to hook) =====
  useP2pEventListeners({
    setDiscoveredPeers,
    setConnectedPeers,
    setRemoteFiles,
    setIsLoadingFiles,
    setIsDownloading,
    setDownloadProgress,
    setIsUploading,
    setUploadProgress,
    setActiveTransfers,
    setPairingRequest,
    setSentPairingPin,
    setIsP2pRunning,
    setIsStopping,
    addLog,
  });

  // ===== Actions (delegated to hook) =====
  const actions = useP2pActions({
    discoveredPeers,
    addLog,
    setIsP2pRunning,
    setIsStarting,
    setIsStopping,
    setIsLoadingFiles,
    setIsDownloading,
    setIsUploading,
    setPairingRequest,
    setDiscoveredPeers,
    setLocalFiles,
    setMyDeviceNameState,
    setSharedFolderPathState,
    setSharedFolderDisplayNameState,
    setShowFolderNameState,
  });

  // ===== Initial Data Load =====
  useEffect(() => {
    async function loadInitialData() {
      try {
        const status = await getP2pStatus();
        setIsP2pRunning(status);

        const deviceName = await getDeviceName();
        setMyDeviceNameState(deviceName);

        const folderPath = await getSharedFolder();
        setSharedFolderPathState(folderPath);

        const displayName = await getSharedFolderDisplayName();
        setSharedFolderDisplayNameState(displayName);

        const showName = await getShowFolderName();
        setShowFolderNameState(showName);

        const files = await getLocalSharedFiles("/");
        setLocalFiles(files);
      } catch (e) {
        console.error("초기 데이터 로드 실패:", e);
      }
    }
    loadInitialData();
  }, []);

  return (
    <P2pContext.Provider value={{
      isP2pRunning,
      isStarting,
      isStopping,
      discoveredPeers,
      connectedPeers,
      connectionLogs,
      remoteFiles,
      localFiles,
      isLoadingFiles,
      isDownloading,
      downloadProgress,
      isUploading,
      uploadProgress,
      activeTransfers,
      myDeviceName,
      sharedFolderPath,
      sharedFolderDisplayName,
      showFolderName,
      pairingRequest,
      sentPairingPin,
      ...actions,
    }}>
      {children}
    </P2pContext.Provider>
  );
}

export function useP2pContext() {
  const context = useContext(P2pContext);
  if (!context) {
    throw new Error("useP2pContext must be used within a P2pProvider");
  }
  return context;
}
