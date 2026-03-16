import { load } from "@tauri-apps/plugin-store";

export interface AppSettings {
  deviceName: string;
  sharedFolderPath: string;
  sharedFolderDisplayName: string;
}

export const STORE_FILENAME = ".settings.dat";

export function useSettingsStore() {
  const getStore = async () => {
    return await load(STORE_FILENAME);
  };

  const loadSettings = async (): Promise<AppSettings | null> => {
    try {
      const store = await getStore();
      const deviceName = await store.get<string>("deviceName");
      const sharedFolderPath = await store.get<string>("sharedFolderPath");
      const sharedFolderDisplayName = await store.get<string>("sharedFolderDisplayName");

      if (deviceName && sharedFolderPath && sharedFolderDisplayName) {
        return {
          deviceName,
          sharedFolderPath,
          sharedFolderDisplayName,
        };
      }
      return null;
    } catch (e) {
      console.error("Failed to load settings from store", e);
      return null;
    }
  };

  const saveSettings = async (settings: AppSettings) => {
    try {
      const store = await getStore();
      await store.set("deviceName", settings.deviceName);
      await store.set("sharedFolderPath", settings.sharedFolderPath);
      await store.set("sharedFolderDisplayName", settings.sharedFolderDisplayName);
      await store.save();
    } catch (e) {
      console.error("Failed to save settings to store", e);
    }
  };

  return { loadSettings, saveSettings };
}
