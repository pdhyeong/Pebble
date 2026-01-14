import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export interface DiscoveredPeer {
  peerId: string;
  address: string;
  discoveredAt: Date;
}

export function useP2p() {
  const [isP2pRunning, setIsP2pRunning] = useState(false);
  const [isStarting, setIsStarting] = useState(false);
  const [discoveredPeers, setDiscoveredPeers] = useState<DiscoveredPeer[]>([]);
  const [connectionLogs, setConnectionLogs] = useState<string[]>([]);

  // 로그 추가 헬퍼
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
        addLog(`✅ 연결 성공: ${event.payload.slice(0, 12)}...`);
      })
    );

    // 연결 종료
    unlisteners.push(
      listen<string>("connection-closed", (event) => {
        addLog(`❌ 연결 종료: ${event.payload.slice(0, 12)}...`);
      })
    );

    unlisteners.push(
      listen<string>("listening-on", (event) => {
        addLog(`리스닝: ${event.payload}`);
      })
    );

    unlisteners.push(
      listen<string>("peer-info", (event) => {
        addLog(`피어 정보: ${event.payload}`);
      })
    );

    unlisteners.push(
      listen<string>("ping-success", (event) => {
        const [peerId, rttMs] = event.payload.split(":");
        addLog(`Ping: ${peerId.slice(0, 12)}... (${rttMs}ms)`);
      })
    );

    unlisteners.push(
      listen<string>("dial-started", (event) => {
        addLog(`연결 시도 중: ${event.payload}`);
      })
    );

    unlisteners.push(
      listen<string>("dial-failed", (event) => {
        addLog(`연결 시도 실패: ${event.payload}`);
      })
    );

    return () => {
      unlisteners.forEach(p => p.then(fn => fn()));
    };
  }, [addLog]);

  // P2P 시작
  const startP2p = useCallback(async () => {
    setIsStarting(true);
    addLog("P2P 엔진 시작 중...");

    try {
      const result = await invoke<string>("start_p2p");
      addLog(result);
      setIsP2pRunning(true);
    } catch (e) {
      addLog(`P2P 시작 실패: ${e}`);
      console.error("P2P 시작 실패:", e);
    } finally {
      setIsStarting(false);
    }
  }, [addLog]);

  // 피어 연결
  const connectToPeer = useCallback(async (peerAddr: string) => {
    addLog(`연결 시도: ${peerAddr}`);

    try {
      await invoke("connect_to_peer", { addr: peerAddr });
    } catch (e) {
      addLog(`연결 실패: ${e}`);
      console.error("연결 실패:", e);
    }
  }, [addLog]);

  // 목록 초기화
  const clearAll = useCallback(() => {
    setDiscoveredPeers([]);
    setConnectionLogs([]);
  }, []);

  return {
    isP2pRunning,
    isStarting,
    discoveredPeers,
    connectionLogs,
    startP2p,
    connectToPeer,
    clearAll,
  };
}
