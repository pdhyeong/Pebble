import { useCallback, useEffect, useState } from "react";
import { applySettingsToBackend } from "../adapters/tauri/commands";
import { describeError } from "../adapters/tauri/errors";
import { loadSettings, saveSettings } from "../adapters/tauri/settingsStore";
import type { AppSettings } from "../core/settings";

async function pushSettings(settings: AppSettings) {
  try {
    await applySettingsToBackend(settings);
  } catch (e) {
    console.error("설정 백엔드 동기화 실패:", describeError(e));
  }
}

export function useAppSetup() {
  const [isReady, setIsReady] = useState(false);
  const [needsSetup, setNeedsSetup] = useState(true);

  useEffect(() => {
    async function init() {
      const settings = await loadSettings();
      if (settings) {
        await pushSettings(settings);
        setNeedsSetup(false);
      }
      setIsReady(true);
    }
    init();
  }, []);

  const completeSetup = useCallback(async (settings: AppSettings) => {
    await saveSettings(settings);
    await pushSettings(settings);
    setNeedsSetup(false);
  }, []);

  return { isReady, needsSetup, completeSetup };
}
