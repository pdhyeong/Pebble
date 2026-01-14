import {
  Folder,
  FileText,
  Image,
  Film,
  Music,
  File,
  Search,
  ArrowLeft,
  Grid3x3,
  List,
  FolderOpen,
} from "lucide-react";
import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

interface FileItem {
  name: string;
  isDirectory: boolean;
}

type FileType = "folder" | "document" | "image" | "video" | "audio" | "other";

function getFileType(name: string, isDirectory: boolean): FileType {
  if (isDirectory) return "folder";

  const ext = name.split(".").pop()?.toLowerCase() || "";

  if (["pdf", "doc", "docx", "txt", "xls", "xlsx", "ppt", "pptx"].includes(ext)) {
    return "document";
  }
  if (["jpg", "jpeg", "png", "gif", "bmp", "svg", "webp"].includes(ext)) {
    return "image";
  }
  if (["mp4", "avi", "mov", "mkv", "wmv", "flv"].includes(ext)) {
    return "video";
  }
  if (["mp3", "wav", "flac", "aac", "ogg", "m4a"].includes(ext)) {
    return "audio";
  }
  return "other";
}

function getIcon(type: FileType) {
  switch (type) {
    case "folder":
      return Folder;
    case "document":
      return FileText;
    case "image":
      return Image;
    case "video":
      return Film;
    case "audio":
      return Music;
    default:
      return File;
  }
}

function getIconColor(type: FileType) {
  switch (type) {
    case "folder":
      return "from-yellow-400 to-orange-400";
    case "document":
      return "from-blue-400 to-cyan-400";
    case "image":
      return "from-pink-400 to-rose-400";
    case "video":
      return "from-purple-400 to-indigo-400";
    case "audio":
      return "from-green-400 to-emerald-400";
    default:
      return "from-gray-400 to-slate-400";
  }
}

