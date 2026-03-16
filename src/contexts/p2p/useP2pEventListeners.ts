// P2P 이벤트 리스너 훅 - P2pContext에서 분리
import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import type {
    DiscoveredPeer,
    ConnectedPeer,
    RemoteFilesData,
    ActiveTransfer,
    PairingRequest,
    SentPairingPin,
} from "./types";

interface P2pEventState {
    setDiscoveredPeers: React.Dispatch<React.SetStateAction<DiscoveredPeer[]>>;
    setConnectedPeers: React.Dispatch<React.SetStateAction<ConnectedPeer[]>>;
    setRemoteFiles: React.Dispatch<React.SetStateAction<RemoteFilesData | null>>;
    setIsLoadingFiles: React.Dispatch<React.SetStateAction<boolean>>;
    setIsDownloading: React.Dispatch<React.SetStateAction<boolean>>;
    setDownloadProgress: React.Dispatch<React.SetStateAction<number>>;
    setIsUploading: React.Dispatch<React.SetStateAction<boolean>>;
    setUploadProgress: React.Dispatch<React.SetStateAction<number>>;
    setActiveTransfers: React.Dispatch<React.SetStateAction<Map<string, ActiveTransfer>>>;
    setPairingRequest: React.Dispatch<React.SetStateAction<PairingRequest | null>>;
    setSentPairingPin: React.Dispatch<React.SetStateAction<SentPairingPin | null>>;
    setIsP2pRunning: React.Dispatch<React.SetStateAction<boolean>>;
    setIsStopping: React.Dispatch<React.SetStateAction<boolean>>;
    addLog: (message: string) => void;
}

