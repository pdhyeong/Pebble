import { useState } from "react";
import { motion } from "motion/react";
import { Monitor, Plus, Radio } from "lucide-react";
import { useP2p } from "../../hooks/useP2p";
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

  const {
    isP2pRunning,
    isStarting,
    discoveredPeers,
    connectionLogs,
    startP2p,
    connectToPeer,
    clearAll,
  } = useP2p();

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
          discoveredPeers={discoveredPeers}
          connectionLogs={connectionLogs}
          onStart={startP2p}
          onConnect={connectToPeer}
          onClear={clearAll}
        />
      )}

      {/* Devices List */}
      <div className="flex-1 overflow-y-auto px-4 py-4">
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

      {/* Network Info Footer */}
      <div className="sticky bottom-0 bg-gradient-to-br from-primary/5 to-chart-2/5 border-t border-border/50 px-4 py-3">
        <div className="flex items-center justify-between text-sm mb-2">
          <span className="text-muted-foreground">네트워크</span>
          <span className="font-medium font-mono">192.168.0.1</span>
        </div>
        <div className="flex items-center gap-2">
          <div className="flex-1 h-2 bg-muted/50 rounded-full overflow-hidden">
            <motion.div
              initial={{ width: 0 }}
              animate={{ width: "85%" }}
              transition={{ duration: 1, ease: "easeOut" }}
              className="h-full bg-gradient-to-r from-green-400 to-emerald-400 rounded-full"
            />
          </div>
          <span className="text-xs text-muted-foreground">신호 강도</span>
        </div>
      </div>

      {/* Pair Device Modal */}
      <PairDeviceModal
        isOpen={showPairModal}
        onClose={() => setShowPairModal(false)}
      />
    </div>
  );
}
