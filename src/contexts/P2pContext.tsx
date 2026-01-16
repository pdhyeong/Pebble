import { createContext, useContext, useState, useEffect, useCallback, type ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export interface DiscoveredPeer {
  peerId: string;
  address: string;
  discoveredAt: Date;
}

export interface ConnectedPeer {
  peerId: string;
  deviceName: string;
  connectedAt: Date;
}

export interface LocalFileInfo {
  name: string;
  path: string;
  is_dir: boolean;
  size: number;
}

export interface RemoteFileInfo {
  name: string;
  path: string;
  is_dir: boolean;
  size: number;
}

export interface RemoteFilesData {
  peer_id: string;
  path: string;
  files: RemoteFileInfo[];
  error: string | null;
}

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
  myDeviceName: string;
  sharedFolderPath: string;
  startP2p: () => Promise<void>;
  stopP2p: () => Promise<void>;
  connectToPeer: (addr: string) => Promise<void>;
  requestFileList: (peerId: string, path?: string) => Promise<void>;
  downloadFile: (peerId: string, path: string) => Promise<void>;
  uploadFile: (peerId: string, filePath: string, remotePath?: string) => Promise<void>;
  clearAll: () => void;
  setMyDeviceName: (name: string) => Promise<void>;
  setSharedFolder: (path: string) => Promise<void>;
  refreshLocalFiles: (path?: string) => Promise<void>;
}

const P2pContext = createContext<P2pContextValue | null>(null);

