import { useEffect, useState } from "react";
import { getActivityHistory, getSharedFolderStats } from "../adapters/tauri/commands";
import { describeError } from "../adapters/tauri/errors";
import type { ActivityRecord, SharedFolderStats } from "../core/contract";

const ACTIVITY_REFRESH_MS = 5000;

export function useActivityData() {
  const [activities, setActivities] = useState<ActivityRecord[]>([]);
  const [folderStats, setFolderStats] = useState<SharedFolderStats | null>(null);

  useEffect(() => {
    const loadData = async () => {
      try {
        const [activityData, statsData] = await Promise.all([getActivityHistory(), getSharedFolderStats()]);
        setActivities(activityData);
        setFolderStats(statsData);
      } catch (error) {
        console.error("활동 데이터 불러오기 실패:", describeError(error));
      }
    };

    loadData();
    const interval = setInterval(loadData, ACTIVITY_REFRESH_MS);
    return () => clearInterval(interval);
  }, []);

  return { activities, folderStats };
}
