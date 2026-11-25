import { load } from "@tauri-apps/plugin-store";
import type { AppSettings } from "../../core/settings";
import { describeError } from "./errors";

const STORE_FILENAME = ".settings.dat";

export async function loadSettings(): Promise<AppSettings | null> {
  try {
    const store = await load(STORE_FILENAME);
    const deviceName = await store.get<string>("deviceName");
    const sharedFolderPath = await store.get<string>("sharedFolderPath");
    const sharedFolderDisplayName = await store.get<string>("sharedFolderDisplayName");

    if (deviceName && sharedFolderPath && sharedFolderDisplayName) {
      return { deviceName, sharedFolderPath, sharedFolderDisplayName };
    }
    return null;
  } catch (e) {
    console.error("설정 불러오기 실패:", describeError(e));
    return null;
  }
}

export async function saveSettings(settings: AppSettings): Promise<void> {
  const store = await load(STORE_FILENAME);
  await store.set("deviceName", settings.deviceName);
  await store.set("sharedFolderPath", settings.sharedFolderPath);
  await store.set("sharedFolderDisplayName", settings.sharedFolderDisplayName);
  await store.save();
}

export async function saveSettingsOrAlert(settings: AppSettings): Promise<boolean> {
  try {
    await saveSettings(settings);
    return true;
  } catch (e) {
    const reason = describeError(e);
    console.error("설정 저장 실패:", reason);
    alert(`설정을 저장하지 못했습니다. 다음 실행 때 다시 물어볼 수 있습니다.\n${reason}`);
    return false;
  }
}