export function FilesView() {
  const [viewMode, setViewMode] = useState<"grid" | "list">("list");
  const [currentPath, setCurrentPath] = useState<string>("");
  const [pathHistory, setPathHistory] = useState<string[]>([]);
  const [searchQuery, setSearchQuery] = useState("");
  const [files, setFiles] = useState<FileItem[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string>("");

  async function selectFolder() {
    try {
      setError("");
      const folderPath = await open({
        directory: true,
        multiple: false,
      });

      if (folderPath) {
        setPathHistory([]);
        await loadFolder(folderPath as string);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  async function loadFolder(path: string) {
    setIsLoading(true);
    setError("");
    try {
      const result = await invoke<string[]>("get_file_list", {
        folderPath: path,
      });

      const fileItems: FileItem[] = result.map((name) => {
        const isDirectory = !name.includes(".") || name.startsWith(".");
        return { name, isDirectory };
      });

      fileItems.sort((a, b) => {
        if (a.isDirectory && !b.isDirectory) return -1;
        if (!a.isDirectory && b.isDirectory) return 1;
        return a.name.localeCompare(b.name);
      });

      setFiles(fileItems);
      setCurrentPath(path);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  }

  async function navigateToFolder(folderName: string) {
    const newPath = `${currentPath}/${folderName}`;
    setPathHistory([...pathHistory, currentPath]);
    await loadFolder(newPath);
  }

  async function goBack() {
    if (pathHistory.length > 0) {
      const previousPath = pathHistory[pathHistory.length - 1];
      setPathHistory(pathHistory.slice(0, -1));
      await loadFolder(previousPath);
    }
  }

  const filteredFiles = files.filter((file) =>
    file.name.toLowerCase().includes(searchQuery.toLowerCase())
  );

  const displayPath = currentPath.split("/").pop() || "폴더 선택";

  return (
    <div className="flex flex-col h-full">
      {/* Search and Path Bar */}
      <div className="sticky top-16 z-40 bg-background/95 backdrop-blur-xl border-b border-border/50 px-4 py-3">
        {/* Path breadcrumb */}
        <div className="flex items-center gap-2 mb-3 overflow-x-auto">
          {pathHistory.length > 0 && (
            <button
              onClick={goBack}
              className="p-1.5 rounded-lg hover:bg-muted/50 transition-colors flex-shrink-0"
            >
              <ArrowLeft className="w-4 h-4" />
            </button>
          )}
          <div className="flex items-center gap-1 text-sm">
            <span className="text-primary font-medium truncate max-w-[200px]">
              {displayPath}
            </span>
          </div>
        </div>

        {/* Search bar and folder select */}
        <div className="flex items-center gap-2">
          <button
            onClick={selectFolder}
            className="px-4 py-2.5 rounded-xl bg-gradient-to-r from-primary to-chart-2 text-white font-medium hover:shadow-lg transition-all flex items-center gap-2 flex-shrink-0"
          >
            <FolderOpen className="w-4 h-4" />
            <span className="text-sm">폴더 선택</span>
          </button>

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
            onClick={() => setViewMode(viewMode === "grid" ? "list" : "grid")}
            className="p-2.5 rounded-xl bg-muted/50 border border-border/50 hover:bg-muted transition-colors"
          >
            {viewMode === "grid" ? (
              <List className="w-4 h-4" />
            ) : (
              <Grid3x3 className="w-4 h-4" />
            )}
          </button>
        </div>
      </div>

      {/* Error Message */}
      {error && (
        <div className="mx-4 mt-4 p-3 rounded-xl bg-destructive/10 border border-destructive/20 text-destructive text-sm">
          {error}
        </div>
      )}

      {/* Files Grid/List */}
      <div className="flex-1 overflow-y-auto px-4 py-4">
        {isLoading ? (
          <div className="flex items-center justify-center py-16">
            <div className="w-8 h-8 border-2 border-primary border-t-transparent rounded-full animate-spin" />
          </div>
        ) : files.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-16 text-center">
            <div className="w-20 h-20 rounded-3xl bg-muted/50 flex items-center justify-center mb-4">
              <FolderOpen className="w-10 h-10 text-muted-foreground" />
            </div>
            <p className="text-muted-foreground">
              {currentPath ? "폴더가 비어있습니다" : "폴더를 선택해주세요"}
            </p>
          </div>
        ) : viewMode === "list" ? (
          <div className="space-y-2">
            {filteredFiles.map((file) => {
              const type = getFileType(file.name, file.isDirectory);
              const Icon = getIcon(type);
              const gradient = getIconColor(type);

              return (
                <div
                  key={file.name}
                  onClick={() =>
                    file.isDirectory && navigateToFolder(file.name)
                  }
                  className={`flex items-center gap-3 p-3 rounded-xl bg-card border border-border hover:shadow-md transition-all ${
                    file.isDirectory ? "cursor-pointer" : ""
                  }`}
                >
                  <div
                    className={`w-11 h-11 rounded-xl bg-gradient-to-br ${gradient} flex items-center justify-center flex-shrink-0 shadow-sm`}
                  >
                    <Icon className="w-5 h-5 text-white" />
                  </div>

                  <div className="flex-1 min-w-0">
                    <p className="truncate font-medium text-sm">{file.name}</p>
                    <p className="text-xs text-muted-foreground mt-0.5">
                      {file.isDirectory ? "폴더" : "파일"}
                    </p>
                  </div>
                </div>
              );
            })}
          </div>
        ) : (
          <div className="grid grid-cols-2 gap-3">
            {filteredFiles.map((file) => {
              const type = getFileType(file.name, file.isDirectory);
              const Icon = getIcon(type);
              const gradient = getIconColor(type);

              return (
                <div
                  key={file.name}
                  onClick={() =>
                    file.isDirectory && navigateToFolder(file.name)
                  }
                  className={`flex flex-col p-3 rounded-2xl bg-card border border-border hover:shadow-md transition-all ${
                    file.isDirectory ? "cursor-pointer" : ""
                  }`}
                >
                  <div
                    className={`w-full aspect-square rounded-xl bg-gradient-to-br ${gradient} flex items-center justify-center mb-3 shadow-sm`}
                  >
                    <Icon className="w-8 h-8 text-white" />
                  </div>

                  <p className="truncate font-medium text-sm mb-1">
                    {file.name}
                  </p>
                  <p className="text-xs text-muted-foreground">
                    {file.isDirectory ? "폴더" : "파일"}
                  </p>
                </div>
              );
            })}
          </div>
        )}

        {filteredFiles.length === 0 && files.length > 0 && (
          <div className="flex flex-col items-center justify-center py-16 text-center">
            <div className="w-20 h-20 rounded-3xl bg-muted/50 flex items-center justify-center mb-4">
              <Search className="w-10 h-10 text-muted-foreground" />
            </div>
            <p className="text-muted-foreground">검색 결과가 없습니다</p>
          </div>
        )}
      </div>

      {/* File Count Footer */}
      {files.length > 0 && (
        <div className="sticky bottom-0 bg-gradient-to-br from-primary/5 to-chart-2/5 border-t border-border/50 px-4 py-3">
          <div className="flex items-center justify-between text-sm">
            <span className="text-muted-foreground">총 항목</span>
            <span className="font-medium">{files.length}개</span>
          </div>
        </div>
      )}
    </div>
  );
}
