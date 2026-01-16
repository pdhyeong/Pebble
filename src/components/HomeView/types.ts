export type ViewMode = "home" | "local" | "remote";

export type DisplayMode = "grid" | "list";

export interface FileInfo {
  name: string;
  path: string;
  is_dir: boolean;
  size: number;
}

export interface HomeViewState {
  viewMode: ViewMode;
  selectedPeerId: string | null;
}

export interface FileViewState {
  displayMode: DisplayMode;
  searchQuery: string;
  currentPath: string;
}
