import type { Snapshot, TaskStatus } from "../api/types";

// Canonical status order = the pond-core TaskStatus::all() order (and the types.ts union order).
const STATUS_ORDER: TaskStatus[] = [
  "draft",
  "ready",
  "in-progress",
  "completed",
  "on-hold",
  "rejected",
  "aborted",
];

/** The distinct statuses among `collection`'s items, in canonical status order. */
export function presentStatuses(snapshot: Snapshot, collection: string): TaskStatus[] {
  const present = new Set<TaskStatus>();
  for (const item of snapshot.items) {
    if (item.collection === collection) present.add(item.status);
  }
  return STATUS_ORDER.filter((s) => present.has(s));
}
