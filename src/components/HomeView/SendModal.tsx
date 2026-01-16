import { Monitor, Send } from "lucide-react";
import type { ConnectedPeer } from "../../contexts/P2pContext";

interface SendModalProps {
  isOpen: boolean;
  selectedFileCount: number;
  peers: ConnectedPeer[];
  onSend: (peerId: string) => void;
  onClose: () => void;
}

export function SendModal({
  isOpen,
  selectedFileCount,
  peers,
  onSend,
  onClose,
}: SendModalProps) {
  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-[100] flex items-center justify-center bg-black/50 backdrop-blur-sm">
      <div className="bg-card border border-border rounded-2xl w-[90%] max-w-md shadow-2xl animate-in fade-in zoom-in-95 duration-200">
        <div className="p-4 border-b border-border">
          <h3 className="font-semibold">파일 전송</h3>
          <p className="text-sm text-muted-foreground mt-1">
            {selectedFileCount}개 파일을 전송할 기기를 선택하세요
          </p>
        </div>

        <div className="p-4 space-y-2 max-h-64 overflow-y-auto">
          {peers.map((peer) => (
            <button
              key={peer.peerId}
              onClick={() => onSend(peer.peerId)}
              className="w-full flex items-center gap-3 p-3 rounded-xl bg-muted/50 hover:bg-muted transition-colors text-left"
            >
              <div className="w-10 h-10 rounded-xl bg-gradient-to-br from-green-400 to-emerald-400 flex items-center justify-center">
                <Monitor className="w-5 h-5 text-white" />
              </div>
              <div className="flex-1 min-w-0">
                <p className="font-medium text-sm">
                  {peer.deviceName || "알 수 없는 기기"}
                </p>
                <p className="text-xs text-muted-foreground truncate font-mono">
                  {peer.peerId.slice(0, 16)}...
                </p>
              </div>
              <Send className="w-4 h-4 text-muted-foreground" />
            </button>
          ))}
        </div>

        <div className="p-4 border-t border-border">
          <button
            onClick={onClose}
            className="w-full py-2 rounded-xl bg-muted/50 hover:bg-muted transition-colors text-sm"
          >
            취소
          </button>
        </div>
      </div>
    </div>
  );
}
