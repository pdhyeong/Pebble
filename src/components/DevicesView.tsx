import { motion } from "motion/react";
import { Smartphone, Monitor, Tablet, Wifi, WifiOff, QrCode, Plus, CheckCircle2, MoreVertical } from "lucide-react";
import { useState } from "react";

interface Device {
  id: string;
  name: string;
  type: "desktop" | "mobile" | "tablet";
  status: "online" | "offline";
  lastSeen: string;
  ipAddress: string;
  osInfo: string;
}

export function DevicesView() {
  const [showPairModal, setShowPairModal] = useState(false);

  const devices: Device[] = [
    {
      id: "1",
      name: "맥북 프로",
      type: "desktop",
      status: "online",
      lastSeen: "방금",
      ipAddress: "192.168.0.102",
      osInfo: "macOS Sonoma 14.2"
    },
    {
      id: "2",
      name: "내 아이폰",
      type: "mobile",
      status: "online",
      lastSeen: "5분 전",
      ipAddress: "192.168.0.103",
      osInfo: "iOS 17.2"
    },
    {
      id: "3",
      name: "아이패드",
      type: "tablet",
      status: "offline",
      lastSeen: "2시간 전",
      ipAddress: "192.168.0.104",
      osInfo: "iPadOS 17.2"
    },
  ];

  const getDeviceIcon = (type: string) => {
    switch (type) {
      case "desktop": return Monitor;
      case "mobile": return Smartphone;
      case "tablet": return Tablet;
      default: return Monitor;
    }
  };

  const getDeviceColor = (type: string) => {
    switch (type) {
      case "desktop": return "from-blue-400 to-cyan-400";
      case "mobile": return "from-purple-400 to-indigo-400";
      case "tablet": return "from-pink-400 to-rose-400";
      default: return "from-gray-400 to-gray-500";
    }
  };

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="sticky top-16 z-40 bg-background/95 backdrop-blur-xl border-b border-border/50 px-4 py-4">
        <div className="flex items-center justify-between mb-2">
          <div>
            <h2 className="mb-1">연결된 기기</h2>
            <p className="text-sm text-muted-foreground">
              {devices.filter(d => d.status === "online").length}개 기기 온라인
            </p>
          </div>
          <button
            onClick={() => setShowPairModal(true)}
            className="px-4 py-2.5 rounded-xl bg-gradient-to-r from-primary to-chart-2 text-white font-medium hover:shadow-lg transition-shadow flex items-center gap-2"
          >
            <Plus className="w-4 h-4" />
            기기 추가
          </button>
        </div>
      </div>

      {/* Devices List */}
      <div className="flex-1 overflow-y-auto px-4 py-4">
        <div className="space-y-3">
          {devices.map((device) => {
            const Icon = getDeviceIcon(device.type);
            const gradient = getDeviceColor(device.type);
            const isOnline = device.status === "online";

            return (
              <div
                key={device.id}
                className="flex items-center gap-4 p-4 rounded-2xl bg-card border border-border hover:shadow-lg transition-all"
              >
                <div className={`w-14 h-14 rounded-xl bg-gradient-to-br ${gradient} flex items-center justify-center flex-shrink-0 shadow-md relative`}>
                  <Icon className="w-7 h-7 text-white" />
                  <div className={`absolute -bottom-1 -right-1 w-5 h-5 rounded-full border-2 border-card flex items-center justify-center ${
                    isOnline ? "bg-green-500" : "bg-gray-400"
                  }`}>
                    {isOnline ? (
                      <Wifi className="w-3 h-3 text-white" />
                    ) : (
                      <WifiOff className="w-3 h-3 text-white" />
                    )}
                  </div>
                </div>

                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2 mb-1">
                    <h4 className="truncate">{device.name}</h4>
                    {isOnline && (
                      <span className="px-2 py-0.5 rounded-full bg-green-500/10 text-green-600 text-xs font-medium border border-green-500/20">
                        온라인
                      </span>
                    )}
                  </div>
                  <p className="text-xs text-muted-foreground mb-1">
                    {device.osInfo}
                  </p>
                  <div className="flex items-center gap-3 text-xs text-muted-foreground/70">
                    <span className="font-mono">{device.ipAddress}</span>
                    <span>•</span>
                    <span>마지막 접속: {device.lastSeen}</span>
                  </div>
                </div>

                <button className="p-2 rounded-lg hover:bg-muted/50 transition-colors flex-shrink-0">
                  <MoreVertical className="w-4 h-4 text-muted-foreground" />
                </button>
              </div>
            );
          })}
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
      {showPairModal && (
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          className="fixed inset-0 bg-black/50 backdrop-blur-sm z-50 flex items-center justify-center p-4"
          onClick={() => setShowPairModal(false)}
        >
          <motion.div
            initial={{ scale: 0.9, opacity: 0 }}
            animate={{ scale: 1, opacity: 1 }}
            exit={{ scale: 0.9, opacity: 0 }}
            onClick={(e) => e.stopPropagation()}
            className="bg-background w-full max-w-md rounded-3xl overflow-hidden shadow-2xl"
          >
            {/* Header */}
            <div className="bg-gradient-to-br from-primary/10 to-chart-2/10 border-b border-border/50 px-6 py-5 text-center">
              <h3 className="mb-2">새 기기 연결</h3>
              <p className="text-sm text-muted-foreground">
                연결할 기기에서 이 QR 코드를 스캔하세요
              </p>
            </div>

            {/* QR Code */}
            <div className="p-8 flex flex-col items-center">
              <div className="w-64 h-64 rounded-2xl bg-white p-6 mb-6 shadow-lg">
                <div className="w-full h-full bg-gradient-to-br from-primary to-chart-2 rounded-xl flex items-center justify-center">
                  <QrCode className="w-32 h-32 text-white" />
                </div>
              </div>

              <div className="text-center mb-6">
                <p className="text-sm text-muted-foreground mb-2">또는 페어링 코드 입력</p>
                <div className="flex items-center justify-center gap-2">
                  <span className="px-4 py-2 rounded-lg bg-muted/50 font-mono text-lg font-bold">
                    8 4 2 7
                  </span>
                </div>
              </div>

              <div className="w-full space-y-2">
                <div className="flex items-center gap-3 p-3 rounded-xl bg-muted/30 border border-border/50">
                  <CheckCircle2 className="w-5 h-5 text-primary flex-shrink-0" />
                  <span className="text-sm">같은 Wi-Fi 네트워크에 연결됨</span>
                </div>
                <div className="flex items-center gap-3 p-3 rounded-xl bg-muted/30 border border-border/50">
                  <CheckCircle2 className="w-5 h-5 text-primary flex-shrink-0" />
                  <span className="text-sm">Pebble 앱이 설치됨</span>
                </div>
              </div>
            </div>

            {/* Footer */}
            <div className="border-t border-border/50 px-6 py-4">
              <button
                onClick={() => setShowPairModal(false)}
                className="w-full px-6 py-3 rounded-xl bg-muted/50 hover:bg-muted transition-colors font-medium"
              >
                취소
              </button>
            </div>
          </motion.div>
        </motion.div>
      )}
    </div>
  );
}
