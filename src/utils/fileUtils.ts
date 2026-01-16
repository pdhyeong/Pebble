import { Folder, Image, Film, Music, FileText, File, type LucideIcon } from "lucide-react";

const IMAGE_EXTENSIONS = ["jpg", "jpeg", "png", "gif", "webp", "svg"];
const VIDEO_EXTENSIONS = ["mp4", "avi", "mov", "mkv", "webm"];
const AUDIO_EXTENSIONS = ["mp3", "wav", "flac", "aac", "m4a"];
const DOCUMENT_EXTENSIONS = ["pdf", "doc", "docx", "txt", "md"];

/**
 * 파일명과 디렉토리 여부에 따라 적절한 아이콘 반환
 */
export function getFileIconByName(fileName: string, isDir: boolean): LucideIcon {
  if (isDir) return Folder;

  const ext = fileName.split(".").pop()?.toLowerCase() || "";

  if (IMAGE_EXTENSIONS.includes(ext)) return Image;
  if (VIDEO_EXTENSIONS.includes(ext)) return Film;
  if (AUDIO_EXTENSIONS.includes(ext)) return Music;
  if (DOCUMENT_EXTENSIONS.includes(ext)) return FileText;

  return File;
}

/**
 * 파일명과 디렉토리 여부에 따라 적절한 그라디언트 색상 반환
 */
export function getFileColorByName(fileName: string, isDir: boolean): string {
  if (isDir) return "from-blue-400 to-cyan-400";

  const ext = fileName.split(".").pop()?.toLowerCase() || "";

  if (IMAGE_EXTENSIONS.includes(ext)) return "from-pink-400 to-rose-500";
  if (VIDEO_EXTENSIONS.includes(ext)) return "from-purple-400 to-indigo-500";
  if (AUDIO_EXTENSIONS.includes(ext)) return "from-orange-400 to-yellow-500";
  if (DOCUMENT_EXTENSIONS.includes(ext)) return "from-blue-400 to-blue-500";

  return "from-gray-400 to-gray-500";
}

/**
 * 바이트 단위 파일 크기를 읽기 쉬운 형식으로 변환
 */
export function formatFileSize(bytes: number): string {
  if (bytes === 0) return "0 B";

  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));

  return `${(bytes / Math.pow(k, i)).toFixed(1)} ${sizes[i]}`;
}
