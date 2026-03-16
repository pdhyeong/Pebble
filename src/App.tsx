import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Header } from "./components/Header";
import { HomeView } from "./components/HomeView/index";
import { ActivityView } from "./components/ActivityView";
import { DevicesView } from "./components/DevicesView/index";
import { BottomNav } from "./components/BottomNav";
import { P2pProvider, useP2pContext } from "./contexts/P2pContext";
import PairingModal from "./components/PairingModal";
import { TransferProgressBar } from "./components/Transfer/TransferProgressBar";
import { InitialSetup } from "./components/InitialSetup";
import { useSettingsStore, type AppSettings } from "./hooks/useSettingsStore";

import { SharedFolderModal } from "./components/SharedFolderModal";

function AppContent() {
  const [currentTab, setCurrentTab] = useState<"home" | "activity" | "devices">("home");
  const [showSettingsModal, setShowSettingsModal] = useState(false);
  
  const { 
    activeTransfers,
    myDeviceName,
    sharedFolderPath,
    sharedFolderDisplayName,
    setMyDeviceName,
    setSharedFolder,
    setSharedFolderDisplayName
  } = useP2pContext();
  
  const { saveSettings } = useSettingsStore();

  // 전송 취소 핸들러
  const handleCancelTransfer = async (transferId: string) => {
    try {
      await invoke("cancel_transfer", { transferId });
    } catch (error) {
      console.error("전송 취소 실패:", error);
    }
  };

  const handleSaveSettings = async (newPath: string, newDeviceName: string, newDisplayName: string) => {
    // 1. 상태 업데이트 및 백엔드 반영
    await setSharedFolder(newPath);
    await setMyDeviceName(newDeviceName);
    await setSharedFolderDisplayName(newDisplayName);
    
    // 2. 영구 저장소에 저장 (다음 앱 시작 시 유지되도록)
    await saveSettings({
      deviceName: newDeviceName,
      sharedFolderPath: newPath,
      sharedFolderDisplayName: newDisplayName,
    });
  };

  // Map을 배열로 변환
  const transfersArray = Array.from(activeTransfers.values());

  return (
    <div className="min-h-screen bg-background text-foreground">
      <Header onOpenSettings={() => setShowSettingsModal(true)} />

      <main className="pb-20">
        {currentTab === "home" && <HomeView />}
        {currentTab === "activity" && <ActivityView />}
        {currentTab === "devices" && <DevicesView />}
      </main>

      <BottomNav currentTab={currentTab} onTabChange={setCurrentTab} />

      <PairingModal />
      
      <SharedFolderModal
        isOpen={showSettingsModal}
        onClose={() => setShowSettingsModal(false)}
        currentPath={sharedFolderPath}
        currentDeviceName={myDeviceName}
        currentDisplayName={sharedFolderDisplayName}
        onSave={handleSaveSettings}
      />

      {/* Transfer Progress Bars - 병렬 전송 지원 */}
      <div className="fixed bottom-16 left-0 right-0 z-50 flex flex-col gap-1 px-2">
        {transfersArray.map((transfer, index) => (
          <TransferProgressBar
            key={transfer.transferId}
            type={transfer.type}
            fileName={transfer.fileName}
            progress={transfer.progress}
            bytesTransferred={transfer.bytesTransferred}
            totalBytes={transfer.totalBytes}
            speed={transfer.speed}
            transferId={transfer.transferId}
            onCancel={() => handleCancelTransfer(transfer.transferId)}
            isStacked={transfersArray.length > 1}
            stackIndex={index}
          />
        ))}
      </div>
    </div>
  );
}

function App() {
  const [isReady, setIsReady] = useState(false);
  const [needsSetup, setNeedsSetup] = useState(true);
  const { loadSettings, saveSettings } = useSettingsStore();

  useEffect(() => {
    async function init() {
      const settings = await loadSettings();
      if (settings) {
        // 백엔드 상태와 동기화
        try {
          await invoke("set_device_name", { name: settings.deviceName });
          await invoke("set_shared_folder", { path: settings.sharedFolderPath });
          await invoke("set_shared_folder_display_name", { name: settings.sharedFolderDisplayName });
        } catch (e) {
          console.error("설정 백엔드 동기화 실패:", e);
        }
        setNeedsSetup(false);
      }
      setIsReady(true);
    }
    init();
  }, []);

  const handleSetupComplete = async (settings: AppSettings) => {
    await saveSettings(settings);
    try {
      await invoke("set_device_name", { name: settings.deviceName });
      await invoke("set_shared_folder", { path: settings.sharedFolderPath });
      await invoke("set_shared_folder_display_name", { name: settings.sharedFolderDisplayName });
    } catch (e) {
      console.error("초기 설정 백엔드 동기화 실패:", e);
    }
    setNeedsSetup(false);
  };

  if (!isReady) {
    return (
      <div className="min-h-screen bg-background flex items-center justify-center">
        <div className="animate-pulse flex flex-col items-center gap-4">
          <div className="w-12 h-12 bg-primary/20 rounded-full" />
          <p className="text-muted-foreground text-sm font-medium">로딩 중...</p>
        </div>
      </div>
    );
  }

  if (needsSetup) {
    return <InitialSetup onComplete={handleSetupComplete} />;
  }

  return (
    <P2pProvider>
      <AppContent />
    </P2pProvider>
  );
}

export default App;
