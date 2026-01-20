import { useState, useCallback } from "react";

export interface TransferItem {
    id: string;
    fileName: string;
    filePath: string;
    fileSize: number;
    peerId: string;
    remotePath: string;
    progress: number;
    speed: number;
    status: "waiting" | "uploading" | "complete" | "error";
    error?: string;
    startTime?: number;
}

const MAX_CONCURRENT = 3;

export function useTransferQueue() {
    const [queue, setQueue] = useState<TransferItem[]>([]);

    // 큐에 파일 추가
    const addFiles = useCallback(
        (
            files: { name: string; path: string; size: number }[],
            peerId: string,
            remotePath: string
        ) => {
            const newItems: TransferItem[] = files.map((f) => ({
                id: `${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
                fileName: f.name,
                filePath: f.path,
                fileSize: f.size,
                peerId,
                remotePath,
                progress: 0,
                speed: 0,
                status: "waiting",
            }));

            setQueue((prev) => [...prev, ...newItems]);
            return newItems.map((item) => item.id);
        },
        []
    );

    // 전송 시작
    const startTransfer = useCallback((id: string) => {
        setQueue((prev) =>
            prev.map((item) =>
                item.id === id
                    ? { ...item, status: "uploading", startTime: Date.now() }
                    : item
            )
        );
    }, []);

    // 진행률 업데이트
    const updateProgress = useCallback(
        (id: string, progress: number, speed: number) => {
            setQueue((prev) =>
                prev.map((item) =>
                    item.id === id ? { ...item, progress, speed } : item
                )
            );
        },
        []
    );

    // 전송 완료
    const completeTransfer = useCallback((id: string) => {
        setQueue((prev) =>
            prev.map((item) =>
                item.id === id ? { ...item, status: "complete", progress: 100 } : item
            )
        );
    }, []);

    // 전송 실패
    const failTransfer = useCallback((id: string, error: string) => {
        setQueue((prev) =>
            prev.map((item) =>
                item.id === id ? { ...item, status: "error", error } : item
            )
        );
    }, []);

    // 전송 취소
    const cancelTransfer = useCallback((id: string) => {
        setQueue((prev) => prev.filter((item) => item.id !== id));
    }, []);

    // 큐 초기화
    const clearQueue = useCallback(() => {
        setQueue([]);
    }, []);

    // 완료된 항목 제거
    const clearCompleted = useCallback(() => {
        setQueue((prev) => prev.filter((item) => item.status !== "complete"));
    }, []);

    // 통계
    const stats = {
        total: queue.length,
        waiting: queue.filter((q) => q.status === "waiting").length,
        uploading: queue.filter((q) => q.status === "uploading").length,
        complete: queue.filter((q) => q.status === "complete").length,
        error: queue.filter((q) => q.status === "error").length,
        canStartMore:
            queue.filter((q) => q.status === "uploading").length < MAX_CONCURRENT,
    };

    return {
        queue,
        stats,
        addFiles,
        startTransfer,
        updateProgress,
        completeTransfer,
        failTransfer,
        cancelTransfer,
        clearQueue,
        clearCompleted,
    };
}
