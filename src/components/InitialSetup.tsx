import { useState } from "react";
import { FolderOpen, User, Folder, ArrowRight } from "lucide-react";
import { open } from "@tauri-apps/plugin-dialog";
import { type AppSettings } from "../hooks/useSettingsStore";

interface InitialSetupProps {
  onComplete: (settings: AppSettings) => void;
}

export function InitialSetup({ onComplete }: InitialSetupProps) {
  const [deviceName, setDeviceName] = useState("");
  const [folderPath, setFolderPath] = useState("");
  const [displayName, setDisplayName] = useState("");

  const handleSelectFolder = async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "공유 폴더 선택",
      });
      if (selected && typeof selected === "string") {
        setFolderPath(selected);
        
        // 간단한 초기 폴더 이름 세팅
        if (!displayName) {
            const parts = selected.split(/[/\\]/);
            const lastPart = parts[parts.length - 1];
            setDisplayName(lastPart || "공유 폴더");
        }
      }
    } catch (e) {
      console.error("폴더 선택 실패:", e);
    }
  };

  const isFormValid = deviceName.trim() !== "" && folderPath !== "" && displayName.trim() !== "";

  const handleComplete = () => {
    if (isFormValid) {
      onComplete({
        deviceName: deviceName.trim(),
        sharedFolderPath: folderPath,
        sharedFolderDisplayName: displayName.trim(),
      });
    }
  };

  return (
    <div className="fixed inset-0 z-[200] bg-background overflow-y-auto">
      <div className="min-h-full flex items-center justify-center p-4 py-8">
        <div className="w-full max-w-lg space-y-8 animate-in fade-in slide-in-from-bottom-8 duration-500 bg-card sm:p-8 rounded-2xl sm:shadow-lg sm:border sm:border-border/50">
          <div className="text-center space-y-2">
            <div className="mx-auto w-16 h-16 bg-primary/10 rounded-2xl flex items-center justify-center mb-6 text-primary">
              <FolderOpen className="w-8 h-8" />
            </div>
            <h1 className="text-3xl font-bold tracking-tight">Pebble 기본 설정</h1>
            <p className="text-muted-foreground">
              파일을 안전하게 공유하기 위한 첫 단계입니다.
            </p>
          </div>

          <div className="space-y-6">
          {/* 기기 이름 설정 */}
          <div className="space-y-2">
            <label className="text-sm font-medium flex items-center gap-2">
              <User className="w-4 h-4 text-primary" />
              나의 기기 이름
            </label>
            <input
              type="text"
              value={deviceName}
              onChange={(e) => setDeviceName(e.target.value)}
              placeholder="예: 내 맥북, 사무실 PC"
              className="w-full p-4 rounded-xl bg-muted/30 border border-border text-base focus:outline-none focus:ring-2 focus:ring-primary/50 transition-all"
            />
            <p className="text-xs text-muted-foreground px-1">
              상대방이 나를 식별하는 이름입니다.
            </p>
          </div>

          {/* 공유 폴더 설정 */}
          <div className="space-y-2">
            <label className="text-sm font-medium flex items-center gap-2">
              <FolderOpen className="w-4 h-4 text-primary" />
              기본 공유 폴더
            </label>
            <div className="flex gap-2 relative">
              <div 
                className={`flex-1 p-4 rounded-xl border border-border font-mono text-sm truncate flex items-center cursor-pointer transition-colors ${folderPath ? 'bg-primary/5 border-primary/20 text-foreground' : 'bg-muted/30 text-muted-foreground'}`}
                onClick={handleSelectFolder}
              >
                {folderPath || "폴더를 선택해주세요"}
              </div>
              <button
                onClick={handleSelectFolder}
                className="px-6 rounded-xl bg-primary text-primary-foreground font-medium hover:bg-primary/90 transition-colors shadow-sm"
              >
                선택
              </button>
            </div>
            <p className="text-xs text-muted-foreground px-1">
              이 폴더의 파일들이 P2P로 공유됩니다.
            </p>
          </div>

          {/* 공유 폴더 표시 이름 */}
          <div className="space-y-2">
            <label className="text-sm font-medium flex items-center gap-2">
              <Folder className="w-4 h-4 text-primary" />
              폴더 표시 이름
            </label>
            <input
              type="text"
              value={displayName}
              onChange={(e) => setDisplayName(e.target.value)}
              placeholder="예: 다운로드, 공용 파일"
              className="w-full p-4 rounded-xl bg-muted/30 border border-border text-base focus:outline-none focus:ring-2 focus:ring-primary/50 transition-all"
            />
            <p className="text-xs text-muted-foreground px-1">
              상대방에게 보여질 깔끔한 폴더 이름.
            </p>
          </div>
        </div>

        <button
          onClick={handleComplete}
          disabled={!isFormValid}
          className="w-full p-4 rounded-xl bg-gradient-to-r from-primary to-chart-2 text-white font-bold text-lg hover:shadow-lg transition-all disabled:opacity-50 disabled:hover:shadow-none flex items-center justify-center gap-2 group mt-8"
        >
          <span>Pebble 시작하기</span>
          <ArrowRight className="w-5 h-5 group-hover:translate-x-1 transition-transform" />
        </button>
      </div>
      </div>
    </div>
  );
}
