import { describe, expect, it } from "vitest";
import type { Snapshot } from "../api/types";
import { ALL_COLLECTION, visibleItems, allIncompleteCount, sidebarGroups } from "./view";

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
    expect(visibleItems(snap(), { selected: ALL_COLLECTION, search: "", incompleteOnly: false, hideCompleted: false, showArchived: false }).length).toBe(3);
    expect(visibleItems(snap(), { selected: "Inbox", search: "", incompleteOnly: false, hideCompleted: false, showArchived: false }).map(i => i.id))
      .toEqual(["00000001", "00000003"]);
  });

  it("search matches title/collection/id and is case-insensitive", () => {
    expect(visibleItems(snap(), { selected: ALL_COLLECTION, search: "BETA", incompleteOnly: false, hideCompleted: false, showArchived: false }).map(i => i.id))
      .toEqual(["00000002"]);
    expect(visibleItems(snap(), { selected: ALL_COLLECTION, search: "work", incompleteOnly: false, hideCompleted: false, showArchived: false }).map(i => i.id))
      .toEqual(["00000002"]);
  });

  it("incompleteOnly hides completed", () => {
    expect(visibleItems(snap(), { selected: ALL_COLLECTION, search: "", incompleteOnly: true, hideCompleted: false, showArchived: false }).map(i => i.id))
      .toEqual(["00000001", "00000003"]);
  });

  it("allIncompleteCount counts non-completed", () => {
    expect(allIncompleteCount(snap())).toBe(2);
  });
});

describe("hideCompleted + showArchived", () => {
  it("hideCompleted removes completed items from visibleItems", () => {
    const s = snap();
    const all = visibleItems(s, { selected: ALL_COLLECTION, search: "", incompleteOnly: false, hideCompleted: false, showArchived: false });
    expect(all.length).toBe(3);
    const hidden = visibleItems(s, { selected: ALL_COLLECTION, search: "", incompleteOnly: false, hideCompleted: true, showArchived: false });
    expect(hidden.map((i) => i.id)).toEqual(["00000001", "00000003"]);
  });

  it("sidebarGroups hides archived collections unless showArchived", () => {
    const s: Snapshot = {
      items: [],
      collections: [],
      groups: [
        { name: "Work", collections: [
          { name: "Work/A", displayName: "A", groupName: "Work", totalCount: 0, incompleteCount: 0, color: "gray", isArchived: false },
          { name: "Work/B", displayName: "B", groupName: "Work", totalCount: 0, incompleteCount: 0, color: "gray", isArchived: true },
        ]},
      ],
    };
    expect(sidebarGroups(s, false)[0].collections.map((c) => c.name)).toEqual(["Work/A"]);
    expect(sidebarGroups(s, true)[0].collections.map((c) => c.name)).toEqual(["Work/A", "Work/B"]);
  });
});
