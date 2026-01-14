import { motion } from "motion/react";
import { Upload, Download, Clock, Wifi, TrendingUp, HardDrive } from "lucide-react";
import { useState } from "react";

interface Activity {
  id: string;
  type: "upload" | "download";
  fileName: string;
  deviceName: string;
  size: string;
  time: string;
  status: "completed" | "in-progress" | "failed";
}

export function ActivityView() {
  const [timeFilter, setTimeFilter] = useState<"today" | "week" | "month">("today");

  const activities: Activity[] = [
    {
      id: "1",
      type: "download",
      fileName: "프로젝트_제안서.pdf",
      deviceName: "내 아이폰",
      size: "2.4 MB",
      time: "5분 전",
      status: "completed"
    },
    {
      id: "2",
      type: "upload",
      fileName: "vacation_photos.zip",
      deviceName: "맥북 프로",
      size: "156 MB",
      time: "15분 전",
      status: "completed"
    },
    {
      id: "3",
      type: "download",
      fileName: "presentation.pptx",
      deviceName: "내 아이폰",
      size: "12.5 MB",
      time: "2시간 전",
      status: "completed"
    },
    {
      id: "4",
      type: "upload",
      fileName: "design_assets.zip",
      deviceName: "맥북 프로",
      size: "89 MB",
      time: "3시간 전",
      status: "completed"
    },
    {
      id: "5",
      type: "download",
      fileName: "video_tutorial.mp4",
      deviceName: "내 아이폰",
      size: "340 MB",
      time: "어제",
      status: "failed"
    },
  ];

  const stats = {
    totalTransferred: "156.8 GB",
    avgSpeed: "45.2 MB/s",
    activeTime: "24시간 18분",
    storageUsed: "256 GB",
    storageTotal: "512 GB",
    storagePercentage: 50
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
            className="p-4 rounded-2xl bg-gradient-to-br from-blue-500/10 to-cyan-500/10 border border-blue-500/20"
          >
            <div className="flex items-center gap-2 mb-2">
              <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-blue-400 to-cyan-400 flex items-center justify-center">
                <TrendingUp className="w-4 h-4 text-white" />
              </div>
              <span className="text-xs text-muted-foreground">총 전송량</span>
            </div>
            <p className="text-xl font-bold">{stats.totalTransferred}</p>
          </motion.div>

          <motion.div
            initial={{ opacity: 0, y: 10 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 0.05 }}
            className="p-4 rounded-2xl bg-gradient-to-br from-purple-500/10 to-indigo-500/10 border border-purple-500/20"
          >
            <div className="flex items-center gap-2 mb-2">
              <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-purple-400 to-indigo-400 flex items-center justify-center">
                <Wifi className="w-4 h-4 text-white" />
              </div>
              <span className="text-xs text-muted-foreground">평균 속도</span>
            </div>
            <p className="text-xl font-bold">{stats.avgSpeed}</p>
          </motion.div>

          <motion.div
            initial={{ opacity: 0, y: 10 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 0.1 }}
            className="p-4 rounded-2xl bg-gradient-to-br from-orange-500/10 to-yellow-500/10 border border-orange-500/20"
          >
            <div className="flex items-center gap-2 mb-2">
              <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-orange-400 to-yellow-400 flex items-center justify-center">
                <Clock className="w-4 h-4 text-white" />
              </div>
              <span className="text-xs text-muted-foreground">활성 시간</span>
            </div>
            <p className="text-xl font-bold">{stats.activeTime}</p>
          </motion.div>

          <motion.div
            initial={{ opacity: 0, y: 10 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 0.15 }}
            className="p-4 rounded-2xl bg-gradient-to-br from-pink-500/10 to-rose-500/10 border border-pink-500/20"
          >
            <div className="flex items-center gap-2 mb-2">
              <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-pink-400 to-rose-400 flex items-center justify-center">
                <HardDrive className="w-4 h-4 text-white" />
              </div>
              <span className="text-xs text-muted-foreground">저장공간</span>
            </div>
            <p className="text-xl font-bold">{stats.storageUsed}</p>
            <div className="mt-2 h-1.5 bg-muted/50 rounded-full overflow-hidden">
              <motion.div
                initial={{ width: 0 }}
                animate={{ width: `${stats.storagePercentage}%` }}
                transition={{ duration: 1, ease: "easeOut" }}
                className="h-full bg-gradient-to-r from-pink-400 to-rose-400 rounded-full"
              />
            </div>
          </motion.div>
        </div>

        {/* Time Filter */}
        <div className="flex gap-2 p-1 bg-muted/50 rounded-xl">
          <button
            onClick={() => setTimeFilter("today")}
            className={`flex-1 px-4 py-2 rounded-lg text-sm font-medium transition-all ${
              timeFilter === "today"
                ? "bg-background shadow-sm"
                : "text-muted-foreground hover:text-foreground"
            }`}
          >
            오늘
          </button>
          <button
            onClick={() => setTimeFilter("week")}
            className={`flex-1 px-4 py-2 rounded-lg text-sm font-medium transition-all ${
              timeFilter === "week"
                ? "bg-background shadow-sm"
                : "text-muted-foreground hover:text-foreground"
            }`}
          >
            이번 주
          </button>
          <button
            onClick={() => setTimeFilter("month")}
            className={`flex-1 px-4 py-2 rounded-lg text-sm font-medium transition-all ${
              timeFilter === "month"
                ? "bg-background shadow-sm"
                : "text-muted-foreground hover:text-foreground"
            }`}
          >
            이번 달
          </button>
        </div>
      </div>

      {/* Activities List */}
      <div className="flex-1 overflow-y-auto px-4 py-4">
        <h3 className="mb-3">최근 활동</h3>
        <div className="space-y-2">
          {activities.map((activity, index) => {
            const ActivityIcon = activity.type === "upload" ? Upload : Download;
            const gradient = activity.type === "upload" ? "from-green-400 to-emerald-400" : "from-blue-400 to-cyan-400";

            return (
              <motion.div
                key={activity.id}
                initial={{ opacity: 0, x: -20 }}
                animate={{ opacity: 1, x: 0 }}
                transition={{ delay: index * 0.05 }}
                className="relative"
              >
                {/* Timeline line */}
                {index < activities.length - 1 && (
                  <div className="absolute left-5 top-14 bottom-0 w-px bg-border/50 -z-10" />
                )}

                <div className="flex gap-3">
                  {/* Activity Icon */}
                  <div className={`w-10 h-10 rounded-xl bg-gradient-to-br ${gradient} flex items-center justify-center flex-shrink-0 shadow-md`}>
                    <ActivityIcon className="w-5 h-5 text-white" />
                  </div>

                  {/* Activity Content */}
                  <div className="flex-1 min-w-0 p-3 rounded-xl bg-card border border-border hover:shadow-md transition-shadow">
                    <div className="flex items-start justify-between gap-2 mb-2">
                      <div className="flex-1 min-w-0">
                        <div className="flex items-center gap-2 mb-1">
                          <span className={`text-xs px-2 py-0.5 rounded-full bg-gradient-to-r ${gradient} text-white`}>
                            {activity.type === "upload" ? "업로드" : "다운로드"}
                          </span>
                          {activity.status === "failed" && (
                            <span className="text-xs px-2 py-0.5 rounded-full bg-destructive/10 text-destructive">
                              실패
                            </span>
                          )}
                        </div>
                        <p className="font-medium text-sm truncate">{activity.fileName}</p>
                      </div>
                    </div>

                    <div className="flex items-center gap-2 text-xs text-muted-foreground">
                      <Clock className="w-3 h-3" />
                      <span>{activity.time}</span>
                      <span>•</span>
                      <span>{activity.size}</span>
                      <span>•</span>
                      <span>{activity.deviceName}</span>
                    </div>
                  </div>
                </div>
              </motion.div>
            );
          })}
        </div>

        {activities.length === 0 && (
          <div className="flex flex-col items-center justify-center py-16 text-center">
            <div className="w-20 h-20 rounded-3xl bg-muted/50 flex items-center justify-center mb-4">
              <Clock className="w-10 h-10 text-muted-foreground" />
            </div>
            <p className="text-muted-foreground">활동 내역이 없습니다</p>
          </div>
        )}
      </div>
    </div>
  );
}
