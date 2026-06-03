import type { TaskStatus } from "../api/types";

/**
 * Left-click on the leading status dot advances the status.
 * Mirrors Swift `TaskStatus.leadingStatusClickTarget` (TaskViewSupport.swift):
 * ready -> completed, in-progress -> completed, everything else -> ready.
 */
export function leadingStatusClickTarget(status: TaskStatus): TaskStatus {
  switch (status) {
    case "ready":
    case "in-progress":
      return "completed";
    default:
      return "ready";
  }
}

/** Right-click on the leading status dot sets the task back to draft. */
export function rightClickStatusTarget(_status: TaskStatus): TaskStatus {
  return "draft";
}
