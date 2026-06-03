import { describe, expect, it } from "vitest";
import type { Snapshot, TaskItem, TaskStatus } from "../api/types";
import { presentStatuses } from "./bulkStatus";

function item(collection: string, status: TaskStatus): TaskItem {
  return {
    id: `${collection}-${status}-${Math.random()}`,
    version: "1",
    title: "t",
    collection,
    status,
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-01T00:00:00Z",
  };
}

function snap(items: TaskItem[]): Snapshot {
  return { items, collections: [], groups: [] };
}

describe("presentStatuses", () => {
  it("returns the distinct statuses in a collection, deduped", () => {
    const s = snap([
      item("Work", "ready"),
      item("Work", "ready"),
      item("Work", "in-progress"),
      item("Home", "completed"), // different collection — ignored
    ]);
    expect(presentStatuses(s, "Work")).toEqual(["ready", "in-progress"]);
  });

  it("orders by the canonical TaskStatus order regardless of item order", () => {
    const s = snap([
      item("Work", "aborted"),
      item("Work", "draft"),
      item("Work", "completed"),
      item("Work", "ready"),
    ]);
    expect(presentStatuses(s, "Work")).toEqual(["draft", "ready", "completed", "aborted"]);
  });

  it("returns an empty array for a collection with no items", () => {
    expect(presentStatuses(snap([]), "Work")).toEqual([]);
    expect(presentStatuses(snap([item("Home", "ready")]), "Work")).toEqual([]);
  });
});
