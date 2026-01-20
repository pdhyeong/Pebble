import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Header } from "./components/Header";
import { HomeView } from "./components/HomeView/index";
import { ActivityView } from "./components/ActivityView";
import { DevicesView } from "./components/DevicesView/index";
import { BottomNav } from "./components/BottomNav";
import { P2pProvider, useP2pContext } from "./contexts/P2pContext";
import PairingModal from "./components/PairingModal";
import { TransferProgressBar } from "./components/Transfer/TransferProgressBar";

function AppContent() {
  const [currentTab, setCurrentTab] = useState<"home" | "activity" | "devices">("home");
  const { activeTransfers } = useP2pContext();

  // 전송 취소 핸들러
  const handleCancelTransfer = async (transferId: string) => {
    try {
      await invoke("cancel_transfer", { transferId });
    } catch (error) {
      console.error("전송 취소 실패:", error);
    }
  };

  // Map을 배열로 변환
  const transfersArray = Array.from(activeTransfers.values());

  return (
    <div className="min-h-screen bg-background text-foreground">
      <Header />

      <main className="pb-20">
        {currentTab === "home" && <HomeView />}
        {currentTab === "activity" && <ActivityView />}
        {currentTab === "devices" && <DevicesView />}
      </main>

      <BottomNav currentTab={currentTab} onTabChange={setCurrentTab} />

      <PairingModal />

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
  return (
    <P2pProvider>
      <AppContent />
    </P2pProvider>
  );
}

export default App;
