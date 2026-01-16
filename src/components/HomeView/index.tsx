import { useState } from "react";
import { useP2pContext } from "../../contexts/P2pContext";
import { MainHomeView } from "./MainHomeView";
import { LocalFilesView } from "./LocalFilesView";
import { RemoteFilesView } from "./RemoteFilesView";
import type { ViewMode } from "./types";

export function HomeView() {
  const [viewMode, setViewMode] = useState<ViewMode>("home");
  const [selectedPeerId, setSelectedPeerId] = useState<string | null>(null);

  const { requestFileList, refreshLocalFiles } = useP2pContext();

  // 로컬 파일 뷰로 전환
  const handleOpenLocalFiles = () => {
    setViewMode("local");
    refreshLocalFiles("/");
  };

  // 원격 파일 뷰로 전환
  const handleSelectPeer = (peerId: string) => {
    setSelectedPeerId(peerId);
    setViewMode("remote");
    requestFileList(peerId, "/");
  };

  // 홈으로 돌아가기
  const handleBackToHome = () => {
    setViewMode("home");
    setSelectedPeerId(null);
  };

  // 뷰 모드에 따른 렌더링
  if (viewMode === "local") {
    return <LocalFilesView onBack={handleBackToHome} />;
  }

  if (viewMode === "remote" && selectedPeerId) {
    return <RemoteFilesView peerId={selectedPeerId} onBack={handleBackToHome} />;
  }

  return (
    <MainHomeView
      onOpenLocalFiles={handleOpenLocalFiles}
      onSelectPeer={handleSelectPeer}
    />
  );
}
