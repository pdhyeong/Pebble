import { motion } from "motion/react";
import { Check, X, AlertCircle } from "lucide-react";
import { useEffect, useState } from "react";

interface TransferResultToastProps {
    isOpen: boolean;
    totalFiles: number;
    successCount: number;
    failedCount: number;
    failedFiles: string[];
    onClose: () => void;
}

export function TransferResultToast({
    isOpen,
    totalFiles,
    successCount,
    failedCount,
    failedFiles,
    onClose,
}: TransferResultToastProps) {
    const [autoCloseTimer, setAutoCloseTimer] = useState(5);

    useEffect(() => {
        if (!isOpen) return;

        const interval = setInterval(() => {
            setAutoCloseTimer((prev) => {
                if (prev <= 1) {
                    clearInterval(interval);
                    onClose();
                    return 0;
                }
                return prev - 1;
            });
        }, 1000);

        return () => clearInterval(interval);
    }, [isOpen, onClose]);

    if (!isOpen) return null;

    const isSuccess = failedCount === 0;

    return (
        <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: 20 }}
            className="fixed bottom-4 right-4 w-96 bg-white dark:bg-gray-800 rounded-2xl shadow-2xl border border-border/50 p-4 z-50"
        >
            <div className="flex items-start gap-3">
                {/* Icon */}
                <div
                    className={`w-10 h-10 rounded-xl flex items-center justify-center flex-shrink-0 shadow-md ${isSuccess
                            ? "bg-gradient-to-br from-green-400 to-emerald-400"
                            : "bg-gradient-to-br from-orange-400 to-red-400"
                        }`}
                >
                    {isSuccess ? (
                        <Check className="w-5 h-5 text-white" />
                    ) : (
                        <AlertCircle className="w-5 h-5 text-white" />
                    )}
                </div>

                {/* Content */}
                <div className="flex-1 min-w-0">
                    <div className="flex items-start justify-between gap-2 mb-2">
                        <div>
                            <p className="text-sm font-semibold">
                                {isSuccess ? "전송 완료" : "전송 완료 (일부 실패)"}
                            </p>
                            <p className="text-xs text-muted-foreground mt-0.5">
                                {autoCloseTimer}초 후 자동으로 닫힙니다
                            </p>
                        </div>
                        <button
                            onClick={onClose}
                            className="p-1.5 hover:bg-muted/50 rounded-lg transition-colors flex-shrink-0"
                        >
                            <X className="w-4 h-4" />
                        </button>
                    </div>

                    {/* Stats */}
                    <div className="space-y-1 text-sm">
                        <div className="flex items-center justify-between">
                            <span className="text-muted-foreground">전체</span>
                            <span className="font-medium">{totalFiles}개</span>
                        </div>
                        <div className="flex items-center justify-between">
                            <span className="text-green-600">성공</span>
                            <span className="font-medium text-green-600">{successCount}개</span>
                        </div>
                        {failedCount > 0 && (
                            <div className="flex items-center justify-between">
                                <span className="text-red-600">실패</span>
                                <span className="font-medium text-red-600">{failedCount}개</span>
                            </div>
                        )}
                    </div>

                    {/* Failed Files */}
                    {failedFiles.length > 0 && (
                        <div className="mt-3 p-2 bg-red-50 dark:bg-red-900/20 rounded-lg">
                            <p className="text-xs font-medium text-red-600 mb-1">실패한 파일:</p>
                            <ul className="text-xs text-red-600 space-y-0.5">
                                {failedFiles.map((file, idx) => (
                                    <li key={idx} className="truncate">
                                        • {file}
                                    </li>
                                ))}
                            </ul>
                        </div>
                    )}
                </div>
            </div>
        </motion.div>
    );
}
