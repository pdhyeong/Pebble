import { useState } from "react";

interface ActiveTransfer {
    type: "upload" | "download";
    fileName: string;
    progress: number;
    bytesTransferred: number;
    totalBytes: number;
    speed: number;
}

export function useActiveTransfer() {
    const [activeTransfer, setActiveTransfer] = useState<ActiveTransfer | null>(null);

    return {
        activeTransfer,
        setActiveTransfer,
        clearActiveTransfer: () => setActiveTransfer(null),
    };
}
