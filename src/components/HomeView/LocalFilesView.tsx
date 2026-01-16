import { useState } from "react";
import {
  Folder,
  ArrowLeft,
  RefreshCw,
  Settings,
  FolderOpen,
  Search,
  Grid3x3,
  List,
} from "lucide-react";
import { useP2pContext } from "../../contexts/P2pContext";
import { SharedFolderModal } from "../SharedFolderModal";
import { FileListItem } from "./FileListItem";
import { FileGridItem } from "./FileGridItem";
import { SelectionBar } from "./SelectionBar";
import { SendModal } from "./SendModal";
import { useFileSelection } from "./hooks/useFileSelection";
import type { DisplayMode } from "./types";

interface LocalFilesViewProps {
  onBack: () => void;
}

export function LocalFilesView({ onBack }: LocalFilesViewProps) {
  const [showSettingsModal, setShowSettingsModal] = useState(false);
  const [displayMode, setDisplayMode] = useState<DisplayMode>("list");
  const [searchQuery, setSearchQuery] = useState("");
  const [showSendModal, setShowSendModal] = useState(false);
  const [currentPath, setCurrentPath] = useState("/");

  const {
    connectedPeers,
    localFiles,
    myDeviceName,
    sharedFolderPath,
    refreshLocalFiles,
    setMyDeviceName,
    setSharedFolder,
    uploadFile,
  } = useP2pContext();

  const {
    selectedFiles,
    isSelectionMode,
    toggleSelection,
    cancelSelection,
  } = useFileSelection();

  const handleSaveSettings = async (newPath: string, newDeviceName: string) => {
    await setSharedFolder(newPath);
    await setMyDeviceName(newDeviceName);
  };

  // 폴더 진입 핸들러
  const handleFolderClick = (folderPath: string) => {
    setCurrentPath(folderPath);
    refreshLocalFiles(folderPath);
    setSearchQuery("");
    cancelSelection();
  };

  // 뒤로가기 핸들러
  const handleBack = () => {
    if (currentPath === "/") {
      onBack();
      return;
    }
    // 부모 경로 계산
    const parts = currentPath.split("/").filter((p) => p);
    parts.pop();
    const parentPath = parts.length === 0 ? "/" : "/" + parts.join("/");
    setCurrentPath(parentPath);
    refreshLocalFiles(parentPath);
    setSearchQuery("");
    cancelSelection();
  };

  // 파일 전송 핸들러
  const handleSendToDevice = async (peerId: string) => {
    // 선택된 파일들의 절대 경로 구성
    for (const fileName of selectedFiles) {
      const file = localFiles.find((f) => f.name === fileName);
      if (file && !file.is_dir) {
        // 절대 경로 생성: sharedFolderPath + 상대 경로
        const absolutePath = sharedFolderPath + file.path;
        // 원격 경로는 "/"로 설정 (상대방의 공유 폴더 루트)
        await uploadFile(peerId, absolutePath, "/");
      }
    }

    setShowSendModal(false);
    cancelSelection();
  };

  // 필터링된 파일
  const filteredFiles = localFiles.filter((file) =>
    file.name.toLowerCase().includes(searchQuery.toLowerCase())
  );

  // 현재 폴더명
  const currentFolderName =
    currentPath === "/" ? myDeviceName : currentPath.split("/").pop();

  // 현재 경로 표시
  const displayPath =
    currentPath === "/" ? sharedFolderPath || "~/Pebble/Shared" : currentPath;

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="sticky top-16 z-40 bg-background/95 backdrop-blur-xl border-b border-border/50 px-4 py-3">
        <div className="flex items-center gap-3 mb-3">
          <button
            onClick={handleBack}
            className="p-2 rounded-xl hover:bg-muted/50 transition-colors"
          >
            <ArrowLeft className="w-5 h-5" />
          </button>

          <div className="w-10 h-10 rounded-xl bg-gradient-to-br from-blue-400 to-cyan-400 flex items-center justify-center shadow-md flex-shrink-0">
            <Folder className="w-5 h-5 text-white" />
          </div>

          <div className="flex-1 min-w-0">
            <h3 className="truncate font-medium">{currentFolderName}</h3>
            <p className="text-xs text-muted-foreground truncate font-mono">
              {displayPath}
            </p>
          </div>

          <button
            onClick={() => refreshLocalFiles(currentPath)}
            className="p-2 rounded-xl hover:bg-muted/50 transition-colors"
          >
            <RefreshCw className="w-5 h-5" />
          </button>

          <button
            onClick={() => setShowSettingsModal(true)}
            className="p-2 rounded-xl hover:bg-muted/50 transition-colors"
          >
            <Settings className="w-5 h-5 text-muted-foreground" />
          </button>
        </div>

        {/* File count info */}
        <div className="flex items-center justify-between px-1 mb-3">
          <span className="text-sm text-muted-foreground">
            총 {localFiles.length}개 항목
          </span>
          <span className="text-sm font-medium text-blue-600 flex items-center gap-1">
            <Folder className="w-4 h-4" />
            공유 중
          </span>
        </div>

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
      </div>

      {/* Files List */}
      <div className="flex-1 overflow-y-auto px-4 py-4">
        {filteredFiles.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-16 text-center">
            <div className="w-20 h-20 rounded-3xl bg-muted/50 flex items-center justify-center mb-4">
              <FolderOpen className="w-10 h-10 text-muted-foreground" />
            </div>
            <p className="text-muted-foreground mb-2">
              {searchQuery ? "검색 결과가 없습니다" : "공유 폴더가 비어있습니다"}
            </p>
            <p className="text-sm text-muted-foreground">
              {searchQuery
                ? "다른 검색어를 입력해보세요"
                : "공유 폴더에 파일을 추가해보세요"}
            </p>
          </div>
        ) : displayMode === "list" ? (
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
        )}
      </div>

      {/* Selection Bar */}
      {isSelectionMode && (
        <SelectionBar
          selectedCount={selectedFiles.length}
          onCancel={cancelSelection}
          connectedPeersCount={connectedPeers.length}
          onSend={() => setShowSendModal(true)}
        />
      )}

      {/* Send Modal */}
      <SendModal
        isOpen={showSendModal}
        selectedFileCount={selectedFiles.length}
        peers={connectedPeers}
        onSend={handleSendToDevice}
        onClose={() => setShowSendModal(false)}
      />

      {/* Settings Modal */}
      <SharedFolderModal
        isOpen={showSettingsModal}
        onClose={() => setShowSettingsModal(false)}
        currentPath={sharedFolderPath}
        currentDeviceName={myDeviceName}
        onSave={handleSaveSettings}
      />
    </div>
  );
}
