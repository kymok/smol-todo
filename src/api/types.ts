export type TaskStatus =
  | "draft" | "ready" | "in-progress" | "completed" | "on-hold" | "rejected" | "aborted";
export type CollectionColor =
  | "gray" | "red" | "orange" | "yellow" | "green" | "blue" | "purple";

export interface TaskNote { id: string; version: string; body: string; createdAt: string; updatedAt: string; }
export interface TaskItem {
  id: string; version: string; title: string; collection: string;
  note?: TaskNote; status: TaskStatus; createdAt: string; updatedAt: string;
}
export interface CollectionSummary {
  name: string; displayName: string; groupName: string;
  totalCount: number; incompleteCount: number;
  statusIndicator?: TaskStatus; color: CollectionColor; isArchived: boolean; promptTemplate?: string;
}
export interface CollectionGroupSummary { name: string; collections: CollectionSummary[]; }
export interface Snapshot { items: TaskItem[]; collections: CollectionSummary[]; groups: CollectionGroupSummary[]; }

export type CollectionColorName = CollectionColor;

export interface Settings {
  usesAutoDraft: boolean;
  alwaysOnTop: boolean;
  defaultPromptTemplate: string;
  lastSelectedCollection: string | null;
}

export interface InstallStatus {
  linkPath: string;
  targetPath: string;
  installed: boolean;
  conflictDescription?: string;
  installDirectoryIsInPath: boolean;
  canUninstall: boolean;
  canInstall: boolean;
  pathHint: string;
}
