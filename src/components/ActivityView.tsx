import { motion } from "motion/react";
import { Upload, Download, Wifi, HardDrive } from "lucide-react";
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface Activity {
  id: string;
  transfer_type: string; // "upload" or "download"
  file_name: string;
  peer_id: string;
  device_name: string | null;
  size: number;
  timestamp: number;
  status: string; // "completed", "failed", "cancelled"
  speed: number | null;
}

interface SharedFolderStats {
  total_size: number;
  file_count: number;
}

export function ActivityView() {
  const [activities, setActivities] = useState<Activity[]>([]);
  const [folderStats, setFolderStats] = useState<SharedFolderStats | null>(null);

  // 데이터 로드
  useEffect(() => {
    loadData();
    const interval = setInterval(loadData, 5000); // 5초마다 갱신
    return () => clearInterval(interval);
  }, []);

  const loadData = async () => {
    try {
      const [activityData, statsData] = await Promise.all([
        invoke<Activity[]>("get_activity_history"),
        invoke<SharedFolderStats>("get_shared_folder_stats"),
      ]);
      setActivities(activityData);
      setFolderStats(statsData);
    } catch (error) {
      console.error("Failed to load activity data:", error);
    }
  };

  // 평균 속도 계산
  const avgSpeed = activities.length > 0
    ? activities
      .filter(a => a.speed !== null && a.status === "completed")
      .reduce((sum, a) => sum + (a.speed || 0), 0) / activities.filter(a => a.speed !== null).length
    : 0;

  // 파일 크기 포맷
  const formatSize = (bytes: number) => {
    if (bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB", "TB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${(bytes / Math.pow(k, i)).toFixed(2)} ${sizes[i]}`;
  };

  // 속도 포맷
  const formatSpeed = (bytesPerSec: number) => {
    return `${formatSize(bytesPerSec)}/s`;
  };

  // 시간 포맷
  const formatTime = (timestamp: number) => {
    const now = Date.now() / 1000;
    const diff = now - timestamp;

    if (diff < 60) return "방금 전";
    if (diff < 3600) return `${Math.floor(diff / 60)}분 전`;
    if (diff < 86400) return `${Math.floor(diff / 3600)}시간 전`;
    return `${Math.floor(diff / 86400)}일 전`;
  };

  return (
    <div className="flex flex-col h-full">
      {/* Statistics Cards */}
      <div className="sticky top-16 z-40 bg-background/95 backdrop-blur-xl border-b border-border/50 px-4 py-4">
        <h2 className="mb-4">통계</h2>

        <div className="grid grid-cols-2 gap-3 mb-4">
          <motion.div
            initial={{ opacity: 0, y: 10 }}
            animate={{ opacity: 1, y: 0 }}
            className="p-4 rounded-2xl bg-gradient-to-br from-purple-500/10 to-indigo-500/10 border border-purple-500/20"
          >
            <div className="flex items-center gap-2 mb-2">
              <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-purple-400 to-indigo-400 flex items-center justify-center">
                <Wifi className="w-4 h-4 text-white" />
              </div>
              <span className="text-xs text-muted-foreground">평균 속도</span>
            </div>
            <p className="text-xl font-bold">{formatSpeed(avgSpeed)}</p>
          </motion.div>

          <motion.div
            initial={{ opacity: 0, y: 10 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 0.05 }}
            className="p-4 rounded-2xl bg-gradient-to-br from-pink-500/10 to-rose-500/10 border border-pink-500/20"
          >
            <div className="flex items-center gap-2 mb-2">
              <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-pink-400 to-rose-400 flex items-center justify-center">
                <HardDrive className="w-4 h-4 text-white" />
              </div>
              <span className="text-xs text-muted-foreground">공유 폴더</span>
            </div>
            <p className="text-xl font-bold">
              {folderStats ? formatSize(folderStats.total_size) : "로딩..."}
            </p>
            <p className="text-xs text-muted-foreground mt-1">
              {folderStats ? `${folderStats.file_count}개 파일` : ""}
            </p>
          </motion.div>
        </div>
      </div>

      {/* Activity List */}
      <div className="flex-1 overflow-y-auto px-4 py-4">
        <h3 className="mb-3">최근 활동</h3>

        {activities.length === 0 ? (
          <div className="text-center text-muted-foreground py-8">
            아직 전송 기록이 없습니다
          </div>
        ) : (
          <div className="space-y-2">
            {activities.slice().reverse().map((activity, index) => (
              <motion.div
                key={activity.id}
                initial={{ opacity: 0, x: -20 }}
                animate={{ opacity: 1, x: 0 }}
                transition={{ delay: index * 0.05 }}
                className={`p-3 rounded-xl border transition-colors ${activity.status === "completed"
                    ? "bg-card/50 border-border/50 hover:bg-card"
                    : activity.status === "failed"
                      ? "bg-destructive/10 border-destructive/20"
                      : "bg-muted/30 border-muted"
                  }`}
              >
                <div className="flex items-center gap-3">
                  <div
                    className={`w-10 h-10 rounded-lg flex items-center justify-center ${activity.transfer_type === "download"
                        ? "bg-blue-500/20"
                        : "bg-green-500/20"
                      }`}
                  >
                    {activity.transfer_type === "download" ? (
                      <Download className="w-5 h-5 text-blue-400" />
                    ) : (
                      <Upload className="w-5 h-5 text-green-400" />
                    )}
                  </div>

                  <div className="flex-1 min-w-0">
                    <p className="font-medium truncate">{activity.file_name}</p>
                    <div className="flex items-center gap-2 text-xs text-muted-foreground">
                      <span>{activity.device_name || activity.peer_id.slice(0, 8)}</span>
                      <span>•</span>
                      <span>{formatSize(activity.size)}</span>
                      <span>•</span>
                      <span>{formatTime(activity.timestamp)}</span>
                    </div>
                  </div>

                  <div className="text-right">
                    <p
                      className={`text-xs font-medium ${activity.status === "completed"
                          ? "text-green-500"
                          : activity.status === "failed"
                            ? "text-destructive"
                            : "text-muted-foreground"
                        }`}
                    >
                      {activity.status === "completed"
                        ? "완료"
                        : activity.status === "failed"
                          ? "실패"
                          : "취소됨"}
                    </p>
                    {activity.speed && activity.status === "completed" && (
                      <p className="text-xs text-muted-foreground">
                        {formatSpeed(activity.speed)}
                      </p>
                    )}
                  </div>
                </div>
              </motion.div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
