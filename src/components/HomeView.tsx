import { Folder, ArrowLeft, Download, MoreVertical, FileText, Image, Film, Music, File, Upload, Check, Monitor } from "lucide-react";
import { useState, useRef } from "react";

interface SharedFolder {
  id: string;
  name: string;
  path: string;
  color: string;
  deviceName: string;
  fileCount: number;
  totalSize: string;
}

interface FolderFile {
  id: string;
  name: string;
  type: "document" | "image" | "video" | "audio" | "other";
  size: string;
  modifiedDate: string;
}

export function HomeView() {
  const [selectedFolder, setSelectedFolder] = useState<SharedFolder | null>(null);
  const [selectedFiles, setSelectedFiles] = useState<string[]>([]);
  const [isSelectionMode, setIsSelectionMode] = useState(false);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const sharedFolders: SharedFolder[] = [
    {
      id: "1",
      name: "작업 문서",
      path: "C:/Users/Documents/Work",
      color: "from-blue-400 to-cyan-400",
      deviceName: "맥북 프로",
      fileCount: 24,
      totalSize: "2.4 GB"
    },
    {
      id: "2",
      name: "사진 보관함",
      path: "C:/Users/Pictures/Shared",
      color: "from-pink-400 to-rose-400",
      deviceName: "내 아이폰",
      fileCount: 156,
      totalSize: "8.7 GB"
    },
    {
      id: "3",
      name: "프로젝트 파일",
      path: "C:/Users/Projects",
      color: "from-purple-400 to-indigo-400",
      deviceName: "맥북 프로",
      fileCount: 89,
      totalSize: "15.2 GB"
    },
    {
      id: "4",
      name: "음악 컬렉션",
      path: "C:/Users/Music/Library",
      color: "from-orange-400 to-yellow-400",
      deviceName: "아이패드",
      fileCount: 342,
      totalSize: "4.8 GB"
    },
  ];

  const folderFiles: FolderFile[] = [
    { id: "1", name: "프로젝트 제안서.pdf", type: "document", size: "2.4 MB", modifiedDate: "2시간 전" },
    { id: "2", name: "회의록_0114.docx", type: "document", size: "856 KB", modifiedDate: "5시간 전" },
    { id: "3", name: "디자인_시안.png", type: "image", size: "4.2 MB", modifiedDate: "1일 전" },
    { id: "4", name: "발표_영상.mp4", type: "video", size: "125 MB", modifiedDate: "2일 전" },
    { id: "5", name: "배경음악.mp3", type: "audio", size: "5.8 MB", modifiedDate: "3일 전" },
    { id: "6", name: "참고자료.zip", type: "other", size: "18 MB", modifiedDate: "1주 전" },
    { id: "7", name: "예산표.xlsx", type: "document", size: "124 KB", modifiedDate: "1주 전" },
    { id: "8", name: "로고_최종본.ai", type: "image", size: "8.4 MB", modifiedDate: "2주 전" },
  ];

  const getFileIcon = (type: string) => {
    switch (type) {
      case "document": return FileText;
      case "image": return Image;
      case "video": return Film;
      case "audio": return Music;
      default: return File;
    }
  };

  const getFileColor = (type: string) => {
    switch (type) {
      case "document": return "from-blue-400 to-blue-500";
      case "image": return "from-pink-400 to-rose-500";
      case "video": return "from-purple-400 to-indigo-500";
      case "audio": return "from-orange-400 to-yellow-500";
      default: return "from-gray-400 to-gray-500";
    }
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

  // Folder List View
  if (!selectedFolder) {
    return (
      <div className="flex flex-col h-full">
        {/* Header */}
        <div className="sticky top-16 z-40 bg-background/95 backdrop-blur-xl border-b border-border/50 px-4 py-4">
          <div className="flex items-center justify-between mb-2">
            <div>
              <h2 className="mb-1">공유 폴더</h2>
              <p className="text-sm text-muted-foreground">
                연결된 기기의 공유 폴더들
              </p>
            </div>
          </div>
        </div>

        {/* Folders List */}
        <div className="flex-1 overflow-y-auto px-4 py-4">
          <div className="space-y-3">
            {sharedFolders.map((folder) => (
              <button
                key={folder.id}
                onClick={() => setSelectedFolder(folder)}
                className="w-full flex items-center gap-4 p-4 rounded-2xl bg-card border border-border hover:shadow-lg hover:border-primary/50 hover:scale-[1.01] transition-all text-left"
              >
                <div className={`w-14 h-14 rounded-xl bg-gradient-to-br ${folder.color} flex items-center justify-center flex-shrink-0 shadow-md`}>
                  <Folder className="w-7 h-7 text-white" />
                </div>

                <div className="flex-1 min-w-0">
                  <h4 className="mb-1 truncate">{folder.name}</h4>
                  <div className="flex items-center gap-2 text-xs text-muted-foreground mb-1">
                    <Monitor className="w-3 h-3" />
                    <span>{folder.deviceName}</span>
                  </div>
                  <p className="text-xs text-muted-foreground/70 truncate font-mono">
                    {folder.path}
                  </p>
                </div>

                <div className="flex flex-col items-end gap-1 flex-shrink-0">
                  <span className="text-sm font-medium">{folder.totalSize}</span>
                  <span className="text-xs text-muted-foreground">{folder.fileCount}개 파일</span>
                </div>
              </button>
            ))}
          </div>

          {/* Empty State */}
          {sharedFolders.length === 0 && (
            <div className="flex flex-col items-center justify-center py-16 text-center">
              <div className="w-20 h-20 rounded-3xl bg-muted/50 flex items-center justify-center mb-4">
                <Folder className="w-10 h-10 text-muted-foreground" />
              </div>
              <p className="text-muted-foreground mb-2">공유 폴더가 없습니다</p>
              <p className="text-sm text-muted-foreground">
                설정에서 공유 폴더를 추가해보세요
              </p>
            </div>
          )}
        </div>
      </div>
    );
  }

  // Folder Detail View (Files inside the selected folder)
  return (
    <div className="flex flex-col h-full">
      {/* Header with Back Button */}
      <div className="sticky top-16 z-40 bg-background/95 backdrop-blur-xl border-b border-border/50 px-4 py-3">
        <div className="flex items-center gap-3 mb-3">
          <button
            onClick={() => {
              setSelectedFolder(null);
              setSelectedFiles([]);
              setIsSelectionMode(false);
            }}
            className="p-2 rounded-xl hover:bg-muted/50 transition-colors"
          >
            <ArrowLeft className="w-5 h-5" />
          </button>

          <div className={`w-10 h-10 rounded-xl bg-gradient-to-br ${selectedFolder.color} flex items-center justify-center shadow-md flex-shrink-0`}>
            <Folder className="w-5 h-5 text-white" />
          </div>

          <div className="flex-1 min-w-0">
            <h3 className="truncate">{selectedFolder.name}</h3>
            <p className="text-xs text-muted-foreground truncate">
              {selectedFolder.deviceName} • {selectedFolder.path}
            </p>
          </div>

          <button
            onClick={handleUploadFromPC}
            className="px-3 py-2 rounded-xl bg-gradient-to-r from-primary to-chart-2 text-white font-medium hover:shadow-lg transition-shadow flex items-center gap-2 text-sm"
          >
            <Upload className="w-4 h-4" />
            전송
          </button>
        </div>

        {/* Hidden file input */}
        <input
          ref={fileInputRef}
          type="file"
          multiple
          onChange={handleFileSelect}
          className="hidden"
        />

        {/* Selection Mode Bar */}
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
        <div className="space-y-2">
          {folderFiles.map((file) => {
            const Icon = getFileIcon(file.type);
            const gradient = getFileColor(file.type);
            const isSelected = selectedFiles.includes(file.name);

            return (
              <div
                key={file.id}
                onClick={() => toggleFileSelection(file.name)}
                className={`flex items-center gap-3 p-3 rounded-xl bg-card border transition-all cursor-pointer hover:scale-[1.01] ${
                  isSelected
                    ? "border-primary bg-primary/5 shadow-md"
                    : "border-border hover:shadow-md"
                }`}
              >
                {/* Selection Checkbox */}
                <div className={`w-5 h-5 rounded-md border-2 flex items-center justify-center flex-shrink-0 transition-colors ${
                  isSelected
                    ? "bg-primary border-primary"
                    : "border-border"
                }`}>
                  {isSelected && (
                    <Check className="w-3 h-3 text-white" />
                  )}
                </div>

                <div className={`w-11 h-11 rounded-xl bg-gradient-to-br ${gradient} flex items-center justify-center flex-shrink-0 shadow-sm`}>
                  <Icon className="w-5 h-5 text-white" />
                </div>

                <div className="flex-1 min-w-0">
                  <p className="truncate font-medium text-sm">{file.name}</p>
                  <div className="flex items-center gap-2 text-xs text-muted-foreground mt-0.5">
                    <span>{file.modifiedDate}</span>
                    <span>•</span>
                    <span>{file.size}</span>
                  </div>
                </div>

                {!isSelectionMode && (
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                    }}
                    className="p-2 rounded-lg hover:bg-muted/50 transition-colors flex-shrink-0"
                  >
                    <MoreVertical className="w-4 h-4 text-muted-foreground" />
                  </button>
                )}
              </div>
            );
          })}
        </div>

        {folderFiles.length === 0 && (
          <div className="flex flex-col items-center justify-center py-16 text-center">
            <div className="w-20 h-20 rounded-3xl bg-muted/50 flex items-center justify-center mb-4">
              <Folder className="w-10 h-10 text-muted-foreground" />
            </div>
            <p className="text-muted-foreground">폴더가 비어있습니다</p>
          </div>
        )}
      </div>

      {/* Folder Info Footer */}
      <div className="sticky bottom-0 bg-gradient-to-br from-primary/5 to-chart-2/5 border-t border-border/50 px-4 py-3">
        <div className="flex items-center justify-between text-sm">
          <span className="text-muted-foreground">총 {selectedFolder.fileCount}개 파일</span>
          <span className="font-medium">{selectedFolder.totalSize}</span>
        </div>
      </div>
    </div>
  );
}
