import { useCallback, useEffect, useState } from "react";
import { describeError } from "../../../adapters/tauri/errors";
import { onFileDrop } from "../../../adapters/tauri/events";
import { getFileSize } from "../../../adapters/tauri/localFiles";
import type { DroppedFile } from "../../Transfer/DropConfirmModal";

async function toDroppedFile(path: string): Promise<DroppedFile> {
  const fileName = path.split("/").pop() || path.split("\\").pop() || "unknown";
  try {
    return { name: fileName, path, size: await getFileSize(path), type: "" };
  } catch (e) {
    console.error("파일 메타데이터 가져오기 실패:", describeError(e));
    return { name: fileName, path, size: 0, type: "" };
  }
}

export function useFileDrop() {
  const [isDragging, setIsDragging] = useState(false);
  const [droppedFiles, setDroppedFiles] = useState<DroppedFile[]>([]);
  const [showDropModal, setShowDropModal] = useState(false);

  const showDropped = useCallback((files: DroppedFile[]) => {
    setDroppedFiles(files);
    setShowDropModal(true);
  }, []);

  const onDragEnter = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragging(true);
  };

  const onDragLeave = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    if (e.currentTarget === e.target) {
      setIsDragging(false);
    }
  };

  const onDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
  };

  const onDrop = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragging(false);

    const files = Array.from(e.dataTransfer.files);
    if (files.length === 0) return;

    const validFiles = files
      .map((f) => ({
        name: f.name,
        path: (f as File & { path?: string }).path || "",
        size: f.size,
        type: f.type,
      }))
      .filter((f) => f.path !== "");

    if (validFiles.length === 0) {
      console.error("유효한 파일 경로가 없습니다");
      alert("파일 경로를 가져올 수 없습니다. 브라우저 개발자 도구 콘솔을 확인하세요.");
      return;
    }

    showDropped(validFiles);
  };

  useEffect(
    () =>
      onFileDrop(async (paths) => {
        if (paths.length === 0) return;
        showDropped(await Promise.all(paths.map(toDroppedFile)));
      }),
    [showDropped],
  );

  const closeDropModal = useCallback(() => {
    setShowDropModal(false);
    setDroppedFiles([]);
  }, []);

  return {
    isDragging,
    dragHandlers: { onDragEnter, onDragLeave, onDragOver, onDrop },
    droppedFiles,
    showDropModal,
    closeDropModal,
  };
}
