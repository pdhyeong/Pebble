import { X, Upload, Check, AlertCircle, Loader2 } from "lucide-react";
import type { TransferItem } from "./hooks/useTransferQueue";

interface TransferItemCardProps {
    item: TransferItem;
    onCancel: (id: string) => void;
}

export function TransferItemCard({ item, onCancel }: TransferItemCardProps) {
    const formatSize = (bytes: number): string => {
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

    const getStatusIcon = () => {
        switch (item.status) {
            case "waiting":
                return <Loader2 className="w-4 h-4 text-gray-400" />;
            case "uploading":
                return <Upload className="w-4 h-4 text-blue-600 animate-pulse" />;
            case "complete":
                return <Check className="w-4 h-4 text-green-600" />;
            case "error":
                return <AlertCircle className="w-4 h-4 text-red-600" />;
        }
    };

    const getStatusText = () => {
        switch (item.status) {
            case "waiting":
                return "대기 중";
            case "uploading":
                return `${item.progress}% · ${formatSpeed(item.speed)}`;
            case "complete":
                return "완료";
            case "error":
                return item.error || "실패";
        }
    };

    const getStatusColor = () => {
        switch (item.status) {
            case "waiting":
                return "bg-gray-50 dark:bg-gray-900/50";
            case "uploading":
                return "bg-blue-50 dark:bg-blue-900/20";
            case "complete":
                return "bg-green-50 dark:bg-green-900/20";
            case "error":
                return "bg-red-50 dark:bg-red-900/20";
        }
    };

    return (
        <div
            className={`p-3 rounded-xl ${getStatusColor()} border border-border/50 transition-all`}
        >
            <div className="flex items-start gap-3">
                {/* Status Icon */}
                <div className="flex-shrink-0 mt-0.5">{getStatusIcon()}</div>

                {/* File Info */}
                <div className="flex-1 min-w-0">
                    <p className="text-sm font-medium truncate">{item.fileName}</p>
                    <p className="text-xs text-gray-500 mt-0.5">
                        {formatSize(item.fileSize)}
                    </p>

                    {/* Progress Bar */}
                    {item.status === "uploading" && (
                        <div className="mt-2">
                            <div className="w-full h-1.5 bg-gray-200 dark:bg-gray-700 rounded-full overflow-hidden">
                                <div
                                    className="h-full bg-blue-600 transition-all duration-300"
                                    style={{ width: `${item.progress}%` }}
                                />
                            </div>
                        </div>
                    )}

                    {/* Status Text */}
                    <p
                        className={`text-xs mt-1.5 ${item.status === "error"
                                ? "text-red-600"
                                : item.status === "complete"
                                    ? "text-green-600"
                                    : "text-gray-600 dark:text-gray-400"
                            }`}
                    >
                        {getStatusText()}
                    </p>
                </div>

                {/* Cancel Button */}
                {(item.status === "waiting" || item.status === "uploading") && (
                    <button
                        onClick={() => onCancel(item.id)}
                        className="flex-shrink-0 p-1.5 hover:bg-gray-200 dark:hover:bg-gray-700 rounded-lg transition-colors"
                    >
                        <X className="w-4 h-4" />
                    </button>
                )}
            </div>
        </div>
    );
}
