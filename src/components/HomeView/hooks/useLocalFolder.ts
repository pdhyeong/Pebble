import { useCallback, useEffect, useState } from "react";
import { onEvent } from "../../../adapters/tauri/events";
import { useP2pContext } from "../../../contexts/P2pContext";

function parentOf(path: string): string {
  const parts = path.split("/").filter((p) => p);
  parts.pop();
  return parts.length === 0 ? "/" : "/" + parts.join("/");
}

export function useLocalFolder() {
  const { localFiles, sharedFolderPath, refreshLocalFiles, uploadFile } = useP2pContext();
  const [currentPath, setCurrentPath] = useState("/");

  useEffect(
    () => onEvent("local-files-changed", () => refreshLocalFiles(currentPath)),
    [currentPath, refreshLocalFiles],
  );

  const openFolder = useCallback(
    (folderPath: string) => {
      setCurrentPath(folderPath);
      refreshLocalFiles(folderPath);
    },
    [refreshLocalFiles],
  );

  const refresh = useCallback(() => refreshLocalFiles(currentPath), [currentPath, refreshLocalFiles]);

  const sendFiles = useCallback(
    async (peerId: string, fileNames: string[]) => {
      for (const fileName of fileNames) {
        const file = localFiles.find((f) => f.name === fileName);
        if (file && !file.is_dir) {
          const absolutePath = sharedFolderPath + file.path;
          await uploadFile(peerId, absolutePath, "/");
        }
      }
    },
    [localFiles, sharedFolderPath, uploadFile],
  );

  return {
    currentPath,
    parentPath: currentPath === "/" ? null : parentOf(currentPath),
    openFolder,
    refresh,
    sendFiles,
  };
}
