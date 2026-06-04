import { useState } from "react";
import { ScrollArea } from "@base-ui-components/react/scroll-area";
import { Plus } from "lucide-react";
import type { Snapshot } from "../../api/types";
import type { ConfirmRequest } from "../../state/confirm";
import { ALL_COLLECTION, visibleItems, type ViewState } from "../../state/view";
import { createItem } from "../../api/client";
import type { FocusDir } from "../../state/editor";
import { TaskRow } from "../TaskRow";
import styles from "./detailPane.module.css";

export interface DetailPaneProps {
  snapshot: Snapshot;
  view: ViewState;
  focusedId: string | null;
  editingTarget: { id: string; field: "title" | "note" } | null;
  usesAutoDraft: boolean;
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
  const items = visibleItems(snapshot, view);
  const [isHeaderStuck, setIsHeaderStuck] = useState(false);
  const title = view.selected === ALL_COLLECTION
    ? "All"
    : snapshot.collections.find((c) => c.name === view.selected)?.displayName ?? view.selected;

  const newTask = () => {
    const target = view.selected === ALL_COLLECTION ? undefined : view.selected;
    createItem(target)
      .then((snap) => {
        onSnapshot(snap);
        const created = [...snap.items].reverse()
          .find((i) => i.title === "" && i.status === "draft" && (!target || i.collection === target));
        if (created) {
          onFocusItem(created.id);
          onEdit(created.id, "title");
        }
      })
      .catch((e) => onError(String(e)));
  };

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
    <div className={styles.detailPane}>
      <ScrollArea.Root className={styles.scrollArea}>
        <ScrollArea.Viewport
          className={styles.viewport}
          onScroll={(event) => setIsHeaderStuck(event.currentTarget.scrollTop > 0)}
        >
          <div
            data-tauri-drag-region
            className={isHeaderStuck ? `${styles.header} ${styles.headerStuck}` : styles.header}
          >
            <h2 className={styles.title}>{title}</h2>
            <div className={styles.actions}>
              <button aria-label="New Task" title="New Task" onClick={newTask}><Plus /></button>
            </div>
          </div>
          <div className={styles.content}>
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
          </div>
        </ScrollArea.Viewport>
        <ScrollArea.Scrollbar>
          <ScrollArea.Thumb />
        </ScrollArea.Scrollbar>
      </ScrollArea.Root>
    </div>
  );
}
