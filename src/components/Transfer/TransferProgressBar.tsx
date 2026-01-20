import { motion, AnimatePresence } from "motion/react";
import { Upload, Download, X } from "lucide-react";
import { useEffect, useState } from "react";

interface TransferProgressBarProps {
    type: "upload" | "download";
    fileName: string;
    progress: number;
    bytesTransferred: number;
    totalBytes: number;
    speed: number; // bytes per second
    onCancel?: () => void;
    transferId?: string;
    isStacked?: boolean;  // 여러 전송이 있을 때
    stackIndex?: number;  // 스택에서의 위치
}

export function TransferProgressBar({
    type,
    fileName,
    progress,
    bytesTransferred,
    totalBytes,
    speed,
    onCancel,
    transferId: _transferId,
    isStacked = false,
    stackIndex: _stackIndex = 0,
}: TransferProgressBarProps) {
    const [timeRemaining, setTimeRemaining] = useState<number>(0);

    // 남은 시간 계산
    useEffect(() => {
        if (speed > 0 && totalBytes > bytesTransferred) {
            const remaining = (totalBytes - bytesTransferred) / speed;
            setTimeRemaining(remaining);
        } else {
            setTimeRemaining(0);
        }
    }, [speed, bytesTransferred, totalBytes]);

    const formatBytes = (bytes: number): string => {
        if (bytes === 0) return "0 B";
        const k = 1024;
        const sizes = ["B", "KB", "MB", "GB"];
        const i = Math.floor(Math.log(bytes) / Math.log(k));
        return `${(bytes / Math.pow(k, i)).toFixed(1)} ${sizes[i]}`;
    };

    const formatSpeed = (bytesPerSecond: number): string => {
        if (bytesPerSecond === 0) return "0 B/s";
        const k = 1024;
        const sizes = ["B/s", "KB/s", "MB/s", "GB/s"];
        const i = Math.floor(Math.log(bytesPerSecond) / Math.log(k));
        return `${(bytesPerSecond / Math.pow(k, i)).toFixed(1)} ${sizes[i]}`;
    };

    const formatTime = (seconds: number): string => {
        if (seconds < 60) return `${Math.round(seconds)}초`;
        if (seconds < 3600) return `${Math.round(seconds / 60)}분`;
        return `${Math.round(seconds / 3600)}시간`;
    };

    const Icon = type === "upload" ? Upload : Download;
    const gradient =
        type === "upload"
            ? "from-green-400 to-emerald-400"
            : "from-blue-400 to-cyan-400";
    const bgGradient =
        type === "upload"
            ? "from-green-500/10 to-emerald-500/10"
            : "from-blue-500/10 to-cyan-500/10";
    const borderColor =
        type === "upload" ? "border-green-500/20" : "border-blue-500/20";

    // 스택 레이아웃일 때는 relative, 아닐 때는 fixed
    const positionClass = isStacked
        ? "relative"
        : "fixed bottom-20 left-4 right-4";

    return (
        <AnimatePresence>
            <motion.div
                initial={{ opacity: 0, y: 20 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0, y: 20 }}
                layout
                className={`${positionClass} mx-auto max-w-2xl bg-gradient-to-br ${bgGradient} backdrop-blur-xl border ${borderColor} rounded-2xl shadow-2xl p-3 z-50`}
            >
                <div className="flex items-start gap-3">
                    {/* Icon */}
                    <div
                        className={`w-10 h-10 rounded-xl bg-gradient-to-br ${gradient} flex items-center justify-center flex-shrink-0 shadow-md`}
                    >
                        <Icon className="w-5 h-5 text-white" />
                    </div>

                    {/* Content */}
                    <div className="flex-1 min-w-0">
                        {/* File name and cancel button */}
                        <div className="flex items-start justify-between gap-2 mb-2">
                            <div className="flex-1 min-w-0">
                                <p className="text-sm font-medium truncate">{fileName}</p>
                                <p className="text-xs text-muted-foreground">
                                    {type === "upload" ? "업로드 중" : "다운로드 중"}
                                </p>
                            </div>
                            {onCancel && (
                                <button
                                    onClick={onCancel}
                                    className="p-1.5 hover:bg-muted/50 rounded-lg transition-colors flex-shrink-0"
                                >
                                    <X className="w-4 h-4" />
                                </button>
                            )}
                        </div>

                        {/* Progress bar */}
                        <div className="mb-2">
                            <div className="w-full h-2 bg-muted/50 rounded-full overflow-hidden">
                                <motion.div
                                    initial={{ width: 0 }}
                                    animate={{ width: `${progress}%` }}
                                    transition={{ duration: 0.3 }}
                                    className={`h-full bg-gradient-to-r ${gradient} rounded-full`}
                                />
                            </div>
                        </div>

                        {/* Stats */}
                        <div className="flex items-center justify-between text-xs text-muted-foreground">
                            <span>
                                {formatBytes(bytesTransferred)} / {formatBytes(totalBytes)} (
                                {progress}%)
                            </span>
                            <div className="flex items-center gap-3">
                                <span>{formatSpeed(speed)}</span>
                                {timeRemaining > 0 && (
                                    <>
                                        <span>•</span>
                                        <span>{formatTime(timeRemaining)} 남음</span>
                                    </>
                                )}
                            </div>
                        </div>
                    </div>
                </div>
            </motion.div>
        </AnimatePresence>
    );
}
