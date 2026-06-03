import type { CollectionGroupSummary, Snapshot, TaskItem } from "../api/types";

export const ALL_COLLECTION = "__all__";

export interface ViewState {
  selected: string; // ALL_COLLECTION or a collection name
  search: string;
  incompleteOnly: boolean;
  hideCompleted: boolean;
  showArchived: boolean;
}

function matchesSearch(item: TaskItem, query: string): boolean {
  if (!query) return true;
  const q = query.toLowerCase();
  return (
    item.title.toLowerCase().includes(q) ||
    item.collection.toLowerCase().includes(q) ||
    item.id.toLowerCase().includes(q) ||
    (item.note?.body.toLowerCase().includes(q) ?? false)
  );
}

export function visibleItems(snapshot: Snapshot, view: ViewState): TaskItem[] {
  return snapshot.items.filter((item) => {
    const collectionMatches = view.selected === ALL_COLLECTION || item.collection === view.selected;
    const completedHidden = (view.incompleteOnly || view.hideCompleted) && item.status === "completed";
    return collectionMatches && !completedHidden && matchesSearch(item, view.search);
  });
}

export function allIncompleteCount(snapshot: Snapshot): number {
  return snapshot.items.filter((i) => i.status !== "completed").length;
}

/** Sidebar groups, optionally hiding archived collections (default: hide). */
export function sidebarGroups(snapshot: Snapshot, showArchived: boolean): CollectionGroupSummary[] {
  return snapshot.groups
    .map((g) => ({ ...g, collections: g.collections.filter((c) => showArchived || !c.isArchived) }))
    .filter((g) => g.name !== "DefaultGroup" || g.collections.length > 0);
}
