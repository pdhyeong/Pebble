import { Send } from "lucide-react";
import type { ReactNode } from "react";

interface ActionButton {
  label: string;
  icon: ReactNode;
  onClick: () => void;
  disabled?: boolean;
  loading?: boolean;
}

interface SelectionBarProps {
  selectedCount: number;
  onCancel: () => void;
  primaryAction?: ActionButton;
  connectedPeersCount?: number;
  onSend?: () => void;
}

export function SelectionBar({
  selectedCount,
  onCancel,
  primaryAction,
  connectedPeersCount = 0,
  onSend,
}: SelectionBarProps) {
  if (selectedCount === 0) return null;

  return (
    <div className="sticky bottom-0 z-50 bg-background/95 backdrop-blur-xl border-t border-border/50 px-4 py-3">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium">{selectedCount}개 파일 선택됨</span>
          <button
            onClick={onCancel}
            className="px-3 py-1 rounded-lg text-sm text-muted-foreground hover:bg-muted transition-colors"
          >
            취소
          </button>
        </div>

        <div className="flex items-center gap-2">
          {primaryAction && (
            <button
              onClick={primaryAction.onClick}
              disabled={primaryAction.disabled || primaryAction.loading}
              className={`px-4 py-2 rounded-xl text-white text-sm font-medium transition-shadow flex items-center gap-2 ${
                primaryAction.disabled || primaryAction.loading
                  ? "bg-gray-400 cursor-not-allowed"
                  : "bg-gradient-to-r from-primary to-chart-2 hover:shadow-lg"
              }`}
            >
              {primaryAction.icon}
              {primaryAction.label}
            </button>
          )}

          {onSend && (
            connectedPeersCount > 0 ? (
              <button
                onClick={onSend}
                className="px-4 py-2 rounded-xl bg-gradient-to-r from-primary to-chart-2 text-white text-sm font-medium hover:shadow-lg transition-shadow flex items-center gap-2"
              >
                <Send className="w-4 h-4" />
                전송
              </button>
            ) : (
              <span className="text-sm text-muted-foreground">연결된 기기 없음</span>
            )
          )}
        </div>
      </div>
    </div>
  );
}
