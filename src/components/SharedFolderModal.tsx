import { X, FolderOpen, Check, User } from "lucide-react";
import { useState, useEffect } from "react";
import { open } from "@tauri-apps/plugin-dialog";

interface SharedFolderModalProps {
  isOpen: boolean;
  onClose: () => void;
  currentPath: string;
  currentDeviceName: string;
  onSave: (path: string, deviceName: string) => void;
}

export function SharedFolderModal({
  isOpen,
  onClose,
  currentPath,
  currentDeviceName,
  onSave,
}: SharedFolderModalProps) {
  const [folderPath, setFolderPath] = useState(currentPath);
  const [deviceName, setDeviceName] = useState(currentDeviceName);

  useEffect(() => {
    setFolderPath(currentPath);
    setDeviceName(currentDeviceName);
  }, [currentPath, currentDeviceName]);

  if (!isOpen) return null;

  const handleSelectFolder = async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "공유 폴더 선택",
      });
      if (selected && typeof selected === "string") {
        setFolderPath(selected);
      }
    } catch (e) {
      console.error("폴더 선택 실패:", e);
    }
  };

  const handleSave = () => {
    onSave(folderPath, deviceName);
    onClose();
  };

  return (
    <div className="fixed inset-0 z-[100] flex items-center justify-center bg-black/50 backdrop-blur-sm">
      <div className="bg-card border border-border rounded-2xl w-[90%] max-w-md shadow-2xl animate-in fade-in zoom-in-95 duration-200">
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b border-border">
          <h2 className="text-lg font-semibold">공유 설정</h2>
          <button
            onClick={onClose}
            className="p-2 rounded-lg hover:bg-muted/50 transition-colors"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Content */}
        <div className="p-4 space-y-4">
          {/* 기기 이름 설정 */}
          <div className="space-y-2">
            <label className="text-sm font-medium flex items-center gap-2">
              <User className="w-4 h-4" />
              기기 이름
            </label>
            <input
              type="text"
              value={deviceName}
              onChange={(e) => setDeviceName(e.target.value)}
              placeholder="내 기기"
              className="w-full p-3 rounded-xl bg-muted/50 border border-border text-sm focus:outline-none focus:ring-2 focus:ring-primary/50"
            />
            <p className="text-xs text-muted-foreground">
              연결된 기기에서 이 이름이 표시됩니다
            </p>
          </div>

          {/* 공유 폴더 설정 */}
          <div className="space-y-2">
            <label className="text-sm font-medium flex items-center gap-2">
              <FolderOpen className="w-4 h-4" />
              공유 폴더
            </label>
            <div className="flex gap-2">
              <div className="flex-1 p-3 rounded-xl bg-muted/50 border border-border font-mono text-sm truncate">
                {folderPath || "선택된 폴더 없음"}
              </div>
              <button
                onClick={handleSelectFolder}
                className="px-4 py-2 rounded-xl bg-primary text-white font-medium hover:bg-primary/90 transition-colors flex items-center gap-2"
              >
                <FolderOpen className="w-4 h-4" />
                선택
              </button>
            </div>
            <p className="text-xs text-muted-foreground">
              이 폴더의 파일들이 연결된 기기에서 보이게 됩니다
            </p>
          </div>

          <div className="p-3 rounded-xl bg-yellow-500/10 border border-yellow-500/20">
            <p className="text-sm text-yellow-600">
              주의: 공유 폴더의 파일은 연결된 모든 기기에서 접근할 수 있습니다.
            </p>
          </div>
        </div>

        {/* Footer */}
        <div className="flex justify-end gap-2 p-4 border-t border-border">
          <button
            onClick={onClose}
            className="px-4 py-2 rounded-xl bg-muted/50 hover:bg-muted transition-colors"
          >
            취소
          </button>
          <button
            onClick={handleSave}
            className="px-4 py-2 rounded-xl bg-gradient-to-r from-primary to-chart-2 text-white font-medium hover:shadow-lg transition-shadow flex items-center gap-2"
          >
            <Check className="w-4 h-4" />
            저장
          </button>
        </div>
      </div>
    </div>
  );
}
