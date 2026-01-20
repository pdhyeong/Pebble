import { useState, useEffect } from "react";
import {
  Folder,
  ArrowLeft,
  Monitor,
  RefreshCw,
  Upload,
  Search,
  Grid3x3,
  List,
  Download,
  FolderOpen,
} from "lucide-react";
import { open } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";
import { useP2pContext } from "../../contexts/P2pContext";
import { FileListItem } from "./FileListItem";
import { FileGridItem } from "./FileGridItem";
import { useFileSelection } from "./hooks/useFileSelection";
import { DropConfirmModal } from "../Transfer/DropConfirmModal";
import { TransferQueuePanel } from "../Transfer/TransferQueuePanel";
import { useTransferQueue } from "../Transfer/hooks/useTransferQueue";
import type { DisplayMode } from "./types";

interface RemoteFilesViewProps {
  peerId: string;
  onBack: () => void;
}

export function RemoteFilesView({ peerId, onBack }: RemoteFilesViewProps) {
  const [displayMode, setDisplayMode] = useState<DisplayMode>("list");
  const [searchQuery, setSearchQuery] = useState("");
  const [isDragging, setIsDragging] = useState(false);
  const [droppedFiles, setDroppedFiles] = useState<any[]>([]);
  const [showDropModal, setShowDropModal] = useState(false);
  const [showQueuePanel, setShowQueuePanel] = useState(false);

  // 전송 큐 훅
  const transferQueue = useTransferQueue();

  const {
    connectedPeers,
    remoteFiles,
    isLoadingFiles,
    isDownloading,
    isUploading,
    requestFileList,
    downloadFile,
    uploadFile,
  } = useP2pContext();

  const {
    selectedFiles,
    isSelectionMode,
    toggleSelection,
    cancelSelection,
  } = useFileSelection();

  const selectedPeer = connectedPeers.find((p) => p.peerId === peerId);

  // 폴더 진입 핸들러
  const handleFolderClick = (folderPath: string) => {
    requestFileList(peerId, folderPath);
    setSearchQuery("");
    cancelSelection();
  };

  // 다운로드 핸들러
  const handleDownloadSelected = async () => {
    if (selectedFiles.length === 0) return;

    for (const fileName of selectedFiles) {
      const file = remoteFiles?.files.find((f) => f.name === fileName);
      if (file && !file.is_dir) {
        await downloadFile(peerId, file.path);
      }
    }

    cancelSelection();
  };

  // 파일 업로드 (전송) 핸들러
  const handleUploadFromPC = async () => {
    try {
      // Tauri 파일 선택 다이얼로그 열기
      const selected = await open({
        multiple: true,
        title: "전송할 파일 선택",
      });

      if (selected) {
        // 단일 파일 또는 다중 파일 처리
        const files = Array.isArray(selected) ? selected : [selected];

        for (const filePath of files) {
          // 현재 원격 경로로 업로드
          const remotePath = remoteFiles?.path || "/";
          await uploadFile(peerId, filePath, remotePath);
        }
      }
    } catch (error) {
      console.error("파일 선택 실패:", error);
    }
  };

  // 드래그 앤 드롭 핸들러
  const handleDragEnter = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    console.log("🎯 드래그 진입!");
    setIsDragging(true);
  };

  const handleDragLeave = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    console.log("👋 드래그 떠남");
    // 자식 요소로 이동 시 false로 변경되는 것 방지
    if (e.currentTarget === e.target) {
      setIsDragging(false);
    }
  };

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
  };

  const handleDrop = async (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragging(false);

    const files = Array.from(e.dataTransfer.files);
    console.log("드롭된 파일 수:", files.length);

    if (files.length === 0) {
      console.log("파일이 없습니다");
      return;
    }

    // 파일 정보 수집 (Tauri에서 File 객체는 path 속성을 가짐)
    const fileInfos = files.map(f => {
      const filePath = (f as any).path || "";
      console.log("파일:", f.name, "경로:", filePath);
      return {
        name: f.name,
        path: filePath,
        size: f.size,
        type: f.type,
      };
    });

    // 경로가 없는 파일 필터링
    const validFiles = fileInfos.filter(f => f.path !== "");

    if (validFiles.length === 0) {
      console.error("유효한 파일 경로가 없습니다");
      console.warn("❌ 파일 경로를 가져올 수 없습니다. Tauri 환경에서 실행 중인지 확인하세요.");
      alert("파일 경로를 가져올 수 없습니다. 브라우저 개발자 도구 콘솔을 확인하세요.");
      return;
    }

    console.log("유효한 파일:", validFiles);
    setDroppedFiles(validFiles);
    setShowDropModal(true);
  };

  // 드롭 확인 후 전송
  const handleConfirmDrop = async () => {
    setShowDropModal(false);

    // 큐에 추가
    const remotePath = remoteFiles?.path || "/";
    const fileInfos = droppedFiles.map(f => ({
      name: f.name,
      path: f.path,
      size: f.size,
    }));

    transferQueue.addFiles(fileInfos, peerId, remotePath);
    setShowQueuePanel(true);
    setDroppedFiles([]);

    // 자동 전송 시작
    processQueue();
  };

  const handleCancelDrop = () => {
    setShowDropModal(false);
    setDroppedFiles([]);
  };

  // 큐 처리
  const processQueue = async () => {
    const { queue, stats } = transferQueue;

    // 동시 전송 수 제한 확인
    if (!stats.canStartMore) return;

    // 대기 중인 파일 찾기
    const waiting = queue.find(q => q.status === "waiting");
    if (!waiting) return;

    // 전송 시작
    transferQueue.startTransfer(waiting.id);

    try {
      await uploadFile(waiting.peerId, waiting.filePath, waiting.remotePath);
      transferQueue.completeTransfer(waiting.id);
    } catch (error) {
      transferQueue.failTransfer(waiting.id, String(error));
    }

    // 다음 파일 처리
    setTimeout(() => processQueue(), 100);
  };

  // Tauri 파일 드롭 이벤트 리스너
  useEffect(() => {
    let unlisten: (() => void) | null = null;

    const setupFileDropListener = async () => {
      const { listen: tauriListen } = await import("@tauri-apps/api/event");
      const { stat } = await import("@tauri-apps/plugin-fs");

      // 올바른 이벤트 이름: tauri://drag-drop
      unlisten = await tauriListen<string[]>("tauri://drag-drop", async (event) => {
        console.log("📦 Tauri 파일 드롭 감지!", event.payload);
        const filePaths = event.payload;

        if (filePaths.length === 0) {
          console.log("파일이 없습니다");
          return;
        }

        // 파일 정보 수집 (크기 포함)
        const fileInfos = await Promise.all(
          filePaths.map(async (path) => {
            const fileName = path.split("/").pop() || path.split("\\").pop() || "unknown";
            try {
              const metadata = await stat(path);
              return {
                name: fileName,
                path: path,
                size: metadata.size,
                type: "",
              };
            } catch (e) {
              console.error("파일 메타데이터 가져오기 실패:", e);
              return {
                name: fileName,
                path: path,
                size: 0,
                type: "",
              };
            }
          })
        );

        console.log("유효한 파일:", fileInfos);
        setDroppedFiles(fileInfos);
        setShowDropModal(true);
      });
    };

    setupFileDropListener();

    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  // 진행률 이벤트 리스너
  useEffect(() => {
    const unlisteners: Promise<() => void>[] = [];

    unlisteners.push(
      listen<string>("file-upload-progress", (event) => {
        try {
          const data = JSON.parse(event.payload);
          // 현재 업로드 중인 항목 찾기
          const uploading = transferQueue.queue.find(
            q => q.status === "uploading" && q.peerId === data.peer_id
          );
          if (uploading) {
            const progress = Math.round((data.bytes_sent / data.total_size) * 100);
            const speed = data.speed || 0;
            transferQueue.updateProgress(uploading.id, progress, speed);
          }
        } catch (e) {
          console.error("진행률 파싱 실패:", e);
        }
      })
    );

    return () => {
      unlisteners.forEach(p => p.then(fn => fn()));
    };
  }, [transferQueue]);

  // 필터링된 파일
  const filteredFiles =
    remoteFiles?.files.filter((file) =>
      file.name.toLowerCase().includes(searchQuery.toLowerCase())
    ) || [];

  return (
    <div
      className="flex flex-col h-full relative"
      onDragEnter={handleDragEnter}
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
    >
      {/* Drop overlay - 전체 화면 오버레이 */}
      {isDragging && (
        <div className="absolute inset-0 bg-blue-100/50 dark:bg-blue-900/30 border-2 border-dashed border-blue-500 rounded-xl flex items-center justify-center z-50">
          <div className="text-center">
            <Upload className="w-16 h-16 text-blue-600 dark:text-blue-400 mx-auto mb-3 animate-bounce" />
            <p className="text-lg font-semibold text-blue-600 dark:text-blue-400">
              파일을 놓아 전송하세요
            </p>
            <p className="text-sm text-blue-500 dark:text-blue-300 mt-1">
              {selectedPeer?.deviceName || "상대방"}에게 전송됩니다
            </p>
          </div>
        </div>
      )}

      {/* Header */}
      <div className="sticky top-16 z-40 bg-background/95 backdrop-blur-xl border-b border-border/50 px-4 py-3">
        <div className="flex items-center gap-3 mb-3">
          <button
            onClick={onBack}
            className="p-2 rounded-xl hover:bg-muted/50 transition-colors"
          >
            <ArrowLeft className="w-5 h-5" />
          </button>

          <div className="w-10 h-10 rounded-xl bg-gradient-to-br from-green-400 to-emerald-400 flex items-center justify-center shadow-md flex-shrink-0">
            <Monitor className="w-5 h-5 text-white" />
          </div>

          <div className="flex-1 min-w-0">
            <h3 className="truncate font-medium">
              {selectedPeer?.deviceName || "상대방 공유폴더"}
            </h3>
            <p className="text-xs text-muted-foreground truncate font-mono">
              {peerId.slice(0, 20)}...
            </p>
          </div>

          <button
            onClick={() => requestFileList(peerId, "/")}
            disabled={isLoadingFiles}
            className="p-2 rounded-xl hover:bg-muted/50 transition-colors"
          >
            <RefreshCw
              className={`w-5 h-5 ${isLoadingFiles ? "animate-spin" : ""}`}
            />
          </button>

          <button
            onClick={handleUploadFromPC}
            disabled={isUploading}
            className={`px-3 py-2 rounded-xl text-white font-medium transition-shadow flex items-center gap-2 text-sm ${isUploading
              ? "bg-gray-400 cursor-not-allowed"
              : "bg-gradient-to-r from-primary to-chart-2 hover:shadow-lg"
              }`}
          >
            {isUploading ? (
              <>
                <RefreshCw className="w-4 h-4 animate-spin" />
                전송 중...
              </>
            ) : (
              <>
                <Upload className="w-4 h-4" />
                전송
              </>
            )}
          </button>
        </div>

        {/* File count info */}
        {remoteFiles && (
          <div className="flex items-center justify-between px-1 mb-3">
            <span className="text-sm text-muted-foreground">
              총 {remoteFiles.files.length}개 항목
            </span>
            <span className="text-sm font-medium text-green-600 flex items-center gap-1">
              <div className="w-2 h-2 rounded-full bg-green-500" />
              연결됨
            </span>
          </div>
        )}

        {/* Search bar */}
        <div className="flex items-center gap-2">
          <div className="flex-1 relative">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
            <input
              type="text"
              placeholder="파일 검색..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="w-full pl-10 pr-4 py-2.5 rounded-xl bg-muted/50 border border-border/50 focus:outline-none focus:ring-2 focus:ring-primary/20 transition-all"
            />
          </div>

          <button
            onClick={() =>
              setDisplayMode(displayMode === "grid" ? "list" : "grid")
            }
            className="p-2.5 rounded-xl bg-muted/50 border border-border/50 hover:bg-muted transition-colors"
          >
            {displayMode === "grid" ? (
              <List className="w-4 h-4" />
            ) : (
              <Grid3x3 className="w-4 h-4" />
            )}
          </button>
        </div>

        {/* Selection mode bar */}
        {isSelectionMode && (
          <div className="mt-3 p-3 rounded-xl bg-primary/10 border border-primary/20 flex items-center justify-between animate-in fade-in slide-in-from-top-2 duration-200">
            <span className="text-sm font-medium text-primary">
              {selectedFiles.length}개 파일 선택됨
            </span>
            <div className="flex gap-2">
              <button
                onClick={handleDownloadSelected}
                disabled={isDownloading}
                className={`px-4 py-1.5 rounded-lg text-white text-sm font-medium transition-shadow flex items-center gap-1 ${isDownloading
                  ? "bg-gray-400 cursor-not-allowed"
                  : "bg-gradient-to-r from-primary to-chart-2 hover:shadow-lg"
                  }`}
              >
                {isDownloading ? (
                  <>
                    <RefreshCw className="w-4 h-4 animate-spin" />
                    다운로드 중...
                  </>
                ) : (
                  <>
                    <Download className="w-4 h-4" />
                    다운로드
                  </>
                )}
              </button>
              <button
                onClick={cancelSelection}
                className="px-4 py-1.5 rounded-lg bg-muted/50 text-sm hover:bg-muted transition-colors"
              >
                취소
              </button>
            </div>
          </div>
        )}
      </div>

      {/* Files List */}
      <div className="flex-1 overflow-y-auto px-4 py-4">
        {isLoadingFiles ? (
          <div className="flex flex-col items-center justify-center py-16">
            <RefreshCw className="w-8 h-8 text-primary animate-spin mb-4" />
            <p className="text-muted-foreground">파일 목록 불러오는 중...</p>
          </div>
        ) : remoteFiles?.error ? (
          <div className="flex flex-col items-center justify-center py-16 text-center">
            <div className="w-20 h-20 rounded-3xl bg-red-500/10 flex items-center justify-center mb-4">
              <Folder className="w-10 h-10 text-red-500" />
            </div>
            <p className="text-red-500 mb-2">오류가 발생했습니다</p>
            <p className="text-sm text-muted-foreground">{remoteFiles.error}</p>
          </div>
        ) : filteredFiles.length > 0 ? (
          displayMode === "list" ? (
            <div className="space-y-2">
              {filteredFiles.map((file, idx) => (
                <FileListItem
                  key={idx}
                  file={file}
                  isSelected={!file.is_dir && selectedFiles.includes(file.name)}
                  showCheckbox={true}
                  onSelect={() => toggleSelection(file.name, file.is_dir)}
                  onFolderClick={() => handleFolderClick(file.path)}
                />
              ))}
            </div>
          ) : (
            <div className="grid grid-cols-2 gap-3">
              {filteredFiles.map((file, idx) => (
                <FileGridItem
                  key={idx}
                  file={file}
                  isSelected={!file.is_dir && selectedFiles.includes(file.name)}
                  onSelect={() => toggleSelection(file.name, file.is_dir)}
                  onFolderClick={() => handleFolderClick(file.path)}
                />
              ))}
            </div>
          )
        ) : (
          <div className="flex flex-col items-center justify-center py-16 text-center">
            <div className="w-20 h-20 rounded-3xl bg-muted/50 flex items-center justify-center mb-4">
              {searchQuery ? (
                <Search className="w-10 h-10 text-muted-foreground" />
              ) : (
                <FolderOpen className="w-10 h-10 text-muted-foreground" />
              )}
            </div>
            <p className="text-muted-foreground mb-2">
              {searchQuery ? "검색 결과가 없습니다" : "공유 폴더가 비어있습니다"}
            </p>
            <p className="text-sm text-muted-foreground">
              {searchQuery
                ? "다른 검색어를 입력해보세요"
                : "상대 기기의 공유 폴더에 파일을 추가해보세요"}
            </p>
          </div>
        )}
      </div>

      {/* Drop Confirm Modal */}
      <DropConfirmModal
        isOpen={showDropModal}
        files={droppedFiles}
        peerName={selectedPeer?.deviceName || "상대방"}
        onConfirm={handleConfirmDrop}
        onCancel={handleCancelDrop}
      />

      {/* Transfer Queue Panel */}
      {showQueuePanel && (
        <TransferQueuePanel
          queue={transferQueue.queue}
          stats={transferQueue.stats}
          onCancel={transferQueue.cancelTransfer}
          onClearCompleted={transferQueue.clearCompleted}
          onClose={() => setShowQueuePanel(false)}
        />
      )}
    </div>
  );
}
