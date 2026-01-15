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
  connectedAt: Date;
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
  discoveredPeers: DiscoveredPeer[];
  connectedPeers: ConnectedPeer[];
  connectionLogs: string[];
  remoteFiles: RemoteFilesData | null;
  isLoadingFiles: boolean;
  startP2p: () => Promise<void>;
  connectToPeer: (addr: string) => Promise<void>;
  requestFileList: (peerId: string, path?: string) => Promise<void>;
  clearAll: () => void;
}

const P2pContext = createContext<P2pContextValue | null>(null);

export function P2pProvider({ children }: { children: ReactNode }) {
  const [isP2pRunning, setIsP2pRunning] = useState(false);
  const [isStarting, setIsStarting] = useState(false);
  const [discoveredPeers, setDiscoveredPeers] = useState<DiscoveredPeer[]>([]);
  const [connectedPeers, setConnectedPeers] = useState<ConnectedPeer[]>([]);
  const [connectionLogs, setConnectionLogs] = useState<string[]>([]);
  const [remoteFiles, setRemoteFiles] = useState<RemoteFilesData | null>(null);
  const [isLoadingFiles, setIsLoadingFiles] = useState(false);

  const addLog = useCallback((message: string) => {
    const log = `[${new Date().toLocaleTimeString()}] ${message}`;
    setConnectionLogs(prev => [log, ...prev].slice(0, 50));
  }, []);

  // P2P 상태 확인
  useEffect(() => {
    async function checkStatus() {
      try {
        const status = await invoke<boolean>("get_p2p_status");
        setIsP2pRunning(status);
      } catch (e) {
        console.error("상태 확인 실패:", e);
      }
    }
    checkStatus();
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
          return [...prev, { peerId, connectedAt: new Date() }];
        });
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

  const clearAll = useCallback(() => {
    setDiscoveredPeers([]);
    setConnectionLogs([]);
    setRemoteFiles(null);
  }, []);

  return (
    <P2pContext.Provider value={{
      isP2pRunning,
      isStarting,
      discoveredPeers,
      connectedPeers,
      connectionLogs,
      remoteFiles,
      isLoadingFiles,
      startP2p,
      connectToPeer,
      requestFileList,
      clearAll,
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
