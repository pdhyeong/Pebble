import { useCallback, useState, type Dispatch } from "react";
import * as commands from "../../adapters/tauri/commands";
import { describeError } from "../../adapters/tauri/errors";
import { shortPeerId } from "../../core/p2pEvents";
import type { DiscoveredPeer, P2pAction } from "../../core/p2pState";

export function useP2pActions(dispatch: Dispatch<P2pAction>, discoveredPeers: DiscoveredPeer[]) {
  const [pendingDownloads, setPendingDownloads] = useState(0);
  const [pendingUploads, setPendingUploads] = useState(0);
  const log = useCallback(
    (message: string) => dispatch({ type: "log", message, at: Date.now() }),
    [dispatch],
  );

  const startP2p = useCallback(async () => {
    dispatch({ type: "engine/starting" });
    log("🚀 P2P 엔진 시작 중...");
    try {
      await commands.startP2p();
      dispatch({ type: "engine/started" });
    } catch (e) {
      log(`❌ P2P 시작 실패: ${describeError(e)}`);
    } finally {
      dispatch({ type: "engine/startFinished" });
    }
  }, [dispatch, log]);

  const stopP2p = useCallback(async () => {
    dispatch({ type: "engine/stopping" });
    log("🛑 P2P 엔진 종료 중...");
    try {
      await commands.stopP2p();
    } catch (e) {
      log(`❌ P2P 종료 실패: ${describeError(e)}`);
      dispatch({ type: "engine/stopFailed" });
    }
  }, [dispatch, log]);

  const initialize = useCallback(async () => {
    try {
      const isP2pRunning = await commands.getP2pStatus();
      const myDeviceName = await commands.getDeviceName();
      const sharedFolderPath = await commands.getSharedFolder();
      const sharedFolderDisplayName = await commands.getSharedFolderDisplayName();
      dispatch({
        type: "settings/loaded",
        settings: { isP2pRunning, myDeviceName, sharedFolderPath, sharedFolderDisplayName },
      });
      dispatch({ type: "localFiles/loaded", files: await commands.getLocalSharedFiles("/") });
      if (!isP2pRunning) await startP2p();
    } catch (e) {
      log(`❌ 초기 데이터 로드 실패: ${describeError(e)}`);
    }
  }, [dispatch, log, startP2p]);

  const connectToPeer = useCallback(
    async (peerAddr: string) => {
      log(`📞 연결 및 페어링 시도: ${peerAddr}`);
      try {
        await commands.connectToPeer(peerAddr);
        const peer = discoveredPeers.find((p) => p.address === peerAddr);
        if (peer) {
          log(`🔐 자동 페어링 요청: ${shortPeerId(peer.peerId)}`);
          await commands.requestPairing(peer.peerId);
        }
      } catch (e) {
        log(`❌ 연결/페어링 실패: ${describeError(e)}`);
      }
    },
    [log, discoveredPeers],
  );

  const requestFileList = useCallback(
    async (peerId: string, path: string = "/") => {
      log(`📂 파일 목록 요청: ${shortPeerId(peerId)} (${path})`);
      dispatch({ type: "remoteFiles/loading" });
      try {
        await commands.requestFileList(peerId, path);
      } catch (e) {
        log(`❌ 파일 목록 요청 실패: ${describeError(e)}`);
        dispatch({ type: "remoteFiles/loadFailed" });
      }
    },
    [dispatch, log],
  );

  const downloadFile = useCallback(
    async (peerId: string, path: string): Promise<string | null> => {
      log(`📥 파일 다운로드 요청: ${path}`);
      setPendingDownloads((n) => n + 1);
      try {
        return await commands.downloadFile(peerId, path);
      } catch (e) {
        log(`❌ 파일 다운로드 요청 실패: ${describeError(e)}`);
        return null;
      } finally {
        setPendingDownloads((n) => n - 1);
      }
    },
    [log],
  );

  const uploadFile = useCallback(
    async (peerId: string, filePath: string, remotePath: string = "/"): Promise<string | null> => {
      log(`📤 파일 업로드 요청: ${filePath} -> ${remotePath}`);
      setPendingUploads((n) => n + 1);
      try {
        return await commands.uploadFile(peerId, filePath, remotePath);
      } catch (e) {
        log(`❌ 파일 업로드 요청 실패: ${describeError(e)}`);
        return null;
      } finally {
        setPendingUploads((n) => n - 1);
      }
    },
    [log],
  );

  const cancelTransfer = useCallback(
    async (transferId: string) => {
      try {
        await commands.cancelTransfer(transferId);
      } catch (e) {
        log(`❌ 전송 취소 실패: ${describeError(e)}`);
      }
    },
    [log],
  );

  const requestPairing = useCallback(
    async (peerId: string) => {
      log(`🔐 페어링 요청: ${shortPeerId(peerId)}`);
      try {
        await commands.requestPairing(peerId);
      } catch (e) {
        log(`❌ 페어링 요청 실패: ${describeError(e)}`);
      }
    },
    [log],
  );

  const respondPairing = useCallback(
    async (peerId: string, approved: boolean) => {
      log(`${approved ? "✅ 승인" : "❌ 거절"}: ${shortPeerId(peerId)}`);
      try {
        await commands.respondPairing(peerId, approved);
        dispatch({ type: "pairing/requestCleared" });
      } catch (e) {
        log(`❌ 페어링 응답 실패: ${describeError(e)}`);
      }
    },
    [dispatch, log],
  );

  const clearAll = useCallback(() => dispatch({ type: "peers/cleared" }), [dispatch]);

  const setMyDeviceName = useCallback(
    async (name: string) => {
      try {
        await commands.setDeviceName(name);
        dispatch({ type: "settings/deviceName", name });
        log(`📱 기기 이름 설정: ${name}`);
      } catch (e) {
        log(`❌ 기기 이름 설정 실패: ${describeError(e)}`);
      }
    },
    [dispatch, log],
  );

  const setSharedFolder = useCallback(
    async (path: string) => {
      try {
        await commands.setSharedFolder(path);
        dispatch({ type: "settings/sharedFolder", path });
        log(`📁 공유 폴더 설정: ${path}`);
        dispatch({ type: "localFiles/loaded", files: await commands.getLocalSharedFiles("/") });
      } catch (e) {
        log(`❌ 공유 폴더 설정 실패: ${describeError(e)}`);
      }
    },
    [dispatch, log],
  );

  const setSharedFolderDisplayName = useCallback(
    async (name: string) => {
      try {
        await commands.setSharedFolderDisplayName(name);
        dispatch({ type: "settings/sharedFolderDisplayName", name });
      } catch (e) {
        log(`❌ 공유 폴더 표시 이름 설정 실패: ${describeError(e)}`);
      }
    },
    [dispatch, log],
  );

  const refreshLocalFiles = useCallback(
    async (relativePath: string = "/") => {
      try {
        dispatch({ type: "localFiles/loaded", files: await commands.getLocalSharedFiles(relativePath) });
      } catch (e) {
        log(`❌ 파일 목록 새로고침 실패: ${describeError(e)}`);
      }
    },
    [dispatch, log],
  );

  return {
    initialize,
    pendingDownloads,
    pendingUploads,
    startP2p,
    stopP2p,
    connectToPeer,
    requestFileList,
    downloadFile,
    uploadFile,
    cancelTransfer,
    requestPairing,
    respondPairing,
    clearAll,
    setMyDeviceName,
    setSharedFolder,
    setSharedFolderDisplayName,
    refreshLocalFiles,
  };
}