export function P2pProvider({ children }: { children: ReactNode }) {
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
  const [isUploading, setIsUploading] = useState(false);
  const [myDeviceName, setMyDeviceNameState] = useState("내 기기");
  const [sharedFolderPath, setSharedFolderPathState] = useState("");

  const addLog = useCallback((message: string) => {
    const log = `[${new Date().toLocaleTimeString()}] ${message}`;
    setConnectionLogs(prev => [log, ...prev].slice(0, 50));
  }, []);

  // 초기 데이터 로드
  useEffect(() => {
    async function loadInitialData() {
      try {
        // P2P 상태 확인
        const status = await invoke<boolean>("get_p2p_status");
        setIsP2pRunning(status);

        // 기기 이름 로드
        const deviceName = await invoke<string>("get_device_name");
        setMyDeviceNameState(deviceName);

        // 공유 폴더 경로 로드
        const folderPath = await invoke<string>("get_shared_folder");
        setSharedFolderPathState(folderPath);

        // 로컬 파일 목록 로드
        const files = await invoke<LocalFileInfo[]>("get_local_shared_files", { relativePath: "/" });
        setLocalFiles(files);
      } catch (e) {
        console.error("초기 데이터 로드 실패:", e);
      }
    }
    loadInitialData();
  }, []);

  // P2P 이벤트 리스너들
  useEffect(() => {
    const unlisteners: Promise<() => void>[] = [];

    // 피어 발견
    unlisteners.push(
      listen<string>("peer-found", (event) => {
        const payload = event.payload;
        const [peerId, ...addrParts] = payload.split(":");
        const address = addrParts.join(":");

        addLog(`🔍 피어 발견: ${peerId.slice(0, 12)}...`);

        setDiscoveredPeers(prev => {
          const exists = prev.find(p => p.peerId === peerId);
          if (exists) {
            return prev.map(p =>
              p.peerId === peerId
                ? { ...p, address, discoveredAt: new Date() }
                : p
            );
          }
          return [...prev, { peerId, address, discoveredAt: new Date() }];
        });
      })
    );

    // 연결 성공
    unlisteners.push(
      listen<string>("connection-success", (event) => {
        const peerId = event.payload;
        addLog(`✅ 연결 성공: ${peerId.slice(0, 12)}...`);
        setConnectedPeers(prev => {
          if (prev.find(p => p.peerId === peerId)) return prev;
          return [...prev, { peerId, deviceName: "연결 중...", connectedAt: new Date() }];
        });
      })
    );

    // 기기 정보 수신
    unlisteners.push(
      listen<string>("device-info-received", (event) => {
        try {
          const data = JSON.parse(event.payload);
          addLog(`📱 기기 이름 수신: ${data.device_name}`);
          setConnectedPeers(prev =>
            prev.map(p =>
              p.peerId === data.peer_id
                ? { ...p, deviceName: data.device_name }
                : p
            )
          );
        } catch (e) {
          console.error("기기 정보 파싱 실패:", e);
        }
      })
    );

    // 연결 종료
    unlisteners.push(
      listen<string>("connection-closed", (event) => {
        const peerId = event.payload;
        addLog(`❌ 연결 종료: ${peerId.slice(0, 12)}...`);
        setConnectedPeers(prev => prev.filter(p => p.peerId !== peerId));
      })
    );

    unlisteners.push(
      listen<string>("listening-on", (event) => {
        addLog(`📡 리스닝: ${event.payload}`);
      })
    );

    unlisteners.push(
      listen<string>("peer-info", (event) => {
        addLog(`ℹ️ 피어 정보: ${event.payload}`);
      })
    );

    unlisteners.push(
      listen<string>("ping-success", (event) => {
        const [peerId, rttMs] = event.payload.split(":");
        addLog(`🏓 Ping: ${peerId.slice(0, 12)}... (${rttMs}ms)`);
      })
    );

    unlisteners.push(
      listen<string>("dial-started", (event) => {
        addLog(`📞 연결 시도 중: ${event.payload}`);
      })
    );

    unlisteners.push(
      listen<string>("dial-failed", (event) => {
        addLog(`❌ 연결 시도 실패: ${event.payload}`);
      })
    );

    // 원격 파일 목록 수신
    unlisteners.push(
      listen<string>("remote-files", (event) => {
        try {
          const data: RemoteFilesData = JSON.parse(event.payload);
          addLog(`📂 파일 목록 수신: ${data.files.length}개 파일`);
          setRemoteFiles(data);
          setIsLoadingFiles(false);
        } catch (e) {
          addLog(`❌ 파일 목록 파싱 실패: ${e}`);
          setIsLoadingFiles(false);
        }
      })
    );

    unlisteners.push(
      listen<string>("file-list-request-sent", (event) => {
        addLog(`📤 파일 목록 요청 전송: ${event.payload}`);
      })
    );

    unlisteners.push(
      listen<string>("file-list-error", (event) => {
        addLog(`❌ 파일 목록 요청 실패: ${event.payload}`);
        setIsLoadingFiles(false);
      })
    );

    // P2P 종료
    unlisteners.push(
      listen<string>("p2p-stopped", (event) => {
        addLog(`🛑 ${event.payload}`);
        setIsP2pRunning(false);
        setIsStopping(false);
        setConnectedPeers([]);
        setDiscoveredPeers([]);
      })
    );

    // 파일 다운로드 시작
    unlisteners.push(
      listen<string>("file-download-started", (event) => {
        addLog(`📥 파일 다운로드 시작: ${event.payload}`);
        setIsDownloading(true);
      })
    );

    // 파일 다운로드 완료
    unlisteners.push(
      listen<string>("file-download-complete", (event) => {
        try {
          const data = JSON.parse(event.payload);
          addLog(`✅ 파일 다운로드 완료: ${data.saved_path}`);
          setIsDownloading(false);
        } catch {
          addLog(`✅ 파일 다운로드 완료`);
          setIsDownloading(false);
        }
      })
    );

    // 파일 다운로드 에러
    unlisteners.push(
      listen<string>("file-download-error", (event) => {
        addLog(`❌ 파일 다운로드 실패: ${event.payload}`);
        setIsDownloading(false);
      })
    );

    // 파일 업로드 시작
    unlisteners.push(
      listen<string>("file-upload-started", (event) => {
        try {
          const data = JSON.parse(event.payload);
          addLog(`📤 파일 업로드 시작: ${data.file_name}`);
          setIsUploading(true);
        } catch {
          addLog(`📤 파일 업로드 시작`);
          setIsUploading(true);
        }
      })
    );

    // 파일 업로드 완료
    unlisteners.push(
      listen<string>("file-upload-complete", (event) => {
        try {
          const data = JSON.parse(event.payload);
          addLog(`✅ 파일 업로드 완료: ${data.file_name}`);
          setIsUploading(false);
        } catch {
          addLog(`✅ 파일 업로드 완료`);
          setIsUploading(false);
        }
      })
    );

    // 파일 업로드 에러
    unlisteners.push(
      listen<string>("file-upload-error", (event) => {
        addLog(`❌ 파일 업로드 실패: ${event.payload}`);
        setIsUploading(false);
      })
    );

    // 파일 수신 (상대방이 나에게 파일을 보냈을 때)
    unlisteners.push(
      listen<string>("file-received", (event) => {
        try {
          const data = JSON.parse(event.payload);
          if (data.success) {
            addLog(`📥 파일 수신: ${data.file_name} -> ${data.saved_path}`);
          } else {
            addLog(`❌ 파일 수신 실패: ${data.error}`);
          }
        } catch {
          addLog(`📥 파일 수신 완료`);
        }
      })
    );

    return () => {
      unlisteners.forEach(p => p.then(fn => fn()));
    };
  }, [addLog]);

  const startP2p = useCallback(async () => {
    setIsStarting(true);
    addLog("🚀 P2P 엔진 시작 중...");

    try {
      const result = await invoke<string>("start_p2p");
      addLog(result);
      setIsP2pRunning(true);
    } catch (e) {
      addLog(`❌ P2P 시작 실패: ${e}`);
      console.error("P2P 시작 실패:", e);
    } finally {
      setIsStarting(false);
    }
  }, [addLog]);

  const stopP2p = useCallback(async () => {
    setIsStopping(true);
    addLog("🛑 P2P 엔진 종료 중...");

    try {
      const result = await invoke<string>("stop_p2p");
      addLog(result);
    } catch (e) {
      addLog(`❌ P2P 종료 실패: ${e}`);
      console.error("P2P 종료 실패:", e);
      setIsStopping(false);
    }
  }, [addLog]);

  const connectToPeer = useCallback(async (peerAddr: string) => {
    addLog(`📞 연결 시도: ${peerAddr}`);

    try {
      await invoke("connect_to_peer", { addr: peerAddr });
    } catch (e) {
      addLog(`❌ 연결 실패: ${e}`);
      console.error("연결 실패:", e);
    }
  }, [addLog]);

  const requestFileList = useCallback(async (peerId: string, path: string = "/") => {
    addLog(`📂 파일 목록 요청: ${peerId.slice(0, 12)}... (${path})`);
    setIsLoadingFiles(true);

    try {
      await invoke("request_file_list", { peerId, path });
    } catch (e) {
      addLog(`❌ 파일 목록 요청 실패: ${e}`);
      console.error("파일 목록 요청 실패:", e);
      setIsLoadingFiles(false);
    }
  }, [addLog]);

  const downloadFile = useCallback(async (peerId: string, path: string) => {
    addLog(`📥 파일 다운로드 요청: ${path}`);
    setIsDownloading(true);

    try {
      await invoke("download_file", { peerId, path });
    } catch (e) {
      addLog(`❌ 파일 다운로드 요청 실패: ${e}`);
      console.error("파일 다운로드 요청 실패:", e);
      setIsDownloading(false);
    }
  }, [addLog]);

  const uploadFile = useCallback(async (peerId: string, filePath: string, remotePath: string = "/") => {
    addLog(`📤 파일 업로드 요청: ${filePath} -> ${remotePath}`);
    setIsUploading(true);

    try {
      await invoke("upload_file", { peerId, filePath, remotePath });
    } catch (e) {
      addLog(`❌ 파일 업로드 요청 실패: ${e}`);
      console.error("파일 업로드 요청 실패:", e);
      setIsUploading(false);
    }
  }, [addLog]);

  const clearAll = useCallback(() => {
    // discoveredPeers만 지우기 (connectedPeers, logs, remoteFiles는 유지)
    setDiscoveredPeers([]);
  }, []);

  const setMyDeviceName = useCallback(async (name: string) => {
    try {
      await invoke("set_device_name", { name });
      setMyDeviceNameState(name);
      addLog(`📱 기기 이름 설정: ${name}`);
    } catch (e) {
      console.error("기기 이름 설정 실패:", e);
    }
  }, [addLog]);

  const setSharedFolder = useCallback(async (path: string) => {
    try {
      await invoke("set_shared_folder", { path });
      setSharedFolderPathState(path);
      addLog(`📁 공유 폴더 설정: ${path}`);
      // 파일 목록 새로고침
      const files = await invoke<LocalFileInfo[]>("get_local_shared_files", { relativePath: "/" });
      setLocalFiles(files);
    } catch (e) {
      console.error("공유 폴더 설정 실패:", e);
    }
  }, [addLog]);

  const refreshLocalFiles = useCallback(async (relativePath: string = "/") => {
    try {
      const files = await invoke<LocalFileInfo[]>("get_local_shared_files", { relativePath });
      setLocalFiles(files);
    } catch (e) {
      console.error("파일 목록 새로고침 실패:", e);
    }
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
      isUploading,
      myDeviceName,
      sharedFolderPath,
      startP2p,
      stopP2p,
      connectToPeer,
      requestFileList,
      downloadFile,
      uploadFile,
      clearAll,
      setMyDeviceName,
      setSharedFolder,
      refreshLocalFiles,
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
