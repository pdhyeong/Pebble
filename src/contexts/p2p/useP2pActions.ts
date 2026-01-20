// P2P 액션(API 호출) 훅
import { useCallback } from "react";
import type { DiscoveredPeer, LocalFileInfo, PairingRequest } from "./types";
import {
    startP2pEngine,
    stopP2pEngine,
    connectToPeer as apiConnectToPeer,
    requestFileList as apiRequestFileList,
    downloadFile as apiDownloadFile,
    uploadFile as apiUploadFile,
    requestPairing as apiRequestPairing,
    respondPairing as apiRespondPairing,
    setDeviceName,
    setSharedFolder as apiSetSharedFolder,
    setSharedFolderDisplayName as apiSetSharedFolderDisplayName,
    setShowFolderName as apiSetShowFolderName,
    getLocalSharedFiles,
} from "./api";

interface P2pActionsState {
    discoveredPeers: DiscoveredPeer[];
    addLog: (message: string) => void;
    setIsP2pRunning: React.Dispatch<React.SetStateAction<boolean>>;
    setIsStarting: React.Dispatch<React.SetStateAction<boolean>>;
    setIsStopping: React.Dispatch<React.SetStateAction<boolean>>;
    setIsLoadingFiles: React.Dispatch<React.SetStateAction<boolean>>;
    setIsDownloading: React.Dispatch<React.SetStateAction<boolean>>;
    setIsUploading: React.Dispatch<React.SetStateAction<boolean>>;
    setPairingRequest: React.Dispatch<React.SetStateAction<PairingRequest | null>>;
    setDiscoveredPeers: React.Dispatch<React.SetStateAction<DiscoveredPeer[]>>;
    setLocalFiles: React.Dispatch<React.SetStateAction<LocalFileInfo[]>>;
    setMyDeviceNameState: React.Dispatch<React.SetStateAction<string>>;
    setSharedFolderPathState: React.Dispatch<React.SetStateAction<string>>;
    setSharedFolderDisplayNameState: React.Dispatch<React.SetStateAction<string>>;
    setShowFolderNameState: React.Dispatch<React.SetStateAction<boolean>>;
}

