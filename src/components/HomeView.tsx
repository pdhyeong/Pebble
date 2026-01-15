import { Folder, ArrowLeft, Download, MoreVertical, FileText, Image, Film, Music, File, Upload, Check, Monitor, Wifi, RefreshCw, Settings, FolderOpen } from "lucide-react";
import { useState, useRef } from "react";
import { useP2pContext, type RemoteFileInfo } from "../contexts/P2pContext";

interface LocalSharedFolder {
  id: string;
  name: string;
  path: string;
}

export function HomeView() {
  const [selectedPeerId, setSelectedPeerId] = useState<string | null>(null);
  const [selectedFiles, setSelectedFiles] = useState<string[]>([]);
  const [isSelectionMode, setIsSelectionMode] = useState(false);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const {
    connectedPeers,
    remoteFiles,
    isLoadingFiles,
    requestFileList,
  } = useP2pContext();

  // 내 공유폴더 설정 (로컬 저장소에서 불러오기 - 추후 구현)
  const [mySharedFolder] = useState<LocalSharedFolder>({
    id: "local",
    name: "내 공유폴더",
    path: "~/Pebble/Shared",
  });

  const getFileIcon = (file: RemoteFileInfo) => {
    const ext = file.name.split('.').pop()?.toLowerCase() || '';
    if (file.is_dir) return Folder;
    if (['jpg', 'jpeg', 'png', 'gif', 'webp', 'svg'].includes(ext)) return Image;
    if (['mp4', 'avi', 'mov', 'mkv', 'webm'].includes(ext)) return Film;
    if (['mp3', 'wav', 'flac', 'aac', 'm4a'].includes(ext)) return Music;
    if (['pdf', 'doc', 'docx', 'txt', 'md'].includes(ext)) return FileText;
    return File;
  };

  const getFileColor = (file: RemoteFileInfo) => {
    const ext = file.name.split('.').pop()?.toLowerCase() || '';
    if (file.is_dir) return "from-blue-400 to-cyan-400";
    if (['jpg', 'jpeg', 'png', 'gif', 'webp', 'svg'].includes(ext)) return "from-pink-400 to-rose-500";
    if (['mp4', 'avi', 'mov', 'mkv', 'webm'].includes(ext)) return "from-purple-400 to-indigo-500";
    if (['mp3', 'wav', 'flac', 'aac', 'm4a'].includes(ext)) return "from-orange-400 to-yellow-500";
    if (['pdf', 'doc', 'docx', 'txt', 'md'].includes(ext)) return "from-blue-400 to-blue-500";
    return "from-gray-400 to-gray-500";
  };

  const formatFileSize = (bytes: number): string => {
    if (bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${(bytes / Math.pow(k, i)).toFixed(1)} ${sizes[i]}`;
  };

  const toggleFileSelection = (fileName: string) => {
    if (!isSelectionMode) {
      setIsSelectionMode(true);
      setSelectedFiles([fileName]);
    } else {
      if (selectedFiles.includes(fileName)) {
        const newSelection = selectedFiles.filter(f => f !== fileName);
        setSelectedFiles(newSelection);
        if (newSelection.length === 0) {
          setIsSelectionMode(false);
        }
      } else {
        setSelectedFiles([...selectedFiles, fileName]);
      }
    }
  };

  const handleDownloadSelected = () => {
    console.log(`Downloading ${selectedFiles.length} files`);
    setSelectedFiles([]);
    setIsSelectionMode(false);
  };

  const cancelSelection = () => {
    setSelectedFiles([]);
    setIsSelectionMode(false);
  };

  const handleUploadFromPC = () => {
    fileInputRef.current?.click();
  };

  const handleFileSelect = (e: React.ChangeEvent<HTMLInputElement>) => {
    const files = e.target.files;
    if (files && files.length > 0) {
      console.log(`Selected ${files.length} files:`, Array.from(files).map(f => f.name));
    }
  };

  const handleSelectPeer = (peerId: string) => {
    setSelectedPeerId(peerId);
    requestFileList(peerId, "/");
  };

  const handleBackToList = () => {
    setSelectedPeerId(null);
    setSelectedFiles([]);
    setIsSelectionMode(false);
  };

  // 피어가 선택되어 있으면 파일 목록 표시
  if (selectedPeerId) {
    return (
      <div className="flex flex-col h-full">
        {/* Header with Back Button */}
        <div className="sticky top-16 z-40 bg-background/95 backdrop-blur-xl border-b border-border/50 px-4 py-3">
          <div className="flex items-center gap-3 mb-3">
            <button
              onClick={handleBackToList}
              className="p-2 rounded-xl hover:bg-muted/50 transition-colors"
            >
              <ArrowLeft className="w-5 h-5" />
            </button>

            <div className="w-10 h-10 rounded-xl bg-gradient-to-br from-green-400 to-emerald-400 flex items-center justify-center shadow-md flex-shrink-0">
              <Monitor className="w-5 h-5 text-white" />
            </div>

            <div className="flex-1 min-w-0">
              <h3 className="truncate">상대방 공유폴더</h3>
              <p className="text-xs text-muted-foreground truncate font-mono">
                {selectedPeerId?.slice(0, 20)}...
              </p>
            </div>

            <button
              onClick={() => requestFileList(selectedPeerId!, "/")}
              disabled={isLoadingFiles}
              className="p-2 rounded-xl hover:bg-muted/50 transition-colors"
            >
              <RefreshCw className={`w-5 h-5 ${isLoadingFiles ? "animate-spin" : ""}`} />
            </button>

            <button
              onClick={handleUploadFromPC}
              className="px-3 py-2 rounded-xl bg-gradient-to-r from-primary to-chart-2 text-white font-medium hover:shadow-lg transition-shadow flex items-center gap-2 text-sm"
            >
              <Upload className="w-4 h-4" />
              전송
            </button>
          </div>

          <input
            ref={fileInputRef}
            type="file"
            multiple
            onChange={handleFileSelect}
            className="hidden"
          />

          {isSelectionMode && (
            <div className="p-3 rounded-xl bg-primary/10 border border-primary/20 flex items-center justify-between animate-in fade-in slide-in-from-top-2 duration-200">
              <span className="text-sm font-medium text-primary">
                {selectedFiles.length}개 파일 선택됨
              </span>
              <div className="flex gap-2">
                <button
                  onClick={handleDownloadSelected}
                  className="px-4 py-1.5 rounded-lg bg-gradient-to-r from-primary to-chart-2 text-white text-sm font-medium hover:shadow-lg transition-shadow flex items-center gap-1"
                >
                  <Download className="w-4 h-4" />
                  다운로드
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
          ) : remoteFiles && remoteFiles.files.length > 0 ? (
            <div className="space-y-2">
              {remoteFiles.files.map((file, idx) => {
                const Icon = getFileIcon(file);
                const gradient = getFileColor(file);
                const isSelected = selectedFiles.includes(file.name);

                return (
                  <div
                    key={idx}
                    onClick={() => toggleFileSelection(file.name)}
                    className={`flex items-center gap-3 p-3 rounded-xl bg-card border transition-all cursor-pointer hover:scale-[1.01] ${
                      isSelected
                        ? "border-primary bg-primary/5 shadow-md"
                        : "border-border hover:shadow-md"
                    }`}
                  >
                    <div className={`w-5 h-5 rounded-md border-2 flex items-center justify-center flex-shrink-0 transition-colors ${
                      isSelected ? "bg-primary border-primary" : "border-border"
                    }`}>
                      {isSelected && <Check className="w-3 h-3 text-white" />}
                    </div>

                    <div className={`w-11 h-11 rounded-xl bg-gradient-to-br ${gradient} flex items-center justify-center flex-shrink-0 shadow-sm`}>
                      <Icon className="w-5 h-5 text-white" />
                    </div>

                    <div className="flex-1 min-w-0">
                      <p className="truncate font-medium text-sm">{file.name}</p>
                      <div className="flex items-center gap-2 text-xs text-muted-foreground mt-0.5">
                        {file.is_dir ? <span>폴더</span> : <span>{formatFileSize(file.size)}</span>}
                      </div>
                    </div>

                    {!isSelectionMode && (
                      <button
                        onClick={(e) => e.stopPropagation()}
                        className="p-2 rounded-lg hover:bg-muted/50 transition-colors flex-shrink-0"
                      >
                        <MoreVertical className="w-4 h-4 text-muted-foreground" />
                      </button>
                    )}
                  </div>
                );
              })}
            </div>
          ) : (
            <div className="flex flex-col items-center justify-center py-16 text-center">
              <div className="w-20 h-20 rounded-3xl bg-muted/50 flex items-center justify-center mb-4">
                <Folder className="w-10 h-10 text-muted-foreground" />
              </div>
              <p className="text-muted-foreground mb-2">공유 폴더가 비어있습니다</p>
              <p className="text-sm text-muted-foreground">
                상대 기기의 ~/Pebble/Shared 폴더에 파일을 추가해보세요
              </p>
            </div>
          )}
        </div>

        {remoteFiles && remoteFiles.files.length > 0 && (
          <div className="sticky bottom-0 bg-gradient-to-br from-primary/5 to-chart-2/5 border-t border-border/50 px-4 py-3">
            <div className="flex items-center justify-between text-sm">
              <span className="text-muted-foreground">총 {remoteFiles.files.length}개 항목</span>
              <span className="font-medium text-green-600 flex items-center gap-1">
                <div className="w-2 h-2 rounded-full bg-green-500" />
                연결됨
              </span>
            </div>
          </div>
        )}
      </div>
    );
  }

  // 메인 화면 - 내 공유폴더 + 상대방 공유폴더
  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="sticky top-16 z-40 bg-background/95 backdrop-blur-xl border-b border-border/50 px-4 py-4">
        <div className="flex items-center justify-between mb-2">
          <div>
            <h2 className="mb-1">공유 폴더</h2>
            <p className="text-sm text-muted-foreground">
              파일을 공유하고 전송하세요
            </p>
          </div>
        </div>
      </div>

      <div className="flex-1 overflow-y-auto px-4 py-4 space-y-6">
        {/* 내 공유폴더 섹션 */}
        <section>
          <h3 className="text-sm font-medium text-muted-foreground mb-3 flex items-center gap-2">
            <FolderOpen className="w-4 h-4" />
            내 공유폴더
          </h3>
          <div className="p-4 rounded-2xl bg-card border border-border">
            <div className="flex items-center gap-4">
              <div className="w-14 h-14 rounded-xl bg-gradient-to-br from-blue-400 to-cyan-400 flex items-center justify-center flex-shrink-0 shadow-md">
                <Folder className="w-7 h-7 text-white" />
              </div>
              <div className="flex-1 min-w-0">
                <h4 className="font-medium mb-1">{mySharedFolder.name}</h4>
                <p className="text-xs text-muted-foreground font-mono truncate">
                  {mySharedFolder.path}
                </p>
              </div>
              <button className="p-2 rounded-xl hover:bg-muted/50 transition-colors">
                <Settings className="w-5 h-5 text-muted-foreground" />
              </button>
            </div>
            <p className="text-xs text-muted-foreground mt-3">
              이 폴더의 파일이 연결된 기기와 공유됩니다
            </p>
          </div>
        </section>

        {/* 상대방 공유폴더 섹션 */}
        <section>
          <h3 className="text-sm font-medium text-muted-foreground mb-3 flex items-center gap-2">
            <Monitor className="w-4 h-4" />
            연결된 기기 ({connectedPeers.length})
          </h3>

          {connectedPeers.length === 0 ? (
            <div className="p-6 rounded-2xl bg-muted/30 border border-border/50 text-center">
              <div className="w-16 h-16 rounded-2xl bg-muted/50 flex items-center justify-center mx-auto mb-4">
                <Wifi className="w-8 h-8 text-muted-foreground" />
              </div>
              <p className="text-muted-foreground mb-2">연결된 기기가 없습니다</p>
              <p className="text-sm text-muted-foreground">
                기기 탭에서 P2P를 시작하고 다른 기기와 연결하세요
              </p>
            </div>
          ) : (
            <div className="space-y-3">
              {connectedPeers.map((peer) => (
                <button
                  key={peer.peerId}
                  onClick={() => handleSelectPeer(peer.peerId)}
                  className="w-full flex items-center gap-4 p-4 rounded-2xl bg-card border border-border hover:shadow-lg hover:border-green-500/50 hover:scale-[1.01] transition-all text-left"
                >
                  <div className="w-14 h-14 rounded-xl bg-gradient-to-br from-green-400 to-emerald-400 flex items-center justify-center flex-shrink-0 shadow-md">
                    <Monitor className="w-7 h-7 text-white" />
                  </div>

                  <div className="flex-1 min-w-0">
                    <h4 className="font-medium mb-1">연결된 기기</h4>
                    <p className="text-xs font-mono text-muted-foreground truncate">
                      {peer.peerId.slice(0, 20)}...
                    </p>
                    <div className="flex items-center gap-2 text-xs text-green-600 mt-1">
                      <div className="w-2 h-2 rounded-full bg-green-500 animate-pulse" />
                      <span>연결됨</span>
                      <span className="text-muted-foreground">
                        • {peer.connectedAt.toLocaleTimeString()}
                      </span>
                    </div>
                  </div>

                  <div className="flex items-center gap-2 text-primary">
                    <Folder className="w-5 h-5" />
                    <span className="text-sm font-medium">폴더 보기</span>
                  </div>
                </button>
              ))}
            </div>
          )}
        </section>
      </div>
    </div>
  );
}
