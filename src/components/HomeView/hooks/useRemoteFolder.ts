import { useCallback, useEffect } from "react";
import { describeError } from "../../../adapters/tauri/errors";
import { onEvent } from "../../../adapters/tauri/events";
import { pickFiles } from "../../../adapters/tauri/localFiles";
import { useP2pContext } from "../../../contexts/P2pContext";

export function useRemoteFolder(peerId: string) {
  const { remoteFiles, requestFileList, downloadFile, uploadFile } = useP2pContext();
  const currentPath = remoteFiles?.path || "/";

  useEffect(
    () =>
      onEvent("remote-directory-changed", (payload) => {
        if (payload.peer_id === peerId) {
          requestFileList(peerId, currentPath);
        }
      }),
    [peerId, currentPath, requestFileList],
  );

  const openFolder = useCallback(
    (folderPath: string) => requestFileList(peerId, folderPath),
    [peerId, requestFileList],
  );

  const refreshRoot = useCallback(() => requestFileList(peerId, "/"), [peerId, requestFileList]);

  const downloadFiles = useCallback(
    async (fileNames: string[]) => {
      for (const fileName of fileNames) {
        const file = remoteFiles?.files.find((f) => f.name === fileName);
        if (file && !file.is_dir) {
          await downloadFile(peerId, file.path);
        }
      }
    },
    [peerId, remoteFiles, downloadFile],
  );

  const uploadFromPC = useCallback(async () => {
    try {
      const files = await pickFiles();
      for (const filePath of files) {
        await uploadFile(peerId, filePath, currentPath);
      }
    } catch (error) {
      console.error("파일 선택 실패:", describeError(error));
    }
  }, [peerId, currentPath, uploadFile]);

  return { currentPath, openFolder, refreshRoot, downloadFiles, uploadFromPC };
}
