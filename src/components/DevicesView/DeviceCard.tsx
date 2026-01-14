import { Smartphone, Monitor, Tablet, Wifi, WifiOff, MoreVertical } from "lucide-react";

export interface Device {
  id: string;
  name: string;
  type: "desktop" | "mobile" | "tablet";
  status: "online" | "offline";
  lastSeen: string;
  ipAddress: string;
  osInfo: string;
}

interface DeviceCardProps {
  device: Device;
}

const deviceIcons = {
  desktop: Monitor,
  mobile: Smartphone,
  tablet: Tablet,
};

const deviceColors = {
  desktop: "from-blue-400 to-cyan-400",
  mobile: "from-purple-400 to-indigo-400",
  tablet: "from-pink-400 to-rose-400",
};

export function DeviceCard({ device }: DeviceCardProps) {
  const Icon = deviceIcons[device.type] || Monitor;
  const gradient = deviceColors[device.type] || "from-gray-400 to-gray-500";
  const isOnline = device.status === "online";

  return (
    <div className="flex items-center gap-4 p-4 rounded-2xl bg-card border border-border hover:shadow-lg transition-all">
      <div
        className={`w-14 h-14 rounded-xl bg-gradient-to-br ${gradient} flex items-center justify-center flex-shrink-0 shadow-md relative`}
      >
        <Icon className="w-7 h-7 text-white" />
        <div
          className={`absolute -bottom-1 -right-1 w-5 h-5 rounded-full border-2 border-card flex items-center justify-center ${
            isOnline ? "bg-green-500" : "bg-gray-400"
          }`}
        >
          {isOnline ? (
            <Wifi className="w-3 h-3 text-white" />
          ) : (
            <WifiOff className="w-3 h-3 text-white" />
          )}
        </div>
      </div>

      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2 mb-1">
          <h4 className="truncate">{device.name}</h4>
          {isOnline && (
            <span className="px-2 py-0.5 rounded-full bg-green-500/10 text-green-600 text-xs font-medium border border-green-500/20">
              온라인
            </span>
          )}
        </div>
        <p className="text-xs text-muted-foreground mb-1">{device.osInfo}</p>
        <div className="flex items-center gap-3 text-xs text-muted-foreground/70">
          <span className="font-mono">{device.ipAddress}</span>
          <span>•</span>
          <span>마지막 접속: {device.lastSeen}</span>
        </div>
      </div>

      <button className="p-2 rounded-lg hover:bg-muted/50 transition-colors flex-shrink-0">
        <MoreVertical className="w-4 h-4 text-muted-foreground" />
      </button>
    </div>
  );
}
