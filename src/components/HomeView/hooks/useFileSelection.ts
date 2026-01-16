import { useState, useCallback } from "react";

export interface UseFileSelectionReturn {
  selectedFiles: string[];
  isSelectionMode: boolean;
  toggleSelection: (fileName: string, isDir: boolean) => void;
  cancelSelection: () => void;
  clearSelection: () => void;
}

export function useFileSelection(): UseFileSelectionReturn {
  const [selectedFiles, setSelectedFiles] = useState<string[]>([]);
  const [isSelectionMode, setIsSelectionMode] = useState(false);

  const toggleSelection = useCallback(
    (fileName: string, isDir: boolean) => {
      // 폴더는 선택 불가
      if (isDir) return;

      if (!isSelectionMode) {
        setIsSelectionMode(true);
        setSelectedFiles([fileName]);
      } else {
        setSelectedFiles((prev) => {
          if (prev.includes(fileName)) {
            const newSelection = prev.filter((f) => f !== fileName);
            if (newSelection.length === 0) {
              setIsSelectionMode(false);
            }
            return newSelection;
          }
          return [...prev, fileName];
        });
      }
    },
    [isSelectionMode]
  );

  const cancelSelection = useCallback(() => {
    setSelectedFiles([]);
    setIsSelectionMode(false);
  }, []);

  return {
    selectedFiles,
    isSelectionMode,
    toggleSelection,
    cancelSelection,
    clearSelection: cancelSelection,
  };
}
