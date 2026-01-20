import { X, Send } from "lucide-react";

interface DroppedFile {
    name: string;
    path: string;
    size: number;
    type: string;
}

interface DropConfirmModalProps {
    isOpen: boolean;
    files: DroppedFile[];
    peerName: string;
    onConfirm: () => void;
    onCancel: () => void;
}

export function DropConfirmModal({
    isOpen,
    files,
    peerName,
    onConfirm,
    onCancel,
}: DropConfirmModalProps) {
    if (!isOpen) return null;

    const totalSize = files.reduce((sum, f) => sum + f.size, 0);

    const formatSize = (bytes: number): string => {
        if (bytes === 0) return "0 B";
        const k = 1024;
        const sizes = ["B", "KB", "MB", "GB"];
        const i = Math.floor(Math.log(bytes) / Math.log(k));
        return `${(bytes / Math.pow(k, i)).toFixed(1)} ${sizes[i]}`;
    };

    const getFileIcon = (fileName: string) => {
        const ext = fileName.split(".").pop()?.toLowerCase();
        const imageExts = ["jpg", "jpeg", "png", "gif", "webp", "svg"];
        const videoExts = ["mp4", "mov", "avi", "mkv"];
        const docExts = ["pdf", "doc", "docx", "txt"];

        if (imageExts.includes(ext || "")) return "🖼️";
        if (videoExts.includes(ext || "")) return "🎬";
        if (docExts.includes(ext || "")) return "📄";
        return "📎";
    };

    return (
        <div className="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-50 p-4 animate-in fade-in duration-200">
            <div className="bg-white dark:bg-gray-800 rounded-2xl shadow-2xl max-w-md w-full p-6 relative animate-in zoom-in-95 duration-200">
                {/* Close button */}
                <button
                    onClick={onCancel}
                    className="absolute top-4 right-4 p-2 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
                >
                    <X className="w-5 h-5" />
                </button>

                {/* Header */}
                <div className="flex items-center gap-3 mb-4">
                    <div className="w-12 h-12 rounded-xl bg-gradient-to-br from-blue-400 to-cyan-400 flex items-center justify-center">
                        <Send className="w-6 h-6 text-white" />
                    </div>
                    <div>
                        <h2 className="text-xl font-bold">파일 전송 확인</h2>
                        <p className="text-sm text-gray-600 dark:text-gray-400">
                            {peerName}에게 전송
                        </p>
                    </div>
                </div>

                {/* File list */}
                <div className="mb-4">
                    <p className="text-sm text-gray-600 dark:text-gray-400 mb-3">
                        다음 파일을 전송하시겠습니까?
                    </p>

                    <div className="max-h-64 overflow-y-auto space-y-2 bg-gray-50 dark:bg-gray-900/50 rounded-xl p-3">
                        {files.map((file, idx) => (
                            <div
                                key={idx}
                                className="flex items-center gap-3 p-2 rounded-lg hover:bg-white dark:hover:bg-gray-800 transition-colors"
                            >
                                <span className="text-2xl flex-shrink-0">
                                    {getFileIcon(file.name)}
                                </span>
                                <div className="flex-1 min-w-0">
                                    <p className="text-sm font-medium truncate">{file.name}</p>
                                    <p className="text-xs text-gray-500">{formatSize(file.size)}</p>
                                </div>
                            </div>
                        ))}
                    </div>
                </div>

                {/* Summary */}
                <div className="mb-6 p-3 rounded-xl bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800">
                    <div className="flex items-center justify-between text-sm">
                        <span className="text-gray-700 dark:text-gray-300">
                            총 {files.length}개 파일
                        </span>
                        <span className="font-semibold text-blue-600 dark:text-blue-400">
                            {formatSize(totalSize)}
                        </span>
                    </div>
                </div>

                {/* Actions */}
                <div className="flex gap-3">
                    <button
                        onClick={onCancel}
                        className="flex-1 px-4 py-3 bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300 rounded-xl font-medium hover:bg-gray-200 dark:hover:bg-gray-600 transition-colors"
                    >
                        취소
                    </button>
                    <button
                        onClick={onConfirm}
                        className="flex-1 px-4 py-3 bg-gradient-to-r from-blue-600 to-cyan-600 text-white rounded-xl font-medium hover:shadow-lg transition-shadow flex items-center justify-center gap-2"
                    >
                        <Send className="w-4 h-4" />
                        전송 시작
                    </button>
                </div>
            </div>
        </div>
    );
}
