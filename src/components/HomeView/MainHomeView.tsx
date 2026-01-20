import { useState } from "react";
import { Folder, FolderOpen, Monitor, Settings, Wifi } from "lucide-react";
import { useP2pContext } from "../../contexts/P2pContext";
import { SharedFolderModal } from "../SharedFolderModal";

interface MainHomeViewProps {
  onOpenLocalFiles: () => void;
  onSelectPeer: (peerId: string) => void;
}

export function MainHomeView({ onOpenLocalFiles, onSelectPeer }: MainHomeViewProps) {
  const [showSettingsModal, setShowSettingsModal] = useState(false);

  const {
    connectedPeers,
    localFiles,
    myDeviceName,
    sharedFolderPath,
    setMyDeviceName,
    setSharedFolder,
  } = useP2pContext();

  const handleSaveSettings = async (newPath: string, newDeviceName: string) => {
    await setSharedFolder(newPath);
    await setMyDeviceName(newDeviceName);
  };

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
          <button
            onClick={() => setShowSettingsModal(true)}
            className="p-2.5 rounded-xl bg-muted/50 border border-border/50 hover:bg-muted transition-colors"
          >
            <Settings className="w-5 h-5 text-muted-foreground" />
          </button>
        </div>
      </div>

      <div className="flex-1 overflow-y-auto px-4 py-4 space-y-6">
        {/* 내 공유폴더 섹션 */}
        <section>
          <h3 className="text-sm font-medium text-muted-foreground mb-3 flex items-center gap-2">
            <FolderOpen className="w-4 h-4" />
            내 공유폴더
          </h3>
          <button
            onClick={onOpenLocalFiles}
            className="w-full p-4 rounded-2xl bg-card border border-border hover:shadow-lg hover:border-blue-500/50 hover:scale-[1.01] transition-all text-left"
          >
            <div className="flex items-center gap-4">
              <div className="w-14 h-14 rounded-xl bg-gradient-to-br from-blue-400 to-cyan-400 flex items-center justify-center flex-shrink-0 shadow-md">
                <Folder className="w-7 h-7 text-white" />
              </div>
              <div className="flex-1 min-w-0">
                <h4 className="font-medium mb-1">{myDeviceName}</h4>
                <p className="text-xs text-muted-foreground font-mono truncate">
                  {sharedFolderPath || "~/Pebble/Shared"}
                </p>
                <p className="text-xs text-muted-foreground mt-1">
                  {localFiles.length}개 파일 공유 중
                </p>
              </div>
              <div className="flex items-center gap-2 text-primary">
                <Folder className="w-5 h-5" />
                <span className="text-sm font-medium">열기</span>
              </div>
            </div>
          </button>
        </section>

        {/* 연결된 기기 섹션 */}
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
                <div
                  key={peer.peerId}
                  onClick={() => onSelectPeer(peer.peerId)}
                  className="p-4 bg-gray-50 dark:bg-gray-700 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-600 cursor-pointer transition-colors"
                >
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-3">
                      <div className="w-10 h-10 bg-blue-500 rounded-full flex items-center justify-center text-white font-bold">
                        {peer.deviceName.charAt(0).toUpperCase()}
                      </div>
                      <div>
                        <div className="font-medium text-gray-900 dark:text-white">
                          {peer.sharedFolderName ? `📁 ${peer.sharedFolderName}` : peer.deviceName}
                        </div>
                        <div className="text-sm text-gray-500 dark:text-gray-400">
                          {peer.peerId.slice(0, 16)}...
                        </div>
                      </div>
                    </div>
                    <svg
                      className="w-5 h-5 text-gray-400"
                      fill="none"
                      stroke="currentColor"
                      viewBox="0 0 24 24"
                    >
                      <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        strokeWidth={2}
                        d="M9 5l7 7-7 7"
                      />
                    </svg>
                  </div>
                </div>
              ))}
            </div>
          )}
        </section>
      </div>

      {/* 공유 폴더 설정 모달 */}
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
