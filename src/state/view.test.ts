import { describe, expect, it } from "vitest";
import type { Snapshot } from "../api/types";
import { ALL_COLLECTION, visibleItems, allIncompleteCount } from "./view";

function snap(): Snapshot {
  const base = { version: "v", createdAt: "t", updatedAt: "t" } as const;
  return {
    items: [
      { ...base, id: "00000001", title: "alpha", collection: "Inbox", status: "ready" },
      { ...base, id: "00000002", title: "beta", collection: "Work/A", status: "completed" },
      { ...base, id: "00000003", title: "gamma", collection: "Inbox", status: "in-progress" },
    ],
    collections: [], groups: [],
  };
}

describe("visibleItems", () => {
  it("ALL shows everything; a collection filters", () => {
    expect(visibleItems(snap(), { selected: ALL_COLLECTION, search: "", incompleteOnly: false }).length).toBe(3);
    expect(visibleItems(snap(), { selected: "Inbox", search: "", incompleteOnly: false }).map(i => i.id))
      .toEqual(["00000001", "00000003"]);
  });

  it("search matches title/collection/id and is case-insensitive", () => {
    expect(visibleItems(snap(), { selected: ALL_COLLECTION, search: "BETA", incompleteOnly: false }).map(i => i.id))
      .toEqual(["00000002"]);
    expect(visibleItems(snap(), { selected: ALL_COLLECTION, search: "work", incompleteOnly: false }).map(i => i.id))
      .toEqual(["00000002"]);
  });

  it("incompleteOnly hides completed", () => {
    expect(visibleItems(snap(), { selected: ALL_COLLECTION, search: "", incompleteOnly: true }).map(i => i.id))
      .toEqual(["00000001", "00000003"]);
  });

  it("allIncompleteCount counts non-completed", () => {
    expect(allIncompleteCount(snap())).toBe(2);
  });
});
