import { useState, useCallback, useEffect, useRef } from "react";
import { onEvent } from "../../../adapters/tauri/events";

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
    transferId?: string;
}

type Outcome = { status: "complete" } | { status: "error"; error: string };

type UploadFn = (peerId: string, filePath: string, remotePath: string) => Promise<string | null>;
type CancelFn = (transferId: string) => Promise<void>;

const MAX_CONCURRENT = 3;

function applyOutcome(item: TransferItem, outcome: Outcome): TransferItem {
    return outcome.status === "complete"
        ? { ...item, status: "complete", progress: 100 }
        : { ...item, status: "error", error: outcome.error };
}

export function useTransferQueue(upload: UploadFn, cancel: CancelFn) {
    const [queue, setQueue] = useState<TransferItem[]>([]);
    const earlyOutcomes = useRef(new Map<string, Outcome>());

    const updateItem = useCallback((id: string, update: (item: TransferItem) => TransferItem) => {
        setQueue((prev) => prev.map((item) => (item.id === id ? update(item) : item)));
    }, []);

    const settleByTransferId = useCallback((transferId: string, outcome: Outcome) => {
        setQueue((prev) => {
            if (!prev.some((item) => item.transferId === transferId)) {
                earlyOutcomes.current.set(transferId, outcome);
                return prev;
            }
            return prev.map((item) => (item.transferId === transferId ? applyOutcome(item, outcome) : item));
        });
    }, []);

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
        },
        []
    );

    useEffect(() => {
        const uploading = queue.filter((q) => q.status === "uploading").length;
        const next = queue.filter((q) => q.status === "waiting").slice(0, Math.max(0, MAX_CONCURRENT - uploading));
        if (next.length === 0) return;

        const startIds = new Set(next.map((item) => item.id));
        setQueue((prev) =>
            prev.map((item) =>
                startIds.has(item.id) ? { ...item, status: "uploading", startTime: Date.now() } : item
            )
        );

        for (const item of next) {
            upload(item.peerId, item.filePath, item.remotePath).then((transferId) => {
                if (!transferId) {
                    updateItem(item.id, (current) =>
                        applyOutcome(current, { status: "error", error: "업로드 요청 실패" })
                    );
                    return;
                }
                const early = earlyOutcomes.current.get(transferId);
                earlyOutcomes.current.delete(transferId);
                updateItem(item.id, (current) => {
                    const linked = { ...current, transferId };
                    return early ? applyOutcome(linked, early) : linked;
                });
            });
        }
    }, [queue, upload, updateItem]);

    useEffect(() => {
        const stops = [
            onEvent("transfer-progress", (p) => {
                if (p.direction !== "upload") return;
                setQueue((prev) =>
                    prev.map((item) =>
                        item.transferId === p.transfer_id && item.status === "uploading"
                            ? { ...item, progress: Math.round(p.progress) }
                            : item
                    )
                );
            }),
            onEvent("transfer-completed", (p) => {
                if (p.direction === "upload") settleByTransferId(p.transfer_id, { status: "complete" });
            }),
            onEvent("transfer-failed", (p) => {
                if (p.direction === "upload") settleByTransferId(p.transfer_id, { status: "error", error: p.error });
            }),
            onEvent("transfer-cancelled", (p) => {
                if (p.direction === "upload") settleByTransferId(p.transfer_id, { status: "error", error: "취소됨" });
            }),
        ];
        return () => stops.forEach((stop) => stop());
    }, [settleByTransferId]);

    // 전송 취소
    const cancelTransfer = useCallback((id: string) => {
        setQueue((prev) => {
            const target = prev.find((item) => item.id === id);
            if (target?.transferId && target.status === "uploading") {
                void cancel(target.transferId);
            }
            return prev.filter((item) => item.id !== id);
        });
    }, [cancel]);

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
    };

    return {
        queue,
        stats,
        addFiles,
        cancelTransfer,
        clearCompleted,
    };
}
