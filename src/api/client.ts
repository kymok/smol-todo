import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { Snapshot } from "./types";

export function getSnapshot(): Promise<Snapshot> {
  return invoke<Snapshot>("get_snapshot");
}

export function onStoreChanged(callback: () => void): Promise<UnlistenFn> {
  return listen("store-changed", () => callback());
}
