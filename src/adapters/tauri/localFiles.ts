import { open } from "@tauri-apps/plugin-dialog";
import { stat } from "@tauri-apps/plugin-fs";

export async function pickFolder(): Promise<string | null> {
  const selected = await open({
    directory: true,
    multiple: false,
    title: "공유 폴더 선택",
  });
  return typeof selected === "string" ? selected : null;
}

export async function pickFiles(): Promise<string[]> {
  const selected = await open({
    multiple: true,
    title: "전송할 파일 선택",
  });
  if (!selected) return [];
  return Array.isArray(selected) ? selected : [selected];
}

export async function getFileSize(path: string): Promise<number> {
  const metadata = await stat(path);
  return metadata.size;
}
