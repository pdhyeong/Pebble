import { useState } from "react";
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
import { useP2pContext } from "../../contexts/P2pContext";
import { FileListItem } from "./FileListItem";
import { FileGridItem } from "./FileGridItem";
import { useFileSelection } from "./hooks/useFileSelection";
import type { DisplayMode } from "./types";

interface RemoteFilesViewProps {
  peerId: string;
  onBack: () => void;
}

export function RemoteFilesView({ peerId, onBack }: RemoteFilesViewProps) {
  const [displayMode, setDisplayMode] = useState<DisplayMode>("list");
  const [searchQuery, setSearchQuery] = useState("");

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

  // 필터링된 파일
  const filteredFiles =
    remoteFiles?.files.filter((file) =>
      file.name.toLowerCase().includes(searchQuery.toLowerCase())
    ) || [];

  return (
    <div className="flex flex-col h-full">
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
            className={`px-3 py-2 rounded-xl text-white font-medium transition-shadow flex items-center gap-2 text-sm ${
              isUploading
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
                className={`px-4 py-1.5 rounded-lg text-white text-sm font-medium transition-shadow flex items-center gap-1 ${
                  isDownloading
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
    </div>
  );
}
