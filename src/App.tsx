import { useState } from "react";
import { Header } from "./components/Header";
import { HomeView } from "./components/HomeView";
import { FilesView } from "./components/FilesView";
import { ActivityView } from "./components/ActivityView";
import { DevicesView } from "./components/DevicesView/index";
import { BottomNav } from "./components/BottomNav";
import { P2pProvider } from "./contexts/P2pContext";

function App() {
  const [currentTab, setCurrentTab] = useState<"home" | "files" | "activity" | "devices">("home");

  return (
    <P2pProvider>
      <div className="min-h-screen bg-background text-foreground">
        <Header />

        <main className="pb-20">
          {currentTab === "home" && <HomeView />}
          {currentTab === "files" && <FilesView />}
          {currentTab === "activity" && <ActivityView />}
          {currentTab === "devices" && <DevicesView />}
        </main>

        <BottomNav currentTab={currentTab} onTabChange={setCurrentTab} />
      </div>
    </P2pProvider>
  );
}

export default App;
