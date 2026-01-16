import { Home, Activity, Smartphone } from "lucide-react";

interface BottomNavProps {
  currentTab: "home" | "activity" | "devices";
  onTabChange: (tab: "home" | "activity" | "devices") => void;
}

export function BottomNav({ currentTab, onTabChange }: BottomNavProps) {
  const tabs = [
    { id: "home" as const, label: "홈", icon: Home },
    { id: "activity" as const, label: "활동", icon: Activity },
    { id: "devices" as const, label: "기기", icon: Smartphone },
  ];

  return (
    <nav className="fixed bottom-0 left-0 right-0 z-50 bg-background/95 backdrop-blur-xl border-t border-border/50">
      <div className="px-4">
        <div className="flex items-center justify-around py-3">
          {tabs.map((tab) => {
            const Icon = tab.icon;
            const isActive = currentTab === tab.id;

            return (
              <button
                key={tab.id}
                onClick={() => onTabChange(tab.id)}
                className={`flex flex-col items-center gap-1 px-4 py-2 rounded-xl transition-all ${
                  isActive
                    ? "text-primary"
                    : "text-muted-foreground hover:text-foreground"
                }`}
              >
                <div className={`relative ${isActive ? "scale-110" : ""} transition-transform`}>
                  <Icon className="w-5 h-5" />
                  {isActive && (
                    <div className="absolute -bottom-1 left-1/2 -translate-x-1/2 w-1 h-1 rounded-full bg-primary" />
                  )}
                </div>
                <span className="text-xs font-medium">{tab.label}</span>
              </button>
            );
          })}
        </div>
      </div>
    </nav>
  );
}
