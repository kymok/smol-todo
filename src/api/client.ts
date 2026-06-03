import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { CollectionColor, Settings, Snapshot, TaskItem, TaskStatus } from "./types";

export function getSnapshot(): Promise<Snapshot> {
  return invoke<Snapshot>("get_snapshot");
}

export function onStoreChanged(callback: () => void): Promise<UnlistenFn> {
  return listen("store-changed", () => callback());
}

// --- Items ---
export function createItem(collection?: string): Promise<Snapshot> {
  return invoke<Snapshot>("create_item", { collection: collection ?? null });
}

export function updateItem(
  id: string,
  fields: { title?: string; collection?: string; status?: TaskStatus },
  ifCurrent?: TaskItem,
): Promise<Snapshot> {
  return invoke<Snapshot>("update_item", {
    id,
    title: fields.title ?? null,
    collection: fields.collection ?? null,
    status: fields.status ?? null,
    ifCurrent: ifCurrent ?? null,
  });
}

export function setStatus(status: TaskStatus, id: string, ifCurrent?: TaskItem): Promise<Snapshot> {
  return invoke<Snapshot>("set_status", { status, id, ifCurrent: ifCurrent ?? null });
}

export function moveItem(id: string, collection: string): Promise<Snapshot> {
  return invoke<Snapshot>("move_item", { id, collection });
}

export function deleteItem(id: string): Promise<Snapshot> {
  return invoke<Snapshot>("delete_item", { id });
}

export function deleteItems(ids: string[]): Promise<Snapshot> {
  return invoke<Snapshot>("delete_items", { ids });
}

// --- Notes ---
export function addNote(id: string, body: string, ifCurrent?: TaskItem): Promise<Snapshot> {
  return invoke<Snapshot>("add_note", { id, body, ifCurrent: ifCurrent ?? null });
}

export function updateNote(id: string, body: string): Promise<Snapshot> {
  return invoke<Snapshot>("update_note", { id, body });
}

export function deleteNote(id: string, ifCurrent?: TaskItem): Promise<Snapshot> {
  return invoke<Snapshot>("delete_note", { id, ifCurrent: ifCurrent ?? null });
}

// --- Merge / split ---
export function mergeItem(id: string, intoPrevious: string, title: string): Promise<Snapshot> {
  return invoke<Snapshot>("merge_item", { id, intoPrevious, title });
}

export function splitItem(
  id: string,
  firstTitle: string,
  secondTitle: string,
  secondId?: string,
): Promise<Snapshot> {
  return invoke<Snapshot>("split_item", { id, firstTitle, secondTitle, secondId: secondId ?? null });
}

// --- Collections ---
export function createCollection(name: string, group?: string): Promise<Snapshot> {
  return invoke<Snapshot>("create_collection", { name, group: group ?? null });
}

export function renameCollection(oldName: string, newName: string): Promise<Snapshot> {
  return invoke<Snapshot>("rename_collection", { old: oldName, new: newName });
}

export function setCollectionColor(name: string, color: CollectionColor): Promise<Snapshot> {
  return invoke<Snapshot>("set_collection_color", { name, color });
}

export function setCollectionArchived(name: string, isArchived: boolean): Promise<Snapshot> {
  return invoke<Snapshot>("set_collection_archived", { name, isArchived });
}

export function moveCollection(name: string, group: string): Promise<Snapshot> {
  return invoke<Snapshot>("move_collection", { name, group });
}

export function clearItems(name: string, completedOnly: boolean): Promise<Snapshot> {
  return invoke<Snapshot>("clear_items", { name, completedOnly });
}

export function deleteCollection(name: string): Promise<Snapshot> {
  return invoke<Snapshot>("delete_collection", { name });
}

// --- Groups ---
export function createGroup(name: string): Promise<Snapshot> {
  return invoke<Snapshot>("create_group", { name });
}

export function renameGroup(oldName: string, newName: string): Promise<Snapshot> {
  return invoke<Snapshot>("rename_group", { old: oldName, new: newName });
}

export function deleteGroup(name: string): Promise<Snapshot> {
  return invoke<Snapshot>("delete_group", { name });
}

// --- Settings ---
export function getSettings(): Promise<Settings> {
  return invoke<Settings>("get_settings");
}

export function setSettings(settings: Settings): Promise<Settings> {
  return invoke<Settings>("set_settings", { settings });
}

export function storePath(): Promise<string> {
  return invoke<string>("store_path");
}
