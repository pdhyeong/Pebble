import { X, Trash2 } from "lucide-react";
import { TransferItemCard } from "./TransferItemCard";
import type { TransferItem } from "./hooks/useTransferQueue";

interface TransferQueuePanelProps {
    queue: TransferItem[];
    stats: {
        total: number;
        waiting: number;
        uploading: number;
        complete: number;
        error: number;
    };
    onCancel: (id: string) => void;
    onClearCompleted: () => void;
    onClose: () => void;
}

export function TransferQueuePanel({
    queue,
    stats,
    onCancel,
    onClearCompleted,
    onClose,
}: TransferQueuePanelProps) {
    if (queue.length === 0) return null;

    const overallProgress =
        queue.length > 0
            ? Math.round(
                queue.reduce((sum, item) => sum + item.progress, 0) / queue.length
            )
            : 0;

    return (
        <div className="fixed bottom-4 right-4 w-96 max-h-[500px] bg-white dark:bg-gray-800 rounded-2xl shadow-2xl border border-border/50 flex flex-col animate-in slide-in-from-bottom-4 duration-300 z-40">
            {/* Header */}
            <div className="p-4 border-b border-border/50">
                <div className="flex items-center justify-between mb-2">
                    <h3 className="font-semibold text-lg">전송 큐</h3>
                    <button
                        onClick={onClose}
                        className="p-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
                    >
                        <X className="w-5 h-5" />
                    </button>
                </div>

                {/* Stats */}
                <div className="flex items-center gap-4 text-sm">
                    <span className="text-gray-600 dark:text-gray-400">
                        전체: {stats.total}
                    </span>
                    {stats.uploading > 0 && (
                        <span className="text-blue-600">업로드 중: {stats.uploading}</span>
                    )}
                    {stats.waiting > 0 && (
                        <span className="text-gray-500">대기: {stats.waiting}</span>
                    )}
                    {stats.complete > 0 && (
                        <span className="text-green-600">완료: {stats.complete}</span>
                    )}
                    {stats.error > 0 && (
                        <span className="text-red-600">실패: {stats.error}</span>
                    )}
                </div>

                {/* Overall Progress */}
                <div className="mt-3">
                    <div className="flex items-center justify-between text-xs text-gray-600 dark:text-gray-400 mb-1">
                        <span>전체 진행률</span>
                        <span>{overallProgress}%</span>
                    </div>
                    <div className="w-full h-2 bg-gray-200 dark:bg-gray-700 rounded-full overflow-hidden">
                        <div
                            className="h-full bg-gradient-to-r from-blue-600 to-cyan-600 transition-all duration-300"
                            style={{ width: `${overallProgress}%` }}
                        />
                    </div>
                </div>
            </div>

            {/* Queue List */}
            <div className="flex-1 overflow-y-auto p-4 space-y-2">
                {queue.map((item) => (
                    <TransferItemCard key={item.id} item={item} onCancel={onCancel} />
                ))}
            </div>

            {/* Footer */}
            {stats.complete > 0 && (
                <div className="p-3 border-t border-border/50">
                    <button
                        onClick={onClearCompleted}
                        className="w-full px-4 py-2 bg-gray-100 dark:bg-gray-700 hover:bg-gray-200 dark:hover:bg-gray-600 rounded-xl text-sm font-medium transition-colors flex items-center justify-center gap-2"
                    >
                        <Trash2 className="w-4 h-4" />
                        완료된 항목 제거
                    </button>
                </div>
            )}
        </div>
    );
}
