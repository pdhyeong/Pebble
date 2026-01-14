import { Menu, Settings } from "lucide-react";

export function Header() {
  return (
    <header className="sticky top-0 z-50 backdrop-blur-xl bg-background/80 border-b border-border/50">
      <div className="px-6 py-4 flex items-center justify-between">
        <button className="p-2 rounded-xl hover:bg-muted/50 transition-colors">
          <Menu className="w-5 h-5" />
        </button>

        <div className="absolute left-1/2 -translate-x-1/2 flex items-center gap-2">
          <div className="w-10 h-10 rounded-2xl bg-gradient-to-br from-primary to-chart-2 flex items-center justify-center shadow-lg">
            <span className="text-xl">🪨</span>
          </div>
          <span className="text-lg font-semibold bg-gradient-to-r from-primary to-chart-2 bg-clip-text text-transparent">
            Pebble
          </span>
        </div>

        <button className="p-2 rounded-xl hover:bg-muted/50 transition-colors">
          <Settings className="w-5 h-5" />
        </button>
      </div>
    </header>
  );
}
