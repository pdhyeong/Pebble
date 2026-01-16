import { Check } from "lucide-react";
import { getFileIconByName, getFileColorByName, formatFileSize } from "../../utils/fileUtils";
import type { FileInfo } from "./types";

interface FileGridItemProps {
  file: FileInfo;
  isSelected: boolean;
  onSelect: () => void;
  onFolderClick?: () => void;
}

export function FileGridItem({
  file,
  isSelected,
  onSelect,
  onFolderClick,
}: FileGridItemProps) {
  const Icon = getFileIconByName(file.name, file.is_dir);
  const gradient = getFileColorByName(file.name, file.is_dir);

  const handleClick = () => {
    if (file.is_dir && onFolderClick) {
      onFolderClick();
    } else {
      onSelect();
    }
  };

  // 폴더는 선택 표시 안함
  const showSelected = !file.is_dir && isSelected;

  return (
    <div
      onClick={handleClick}
      className={`flex flex-col p-3 rounded-2xl bg-card border transition-all cursor-pointer ${
        showSelected
          ? "border-primary bg-primary/5 shadow-md"
          : "border-border hover:shadow-md"
      }`}
    >
      <div className="relative">
        <div
          className={`w-full aspect-square rounded-xl bg-gradient-to-br ${gradient} flex items-center justify-center mb-3 shadow-sm`}
        >
          <Icon className="w-8 h-8 text-white" />
        </div>
        {showSelected && (
          <div className="absolute top-2 right-2 w-6 h-6 rounded-full bg-primary flex items-center justify-center">
            <Check className="w-4 h-4 text-white" />
          </div>
        )}
      </div>

      <p className="truncate font-medium text-sm mb-1">{file.name}</p>
      <p className="text-xs text-muted-foreground">
        {file.is_dir ? "폴더" : formatFileSize(file.size)}
      </p>
    </div>
  );
}
