import { useState } from "react";
import { ScrollArea } from "@base-ui-components/react/scroll-area";
import type { Snapshot } from "../api/types";
import type { ConfirmRequest } from "../state/confirm";
import { ALL_COLLECTION, visibleItems, type ViewState } from "../state/view";
import type { FocusDir } from "../state/editor";
import { TaskRow } from "./TaskRow";
import { DetailContainer } from "./PaneContainers";
import { TITLE_BAR_HEIGHT } from "../layout";

export interface DetailPaneProps {
  snapshot: Snapshot;
  view: ViewState;
  focusedId: string | null;
  editingTarget: { id: string; field: "title" | "note" } | null;
  usesAutoDraft: boolean;
  onSearch: (q: string) => void;
  onFocusItem: (id: string | null) => void;
  onEdit: (id: string, field: "title" | "note") => void;
  onEndEdit: () => void;
  onSnapshot: (snap: Snapshot) => void;
  onError: (msg: string) => void;
  onRequestConfirm: (req: ConfirmRequest) => void;
}

export function DetailPane({
  snapshot, view, focusedId, editingTarget, usesAutoDraft,
  onFocusItem, onEdit, onEndEdit, onSnapshot, onError,
}: DetailPaneProps) {
  const [scrolled, setScrolled] = useState(false);
  const items = visibleItems(snapshot, view);
  const title = view.selected === ALL_COLLECTION
    ? "All"
    : snapshot.collections.find((c) => c.name === view.selected)?.displayName ?? view.selected;

  const moveFocus = (dir: FocusDir) => {
    if (items.length === 0) return;
    const idx = items.findIndex((i) => i.id === focusedId);
    const nextIdx = dir === "down"
      ? Math.min(items.length - 1, (idx < 0 ? -1 : idx) + 1)
      : Math.max(0, (idx < 0 ? items.length : idx) - 1);
    const next = items[nextIdx];
    if (next) {
      onFocusItem(next.id);
      onEdit(next.id, "title"); // entering a row opens its title editor
    }
  };

  return (
    <DetailContainer>
      {/* Title bar: collection title vertically centered in the title-bar-height
          strip. A bottom border fades in (animated) only once the list is scrolled. */}
      <div
        className={`flex shrink-0 items-center border-b transition-colors ${scrolled ? "border-neutral-50" : "border-transparent"}`}
        style={{ height: TITLE_BAR_HEIGHT }}
      >
        <h2>{title}</h2>
      </div>
      <ScrollArea.Root className="min-h-0 flex-1">
        <ScrollArea.Viewport
          className="h-full"
          onScroll={(e) => setScrolled(e.currentTarget.scrollTop > 0)}
        >
          {items.map((item, i) => (
            <TaskRow
              key={item.id}
              item={item}
              previous={i > 0 ? items[i - 1] : undefined}
              showCollection={view.selected === ALL_COLLECTION}
              collections={snapshot.collections}
              focused={focusedId === item.id}
              editingField={editingTarget?.id === item.id ? editingTarget.field : null}
              usesAutoDraft={usesAutoDraft}
              onFocus={() => onFocusItem(item.id)}
              onEditTitle={() => onEdit(item.id, "title")}
              onEditNote={() => onEdit(item.id, "note")}
              onEndEdit={onEndEdit}
              onMoveFocus={moveFocus}
              onSnapshot={onSnapshot}
              onError={onError}
            />
          ))}
        </ScrollArea.Viewport>
        <ScrollArea.Scrollbar>
          <ScrollArea.Thumb />
        </ScrollArea.Scrollbar>
      </ScrollArea.Root>
    </DetailContainer>
  );
}
