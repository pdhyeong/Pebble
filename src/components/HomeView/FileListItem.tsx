import { Check, ArrowLeft } from "lucide-react";
import { getFileIconByName, getFileColorByName, formatFileSize } from "../../utils/fileUtils";
import type { FileInfo } from "./types";

interface FileListItemProps {
  file: FileInfo;
  isSelected: boolean;
  showCheckbox: boolean;
  onSelect: () => void;
  onFolderClick?: () => void;
}

export function FileListItem({
  file,
  isSelected,
  showCheckbox,
  onSelect,
  onFolderClick,
}: FileListItemProps) {
  const Icon = getFileIconByName(file.name, file.is_dir);
  const gradient = getFileColorByName(file.name, file.is_dir);

  const handleClick = () => {
    if (file.is_dir && onFolderClick) {
      onFolderClick();
    } else {
      onSelect();
    }
  };

  return (
    <div
      onClick={handleClick}
      className={`flex items-center gap-3 p-3 rounded-xl bg-card border transition-all cursor-pointer hover:scale-[1.01] ${
        isSelected
          ? "border-primary bg-primary/5 shadow-md"
          : "border-border hover:shadow-md"
      }`}
    >
      {showCheckbox && !file.is_dir && (
        <div
          className={`w-5 h-5 rounded-md border-2 flex items-center justify-center flex-shrink-0 transition-colors ${
            isSelected ? "bg-primary border-primary" : "border-border"
          }`}
        >
          {isSelected && <Check className="w-3 h-3 text-white" />}
        </div>
      )}

      <div
        className={`w-11 h-11 rounded-xl bg-gradient-to-br ${gradient} flex items-center justify-center flex-shrink-0 shadow-sm`}
      >
        <Icon className="w-5 h-5 text-white" />
      </div>

      <div className="flex-1 min-w-0">
        <p className="truncate font-medium text-sm">{file.name}</p>
        <p className="text-xs text-muted-foreground mt-0.5">
          {file.is_dir ? "폴더" : formatFileSize(file.size)}
        </p>
      </div>

      {file.is_dir && (
        <ArrowLeft className="w-4 h-4 text-muted-foreground rotate-180" />
      )}
    </div>
  );
}
