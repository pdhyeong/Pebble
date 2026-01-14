import { Power, RefreshCw, Trash2, Wifi } from "lucide-react";
import type { DiscoveredPeer } from "../../hooks/useP2p";

interface P2PTestPanelProps {
  isP2pRunning: boolean;
  isStarting: boolean;
  discoveredPeers: DiscoveredPeer[];
  connectionLogs: string[];
  onStart: () => void;
  onConnect: (addr: string) => void;
  onClear: () => void;
}

export function P2PTestPanel({
  isP2pRunning,
  isStarting,
  discoveredPeers,
  connectionLogs,
  onStart,
  onConnect,
  onClear,
}: P2PTestPanelProps) {
  return (
    <div className="px-4 py-4 border-b border-border/50 bg-orange-500/5">
      {/* P2P 시작/상태 영역 */}
      <div className="flex items-center justify-between mb-4 p-3 rounded-xl bg-card border border-border">
        <div className="flex items-center gap-3">
          <div
            className={`w-10 h-10 rounded-xl flex items-center justify-center ${
              isP2pRunning
                ? "bg-green-500/20 text-green-600"
                : "bg-gray-500/20 text-gray-500"
            }`}
          >
            <Power className="w-5 h-5" />
          </div>
          <div>
            <p className="font-medium text-sm">
              {isP2pRunning ? "P2P 엔진 실행 중" : "P2P 엔진 중지됨"}
            </p>
            <p className="text-xs text-muted-foreground">
              {isP2pRunning
                ? "mDNS로 피어 검색 중..."
                : "시작 버튼을 눌러 검색을 시작하세요"}
            </p>
          </div>
        </div>
        <button
          onClick={onStart}
          disabled={isP2pRunning || isStarting}
          className={`px-4 py-2 rounded-xl font-medium text-sm transition-all flex items-center gap-2 ${
            isP2pRunning
              ? "bg-green-500/20 text-green-600 cursor-not-allowed"
              : isStarting
                ? "bg-orange-500/20 text-orange-600"
                : "bg-green-500 text-white hover:bg-green-600"
          }`}
        >
          {isStarting ? (
            <>
              <RefreshCw className="w-4 h-4 animate-spin" />
              시작 중...
            </>
          ) : isP2pRunning ? (
            <>
              <Wifi className="w-4 h-4" />
              실행 중
            </>
          ) : (
            <>
              <Power className="w-4 h-4" />
              P2P 시작
            </>
          )}
        </button>
      </div>

      {/* 상태 표시 및 초기화 */}
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-2">
          <div
            className={`w-3 h-3 rounded-full ${
              isP2pRunning ? "bg-green-500 animate-pulse" : "bg-gray-400"
            }`}
          />
          <span className="text-sm font-medium">
            {isP2pRunning ? "피어 검색 활성화됨" : "대기 중"}
          </span>
        </div>
        <button
          onClick={onClear}
          className="p-2 rounded-lg bg-muted/50 hover:bg-muted transition-colors"
          title="목록 지우기"
        >
          <Trash2 className="w-4 h-4" />
        </button>
      </div>

      {/* Discovered Peers */}
      <div className="mb-4">
        <h4 className="text-sm font-medium mb-2 text-muted-foreground">
          발견된 피어 ({discoveredPeers.length})
        </h4>
        {discoveredPeers.length === 0 ? (
          <div className="p-4 rounded-xl bg-muted/30 border border-border/50 text-center">
            <p className="text-sm text-muted-foreground">
              {isP2pRunning
                ? "같은 네트워크에서 다른 Pebble 앱을 실행하면 여기에 표시됩니다"
                : "P2P 엔진을 시작하면 피어 검색이 시작됩니다"}
            </p>
          </div>
        ) : (
          <div className="space-y-2 max-h-48 overflow-y-auto">
            {discoveredPeers.map((peer) => (
              <div
                key={peer.peerId}
                className="flex items-center justify-between p-3 rounded-xl bg-card border border-border"
              >
                <div className="flex-1 min-w-0 mr-3">
                  <p className="text-xs font-mono truncate text-primary">
                    {peer.peerId}
                  </p>
                  <p className="text-xs text-muted-foreground truncate">
                    {peer.address}
                  </p>
                  <p className="text-xs text-muted-foreground/60">
                    발견: {peer.discoveredAt.toLocaleTimeString()}
                  </p>
                </div>
                <button
                  onClick={() => onConnect(peer.address)}
                  className="px-3 py-1.5 rounded-lg bg-primary text-white text-xs font-medium hover:bg-primary/90 transition-colors flex-shrink-0"
                >
                  연결
                </button>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Connection Logs */}
      <div>
        <h4 className="text-sm font-medium mb-2 text-muted-foreground">
          연결 로그
        </h4>
        <div className="p-3 rounded-xl bg-black/80 border border-border/50 h-32 overflow-y-auto font-mono text-xs">
          {connectionLogs.length === 0 ? (
            <p className="text-gray-500">로그가 여기에 표시됩니다...</p>
          ) : (
            connectionLogs.map((log, index) => (
              <p
                key={index}
                className={`${
                  log.includes("실패")
                    ? "text-red-400"
                    : log.includes("성공")
                      ? "text-green-400"
                      : log.includes("발견")
                        ? "text-blue-400"
                        : "text-gray-300"
                }`}
              >
                {log}
              </p>
            ))
          )}
        </div>
      </div>
    </div>
  );
}
