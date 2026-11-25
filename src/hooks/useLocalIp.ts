import { useEffect, useState } from "react";
import { getLocalIp } from "../adapters/tauri/commands";
import { describeError } from "../adapters/tauri/errors";

export function useLocalIp() {
  const [localIp, setLocalIp] = useState<string>("로딩 중...");

  useEffect(() => {
    async function fetchLocalIp() {
      try {
        setLocalIp(await getLocalIp());
      } catch (e) {
        console.error("IP 조회 실패:", describeError(e));
        setLocalIp("알 수 없음");
      }
    }
    fetchLocalIp();
  }, []);

  return localIp;
}
