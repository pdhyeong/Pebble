import { useState, useEffect } from "react";
import { Monitor, Plus, Radio } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { useP2pContext } from "../../contexts/P2pContext";
import { DeviceCard, type Device } from "./DeviceCard";
import { P2PTestPanel } from "./P2PTestPanel";
import { PairDeviceModal } from "./PairDeviceModal";

// 임시 더미 데이터 (실제로는 서버/로컬에서 불러와야 함)
const mockDevices: Device[] = [
  {
    id: "1",
    name: "맥북 프로",
    type: "desktop",
    status: "online",
    lastSeen: "방금",
    ipAddress: "192.168.0.102",
    osInfo: "macOS Sonoma 14.2",
  },
  {
    id: "2",
    name: "내 아이폰",
    type: "mobile",
    status: "online",
    lastSeen: "5분 전",
    ipAddress: "192.168.0.103",
    osInfo: "iOS 17.2",
  },
  {
    id: "3",
    name: "아이패드",
    type: "tablet",
    status: "offline",
    lastSeen: "2시간 전",
    ipAddress: "192.168.0.104",
    osInfo: "iPadOS 17.2",
  },
];

export function DevicesView() {
  const [showPairModal, setShowPairModal] = useState(false);
  const [showTestPanel, setShowTestPanel] = useState(false);
  const [localIp, setLocalIp] = useState<string>("로딩 중...");

  const {
    isP2pRunning,
    isStarting,
    isStopping,
    discoveredPeers,
    connectedPeers,
    connectionLogs,
    startP2p,
    stopP2p,
    connectToPeer,
    clearAll,
  } = useP2pContext();

  // 로컬 IP 주소 가져오기
  useEffect(() => {
    async function fetchLocalIp() {
      try {
        const ip = await invoke<string>("get_local_ip");
        setLocalIp(ip);
      } catch (e) {
        console.error("IP 조회 실패:", e);
        setLocalIp("알 수 없음");
      }
    }
    fetchLocalIp();
  }, []);

  const devices = mockDevices;
  const onlineCount = devices.filter((d) => d.status === "online").length;

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="sticky top-16 z-40 bg-background/95 backdrop-blur-xl border-b border-border/50 px-4 py-4">
        <div className="flex items-center justify-between mb-2">
          <div>
            <h2 className="mb-1">연결된 기기</h2>
            <p className="text-sm text-muted-foreground">
              {onlineCount}개 기기 온라인
            </p>
          </div>
          <div className="flex items-center gap-2">
            <button
              onClick={() => setShowTestPanel(!showTestPanel)}
              className={`px-3 py-2 rounded-xl font-medium transition-all flex items-center gap-2 text-sm ${
                showTestPanel
                  ? "bg-orange-500 text-white"
                  : "bg-orange-500/10 text-orange-600 border border-orange-500/20"
              }`}
            >
              <Radio className="w-4 h-4" />
              P2P 테스트
            </button>
            <button
              onClick={() => setShowPairModal(true)}
              className="px-4 py-2.5 rounded-xl bg-gradient-to-r from-primary to-chart-2 text-white font-medium hover:shadow-lg transition-shadow flex items-center gap-2"
            >
              <Plus className="w-4 h-4" />
              기기 추가
            </button>
          </div>
        </div>
      </div>

      {/* P2P Test Panel */}
      {showTestPanel && (
        <P2PTestPanel
          isP2pRunning={isP2pRunning}
          isStarting={isStarting}
          isStopping={isStopping}
          discoveredPeers={discoveredPeers}
          connectedPeers={connectedPeers}
          connectionLogs={connectionLogs}
          onStart={startP2p}
          onStop={stopP2p}
          onConnect={connectToPeer}
          onClear={clearAll}
        />
      )}

      {/* Devices List */}
      <div className="flex-1 overflow-y-auto px-4 py-4">
        {/* 내 IP 주소 카드 */}
        <div className="mb-4 p-4 rounded-2xl bg-gradient-to-br from-primary/5 to-chart-2/5 border border-border/50">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <div className="w-10 h-10 rounded-xl bg-gradient-to-br from-primary to-chart-2 flex items-center justify-center">
                <Radio className="w-5 h-5 text-white" />
              </div>
              <div>
                <p className="text-sm text-muted-foreground">내 IP 주소</p>
                <p className="font-mono font-medium">{localIp}</p>
              </div>
            </div>
            <div className="flex items-center gap-2">
              <div className="w-2 h-2 rounded-full bg-green-400 animate-pulse" />
              <span className="text-xs text-muted-foreground">연결됨</span>
            </div>
          </div>
        </div>

        <div className="space-y-3">
          {devices.map((device) => (
            <DeviceCard key={device.id} device={device} />
          ))}
        </div>

        {/* Empty State */}
        {devices.length === 0 && (
          <div className="flex flex-col items-center justify-center py-16 text-center">
            <div className="w-20 h-20 rounded-3xl bg-muted/50 flex items-center justify-center mb-4">
              <Monitor className="w-10 h-10 text-muted-foreground" />
            </div>
            <p className="text-muted-foreground mb-2">연결된 기기가 없습니다</p>
            <p className="text-sm text-muted-foreground mb-4">
              새 기기를 추가하여 파일을 공유하세요
            </p>
            <button
              onClick={() => setShowPairModal(true)}
              className="px-5 py-2.5 rounded-xl bg-gradient-to-r from-primary to-chart-2 text-white font-medium hover:shadow-lg transition-shadow flex items-center gap-2"
            >
              <Plus className="w-4 h-4" />
              기기 추가
            </button>
          </div>
        )}
      </div>

      {/* Pair Device Modal */}
      <PairDeviceModal
        isOpen={showPairModal}
        onClose={() => setShowPairModal(false)}
      />
    </div>
  );
}
