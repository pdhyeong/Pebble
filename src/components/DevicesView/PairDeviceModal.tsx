import { motion } from "motion/react";
import { QrCode, CheckCircle2 } from "lucide-react";

interface PairDeviceModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export function PairDeviceModal({ isOpen, onClose }: PairDeviceModalProps) {
  if (!isOpen) return null;

  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      className="fixed inset-0 bg-black/50 backdrop-blur-sm z-50 flex items-center justify-center p-4"
      onClick={onClose}
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
            <p className="text-sm text-muted-foreground mb-2">
              또는 페어링 코드 입력
            </p>
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
            onClick={onClose}
            className="w-full px-6 py-3 rounded-xl bg-muted/50 hover:bg-muted transition-colors font-medium"
          >
            취소
          </button>
        </div>
      </motion.div>
    </motion.div>
  );
}