export function useP2pEventListeners(state: P2pEventState) {
    const {
        setDiscoveredPeers,
        setConnectedPeers,
        setRemoteFiles,
        setIsLoadingFiles,
        setIsDownloading,
        setDownloadProgress,
        setIsUploading,
        setUploadProgress,
        setActiveTransfers,
        setPairingRequest,
        setSentPairingPin,
        setIsP2pRunning,
        setIsStopping,
        addLog,
    } = state;

    useEffect(() => {
        const unlisteners: Promise<() => void>[] = [];

        // 피어 발견
        unlisteners.push(
            listen<string>("peer-found", (event) => {
                const payload = event.payload;
                const [peerId, ...addrParts] = payload.split(":");
                const address = addrParts.join(":");

                addLog(`🔍 피어 발견: ${peerId.slice(0, 12)}...`);

                setDiscoveredPeers((prev) => {
                    const exists = prev.find((p) => p.peerId === peerId);
                    if (exists) {
                        return prev.map((p) =>
                            p.peerId === peerId
                                ? { ...p, address, discoveredAt: new Date() }
                                : p
                        );
                    }
                    return [...prev, { peerId, address, discoveredAt: new Date() }];
                });
            })
        );

        // 연결 성공 (페어링 전)
        unlisteners.push(
            listen<string>("connection-success", (event) => {
                const peerId = event.payload;
                addLog(`🔗 전송 계층 연결: ${peerId.slice(0, 12)}... (페어링 필요)`);
            })
        );

        // 기기 정보 수신
        unlisteners.push(
            listen<string>("device-info-received", (event) => {
                try {
                    const data = JSON.parse(event.payload);
                    addLog(`📱 기기 이름 수신: ${data.device_name}`);
                    setConnectedPeers((prev) =>
                        prev.map((p) =>
                            p.peerId === data.peer_id
                                ? { ...p, deviceName: data.device_name }
                                : p
                        )
                    );
                } catch (e) {
                    console.error("기기 정보 파싱 실패:", e);
                }
            })
        );

        // 연결 종료
        unlisteners.push(
            listen<string>("connection-closed", (event) => {
                const peerId = event.payload;
                addLog(`❌ 연결 종료: ${peerId.slice(0, 12)}...`);
                setConnectedPeers((prev) => prev.filter((p) => p.peerId !== peerId));
            })
        );

        unlisteners.push(
            listen<string>("listening-on", (event) => {
                addLog(`📡 리스닝: ${event.payload}`);
            })
        );

        unlisteners.push(
            listen<string>("peer-info", (event) => {
                addLog(`ℹ️ 피어 정보: ${event.payload}`);
            })
        );

        unlisteners.push(
            listen<string>("ping-success", (event) => {
                const [peerId, rttMs] = event.payload.split(":");
                addLog(`🏓 Ping: ${peerId.slice(0, 12)}... (${rttMs}ms)`);
            })
        );

        unlisteners.push(
            listen<string>("dial-started", (event) => {
                addLog(`📞 연결 시도 중: ${event.payload}`);
            })
        );

        unlisteners.push(
            listen<string>("dial-failed", (event) => {
                addLog(`❌ 연결 시도 실패: ${event.payload}`);
            })
        );

        // 원격 파일 목록 수신
        unlisteners.push(
            listen<string>("remote-files", (event) => {
                try {
                    const data: RemoteFilesData = JSON.parse(event.payload);
                    addLog(`📂 파일 목록 수신: ${data.files.length}개 파일`);
                    setRemoteFiles(data);
                    setIsLoadingFiles(false);
                } catch (e) {
                    addLog(`❌ 파일 목록 파싱 실패: ${e}`);
                    setIsLoadingFiles(false);
                }
            })
        );

        unlisteners.push(
            listen<string>("file-list-request-sent", (event) => {
                addLog(`📤 파일 목록 요청 전송: ${event.payload}`);
            })
        );

        unlisteners.push(
            listen<string>("file-list-error", (event) => {
                addLog(`❌ 파일 목록 요청 실패: ${event.payload}`);
                setIsLoadingFiles(false);
            })
        );

        // P2P 종료
        unlisteners.push(
            listen<string>("p2p-stopped", (event) => {
                addLog(`🛑 ${event.payload}`);
                setIsP2pRunning(false);
                setIsStopping(false);
                setConnectedPeers([]);
                setDiscoveredPeers([]);
            })
        );

        // 파일 다운로드 이벤트
        unlisteners.push(
            listen<string>("file-download-started", (event) => {
                try {
                    const data = JSON.parse(event.payload);
                    addLog(`📥 파일 다운로드 시작: ${data.file_name || event.payload}`);
                    setIsDownloading(true);
                    setDownloadProgress(0);

                    const transferId = data.transfer_id || crypto.randomUUID();
                    setActiveTransfers((prev) => {
                        const newMap = new Map(prev);
                        newMap.set(transferId, {
                            type: "download",
                            fileName: data.file_name || "Unknown",
                            progress: 0,
                            bytesTransferred: 0,
                            totalBytes: data.total_size || 0,
                            speed: 0,
                            transferId,
                        });
                        return newMap;
                    });
                } catch {
                    addLog(`📥 파일 다운로드 시작: ${event.payload}`);
                    setIsDownloading(true);
                    setDownloadProgress(0);
                }
            })
        );

        unlisteners.push(
            listen<string>("file-download-progress", (event) => {
                try {
                    const data = JSON.parse(event.payload);
                    const progress = Math.round(data.progress);
                    setDownloadProgress(progress);

                    const transferId = data.transfer_id || "";
                    setActiveTransfers((prev) => {
                        const newMap = new Map(prev);
                        const transfer = newMap.get(transferId);

                        if (transfer) {
                            // Update existing transfer
                            const bytesTransferred = Math.round(
                                (progress / 100) * transfer.totalBytes
                            );
                            newMap.set(transferId, {
                                ...transfer,
                                progress,
                                bytesTransferred,
                            });
                        } else if (transferId) {
                            // Create new transfer on first progress event
                            newMap.set(transferId, {
                                type: "download",
                                fileName: data.file_name || "Downloading...",
                                progress,
                                bytesTransferred: 0,
                                totalBytes: 0, // Will be calculated from chunks
                                speed: 0,
                                transferId,
                            });
                        }
                        return newMap;
                    });
                } catch { }
            })
        );

        unlisteners.push(
            listen<string>("file-download-complete", (event) => {
                try {
                    const data = JSON.parse(event.payload);
                    addLog(`✅ 파일 다운로드 완료: ${data.saved_path}`);
                    setIsDownloading(false);
                    setDownloadProgress(100);

                    const transferId = data.transfer_id || "";
                    setTimeout(() => {
                        setDownloadProgress(0);
                        setActiveTransfers((prev) => {
                            const newMap = new Map(prev);
                            newMap.delete(transferId);
                            return newMap;
                        });
                    }, 1000);
                } catch {
                    addLog(`✅ 파일 다운로드 완료`);
                    setIsDownloading(false);
                    setDownloadProgress(0);
                }
            })
        );

        unlisteners.push(
            listen<string>("file-download-error", (event) => {
                try {
                    const data = JSON.parse(event.payload);
                    addLog(`❌ 파일 다운로드 실패: ${data.error || "Unknown"}`);
                    setIsDownloading(false);
                    setDownloadProgress(0);
                    
                    const transferId = data.transfer_id || "";
                    if (transferId) {
                        setActiveTransfers((prev) => {
                            const newMap = new Map(prev);
                            newMap.delete(transferId);
                            return newMap;
                        });
                    }
                } catch (e) {
                    addLog(`❌ 파일 다운로드 실패: ${event.payload}`);
                    setIsDownloading(false);
                    setDownloadProgress(0);
                }
            })
        );

        // 파일 업로드 이벤트
        unlisteners.push(
            listen<string>("file-upload-started", (event) => {
                try {
                    const data = JSON.parse(event.payload);
                    addLog(`📤 파일 업로드 시작: ${data.file_name}`);
                    setIsUploading(true);
                    setUploadProgress(0);

                    const transferId = data.transfer_id || "";
                    setActiveTransfers((prev) => {
                        const newMap = new Map(prev);
                        newMap.set(transferId, {
                            type: "upload",
                            fileName: data.file_name || "Unknown",
                            progress: 0,
                            bytesTransferred: 0,
                            totalBytes: data.total_size || 0,
                            speed: 0,
                            transferId,
                        });
                        return newMap;
                    });
                } catch (e) {
                    setIsUploading(true);
                    setUploadProgress(0);
                    console.error("upload started event parse error", e);
                }
            })
        );

        unlisteners.push(
            listen<string>("file-upload-progress", (event) => {
                try {
                    const data = JSON.parse(event.payload);
                    const progress = Math.round(data.progress);
                    setUploadProgress(progress);

                    const transferId = data.transfer_id || "";
                    setActiveTransfers((prev) => {
                        const newMap = new Map(prev);
                        const transfer = newMap.get(transferId);
                        if (transfer) {
                            const bytesTransferred = Math.round(
                                (progress / 100) * transfer.totalBytes
                            );
                            newMap.set(transferId, {
                                ...transfer,
                                progress,
                                bytesTransferred,
                            });
                        }
                        return newMap;
                    });
                } catch { }
            })
        );

        unlisteners.push(
            listen<string>("file-upload-complete", (event) => {
                try {
                    const data = JSON.parse(event.payload);
                    addLog(`✅ 파일 업로드 완료: ${data.file_name}`);
                    setIsUploading(false);
                    setUploadProgress(100);

                    const transferId = data.transfer_id || "";
                    setTimeout(() => {
                        setUploadProgress(0);
                        setActiveTransfers((prev) => {
                            const newMap = new Map(prev);
                            newMap.delete(transferId);
                            return newMap;
                        });
                    }, 1000);
                } catch (e) {
                    setIsUploading(false);
                    setUploadProgress(0);
                    console.error("upload complete event parse error", e);
                }
            })
        );

        unlisteners.push(
            listen<string>("file-upload-error", (event) => {
                try {
                    const data = JSON.parse(event.payload);
                    addLog(`❌ 파일 업로드 실패: ${data.error || "Unknown"}`);
                    setIsUploading(false);
                    setUploadProgress(0);
                    
                    const transferId = data.transfer_id || "";
                    if (transferId) {
                        setActiveTransfers((prev) => {
                            const newMap = new Map(prev);
                            newMap.delete(transferId);
                            return newMap;
                        });
                    }
                } catch (e) {
                    addLog(`❌ 파일 업로드 실패: ${event.payload}`);
                    setIsUploading(false);
                    setUploadProgress(0);
                }
            })
        );

        // 파일 수신
        unlisteners.push(
            listen<string>("file-received", (event) => {
                try {
                    const data = JSON.parse(event.payload);
                    if (data.success) {
                        addLog(`📥 파일 수신: ${data.file_name} -> ${data.saved_path}`);
                    } else {
                        addLog(`❌ 파일 수신 실패: ${data.error}`);
                    }
                } catch {
                    addLog(`📥 파일 수신 완료`);
                }
            })
        );

        // 페어링 이벤트
        unlisteners.push(
            listen<string>("pairing-request", (event) => {
                try {
                    const data = JSON.parse(event.payload);
                    addLog(`🔐 페어링 요청: ${data.device_name} (PIN: ${data.pin})`);
                    setPairingRequest({
                        peerId: data.peer_id,
                        deviceName: data.device_name,
                        pin: data.pin,
                    });
                } catch (e) {
                    console.error("페어링 요청 파싱 실패:", e);
                }
            })
        );

        unlisteners.push(
            listen<string>("pairing-approved", (event) => {
                try {
                    const data = JSON.parse(event.payload);
                    console.log("🔍 Pairing approved data:", data);
                    console.log("🔍 Shared folder name:", data.shared_folder_name);
                    addLog(`✅ 페어링 승인: ${data.device_name}`);
                    setConnectedPeers((prev) => {
                        if (prev.find((p) => p.peerId === data.peer_id)) return prev;
                        return [
                            ...prev,
                            {
                                peerId: data.peer_id,
                                deviceName: data.device_name,
                                connectedAt: new Date(),
                                sharedFolderName: data.shared_folder_name || undefined,
                            },
                        ];
                    });
                    setSentPairingPin(null);
                } catch (e) {
                    console.error("페어링 승인 파싱 실패:", e);
                }
            })
        );

        unlisteners.push(
            listen<string>("pairing-rejected", (event) => {
                try {
                    const data = JSON.parse(event.payload);
                    addLog(`❌ 페어링 거절: ${data.error || "거절됨"}`);
                    setSentPairingPin(null);
                } catch { }
            })
        );

        unlisteners.push(
            listen<string>("pairing-sent", (event) => {
                try {
                    const data = JSON.parse(event.payload);
                    addLog(`🔐 페어링 요청 전송 (PIN: ${data.pin})`);
                    setSentPairingPin({ peerId: data.peer_id, pin: data.pin });
                } catch { }
            })
        );

        return () => {
            unlisteners.forEach((p) => p.then((fn) => fn()));
        };
    }, [
        addLog,
        setDiscoveredPeers,
        setConnectedPeers,
        setRemoteFiles,
        setIsLoadingFiles,
        setIsDownloading,
        setDownloadProgress,
        setIsUploading,
        setUploadProgress,
        setActiveTransfers,
        setPairingRequest,
        setSentPairingPin,
        setIsP2pRunning,
        setIsStopping,
    ]);
}
