// P2P 관련 타입 정의

/** 발견된 피어 */
export interface DiscoveredPeer {
    peerId: string;
    address: string;
    discoveredAt: Date;
}

/** 연결된 피어 */
export interface ConnectedPeer {
    peerId: string;
    deviceName: string;
    connectedAt: Date;
    sharedFolderName?: string;  // 상대방 공유 폴더 이름
}

/** 로컬 파일 정보 */
export interface LocalFileInfo {
    name: string;
    path: string;
    is_dir: boolean;
    size: number;
}

/** 원격 파일 정보 */
export interface RemoteFileInfo {
    name: string;
    path: string;
    is_dir: boolean;
    size: number;
}

/** 원격 파일 목록 응답 */
export interface RemoteFilesData {
    peer_id: string;
    path: string;
    files: RemoteFileInfo[];
    error: string | null;
}

/** 활성 전송 상태 */
export interface ActiveTransfer {
    type: "upload" | "download";
    fileName: string;
    progress: number;
    bytesTransferred: number;
    totalBytes: number;
    speed: number;
    transferId: string;
}

/** 페어링 요청 정보 */
export interface PairingRequest {
    peerId: string;
    deviceName: string;
    pin: string;
}

/** 전송된 페어링 PIN */
export interface SentPairingPin {
    peerId: string;
    pin: string;
}

/** P2P Context 값 인터페이스 */
export interface P2pContextValue {
    // 상태
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

    // 액션
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