export function useP2pActions(state: P2pActionsState) {
    const {
        discoveredPeers,
        addLog,
        setIsP2pRunning,
        setIsStarting,
        setIsStopping,
        setIsLoadingFiles,
        setIsDownloading,
        setIsUploading,
        setPairingRequest,
        setDiscoveredPeers,
        setLocalFiles,
        setMyDeviceNameState,
        setSharedFolderPathState,
        setSharedFolderDisplayNameState,
        setShowFolderNameState,
    } = state;

    const startP2p = useCallback(async () => {
        setIsStarting(true);
        addLog("🚀 P2P 엔진 시작 중...");

        try {
            const result = await startP2pEngine();
            addLog(result);
            setIsP2pRunning(true);
        } catch (e) {
            addLog(`❌ P2P 시작 실패: ${e}`);
            console.error("P2P 시작 실패:", e);
        } finally {
            setIsStarting(false);
        }
    }, [addLog, setIsP2pRunning, setIsStarting]);

    const stopP2p = useCallback(async () => {
        setIsStopping(true);
        addLog("🛑 P2P 엔진 종료 중...");

        try {
            const result = await stopP2pEngine();
            addLog(result);
        } catch (e) {
            addLog(`❌ P2P 종료 실패: ${e}`);
            console.error("P2P 종료 실패:", e);
            setIsStopping(false);
        }
    }, [addLog, setIsStopping]);

    const connectToPeer = useCallback(
        async (peerAddr: string) => {
            addLog(`📞 연결 및 페어링 시도: ${peerAddr}`);

            try {
                await apiConnectToPeer(peerAddr);

                const peer = discoveredPeers.find((p) => p.address === peerAddr);
                if (peer) {
                    addLog(`🔐 자동 페어링 요청: ${peer.peerId.slice(0, 12)}...`);
                    await apiRequestPairing(peer.peerId);
                }
            } catch (e) {
                addLog(`❌ 연결/페어링 실패: ${e}`);
                console.error("연결/페어링 실패:", e);
            }
        },
        [addLog, discoveredPeers]
    );

    const requestFileList = useCallback(
        async (peerId: string, path: string = "/") => {
            addLog(`📂 파일 목록 요청: ${peerId.slice(0, 12)}... (${path})`);
            setIsLoadingFiles(true);

            try {
                await apiRequestFileList(peerId, path);
            } catch (e) {
                addLog(`❌ 파일 목록 요청 실패: ${e}`);
                console.error("파일 목록 요청 실패:", e);
                setIsLoadingFiles(false);
            }
        },
        [addLog, setIsLoadingFiles]
    );

    const downloadFile = useCallback(
        async (peerId: string, path: string) => {
            addLog(`📥 파일 다운로드 요청: ${path}`);
            setIsDownloading(true);

            try {
                await apiDownloadFile(peerId, path);
            } catch (e) {
                addLog(`❌ 파일 다운로드 요청 실패: ${e}`);
                console.error("파일 다운로드 요청 실패:", e);
                setIsDownloading(false);
            }
        },
        [addLog, setIsDownloading]
    );

    const uploadFile = useCallback(
        async (peerId: string, filePath: string, remotePath: string = "/") => {
            addLog(`📤 파일 업로드 요청: ${filePath} -> ${remotePath}`);
            setIsUploading(true);

            try {
                await apiUploadFile(peerId, filePath, remotePath);
            } catch (e) {
                addLog(`❌ 파일 업로드 요청 실패: ${e}`);
                console.error("파일 업로드 요청 실패:", e);
                setIsUploading(false);
            }
        },
        [addLog, setIsUploading]
    );

    const requestPairing = useCallback(
        async (peerId: string) => {
            addLog(`🔐 페어링 요청: ${peerId.slice(0, 12)}...`);

            try {
                await apiRequestPairing(peerId);
            } catch (e) {
                addLog(`❌ 페어링 요청 실패: ${e}`);
                console.error("페어링 요청 실패:", e);
            }
        },
        [addLog]
    );

    const respondPairing = useCallback(
        async (peerId: string, approved: boolean) => {
            addLog(`${approved ? "✅ 승인" : "❌ 거절"}: ${peerId.slice(0, 12)}...`);

            try {
                await apiRespondPairing(peerId, approved);
                setPairingRequest(null);
            } catch (e) {
                addLog(`❌ 페어링 응답 실패: ${e}`);
                console.error("페어링 응답 실패:", e);
            }
        },
        [addLog, setPairingRequest]
    );

    const clearAll = useCallback(() => {
        setDiscoveredPeers([]);
    }, [setDiscoveredPeers]);

    const setMyDeviceName = useCallback(
        async (name: string) => {
            try {
                await setDeviceName(name);
                setMyDeviceNameState(name);
                addLog(`📱 기기 이름 설정: ${name}`);
            } catch (e) {
                console.error("기기 이름 설정 실패:", e);
            }
        },
        [addLog, setMyDeviceNameState]
    );

    const setSharedFolder = useCallback(
        async (path: string) => {
            try {
                await apiSetSharedFolder(path);
                setSharedFolderPathState(path);
                addLog(`📁 공유 폴더 설정: ${path}`);
                const files = await getLocalSharedFiles("/");
                setLocalFiles(files);
            } catch (e) {
                console.error("공유 폴더 설정 실패:", e);
            }
        },
        [addLog, setSharedFolderPathState, setLocalFiles]
    );

    const setSharedFolderDisplayName = useCallback(
        async (name: string) => {
            await apiSetSharedFolderDisplayName(name);
            setSharedFolderDisplayNameState(name);
        },
        [setSharedFolderDisplayNameState]
    );

    const setShowFolderName = useCallback(
        async (show: boolean) => {
            await apiSetShowFolderName(show);
            setShowFolderNameState(show);
        },
        [setShowFolderNameState]
    );

    const refreshLocalFiles = useCallback(
        async (relativePath: string = "/") => {
            try {
                const files = await getLocalSharedFiles(relativePath);
                setLocalFiles(files);
            } catch (e) {
                console.error("파일 목록 새로고침 실패:", e);
            }
        },
        [setLocalFiles]
    );

    return {
        startP2p,
        stopP2p,
        connectToPeer,
        requestFileList,
        downloadFile,
        uploadFile,
        requestPairing,
        respondPairing,
        clearAll,
        setMyDeviceName,
        setSharedFolder,
        setSharedFolderDisplayName,
        setShowFolderName,
        refreshLocalFiles,
    };
}
