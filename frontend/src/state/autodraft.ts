import type { TaskStatus } from "../api/types";

export type AutoDraftPhase = "edit" | "confirm";

export interface AutoDraftInput {
  /** The `usesAutoDraft` setting. */
  usesAutoDraft: boolean;
  /** The task's current status (from the snapshot item being edited). */
  currentStatus: TaskStatus;
  /** "edit" = debounced autosave / blur; "confirm" = Cmd+Enter / Tab commit. */
  phase: AutoDraftPhase;
  /** Whether the title actually changed (trimmed) vs the stored value. */
  titleChanged: boolean;
}

/**
 * The status to apply when saving a TITLE edit (notes never auto-draft).
 * `undefined` means "leave the status unchanged" (send title only).
 *
 * Mirrors Swift `DetailView` (DetailView.swift):
 *   saveTitle:    statusAfterEdit = title == item.title ? nil : autoDraftEditStatus
 *                 autoDraftEditStatus = usesAutoDraft ? .draft : nil
 *   confirmTitle: confirmationStatus = item.status == .draft ? .ready : autoDraftConfirmationStatus
 *                 autoDraftConfirmationStatus = usesAutoDraft ? .ready : nil
 */
export function autoDraftStatus({
  usesAutoDraft,
  currentStatus,
  phase,
  titleChanged,
}: AutoDraftInput): TaskStatus | undefined {
  if (phase === "confirm") {
    if (currentStatus === "draft") return "ready"; // draft → ready on confirm, always
    return usesAutoDraft ? "ready" : undefined; // non-draft → ready when auto-draft on
  }
  // phase === "edit"
  if (!titleChanged) return undefined; // no change → no status change
  if (currentStatus === "draft") return undefined; // a draft stays draft while editing
  return usesAutoDraft ? "draft" : undefined; // non-draft → draft when auto-draft on
}
