import { writeText } from "@tauri-apps/plugin-clipboard-manager";

/** Write `text` to the system clipboard via the Tauri clipboard-manager plugin. */
export function copyText(text: string): Promise<void> {
  return writeText(text);
}
