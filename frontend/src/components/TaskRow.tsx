import { ContextMenu } from "@base-ui-components/react/context-menu";
import { Circle } from "lucide-react";
import type { CollectionColor, CollectionSummary, Snapshot, TaskItem, TaskStatus } from "../api/types";
import { setStatus, moveItem, deleteItem } from "../api/client";
import { leadingStatusClickTarget, rightClickStatusTarget } from "../state/status";
import type { FocusDir } from "../state/editor";
import { InlineEditor } from "./InlineEditor";
import { copyText } from "../lib/clipboard";

const STATUS_COLOR: Record<TaskStatus, CollectionColor> = {
  draft: "gray", ready: "gray", "in-progress": "blue", completed: "green",
  "on-hold": "orange", rejected: "red", aborted: "red",
};

const ALL_STATUSES: TaskStatus[] = [
  "draft", "ready", "in-progress", "completed", "on-hold", "rejected", "aborted",
];

export interface TaskRowProps {
  item: TaskItem;
  previous?: TaskItem;
  showCollection: boolean;
  collections: CollectionSummary[];
  focused: boolean;
  editingField: "title" | "note" | null;
  usesAutoDraft: boolean;
  onFocus: () => void;
  onEditTitle: () => void;
  onEditNote: () => void;
  onEndEdit: () => void;
  onMoveFocus: (dir: FocusDir) => void;
  onSnapshot: (snap: Snapshot) => void;
  onError: (msg: string) => void;
}

export function TaskRow({
  item, previous, showCollection, collections,
  focused, editingField, usesAutoDraft, onFocus, onEditTitle, onEditNote, onEndEdit, onMoveFocus, onSnapshot, onError,
}: TaskRowProps) {
  const advance = (e: React.MouseEvent) => {
    e.stopPropagation();
    setStatus(leadingStatusClickTarget(item.status), item.id, item).then(onSnapshot).catch((e) => onError(String(e)));
  };
  const toDraft = (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setStatus(rightClickStatusTarget(item.status), item.id, item).then(onSnapshot).catch((e) => onError(String(e)));
  };

  return (
    <ContextMenu.Root>
      <ContextMenu.Trigger>
        <div
          onClick={onFocus}
          style={{ background: focused ? "#e0e0e0" : undefined, borderRadius: 4 }}
        >
          <span
            style={{ color: STATUS_COLOR[item.status], cursor: "pointer" }}
            onClick={advance}
            onContextMenu={toDraft}
            title={item.status}
          >
            <Circle size={12} fill="currentColor" />
          </span>
          <div>
            <InlineEditor
              item={item}
              field="title"
              previous={previous}
              editing={editingField === "title"}
              usesAutoDraft={usesAutoDraft}
              onBeginEdit={onEditTitle}
              onEndEdit={onEndEdit}
              onMoveFocus={onMoveFocus}
              onSnapshot={onSnapshot}
              onError={onError}
            />
            <InlineEditor
              item={item}
              field="note"
              editing={editingField === "note"}
              usesAutoDraft={usesAutoDraft}
              onBeginEdit={onEditNote}
              onEndEdit={onEndEdit}
              onMoveFocus={onMoveFocus}
              onSnapshot={onSnapshot}
              onError={onError}
            />
          </div>
          {showCollection ? (
            <span>{item.collection}</span>
          ) : null}
        </div>
      </ContextMenu.Trigger>

      <ContextMenu.Portal>
        <ContextMenu.Positioner>
          <ContextMenu.Popup>
            <ContextMenu.SubmenuRoot>
              <ContextMenu.SubmenuTrigger>Status</ContextMenu.SubmenuTrigger>
              <ContextMenu.Portal>
                <ContextMenu.Positioner>
                  <ContextMenu.Popup>
                    {ALL_STATUSES.map((s) => (
                      <ContextMenu.Item
                        key={s}
                        onClick={() =>
                          setStatus(s, item.id, item).then(onSnapshot).catch((e) => onError(String(e)))
                        }
                      >
                        {s}
                      </ContextMenu.Item>
                    ))}
                  </ContextMenu.Popup>
                </ContextMenu.Positioner>
              </ContextMenu.Portal>
            </ContextMenu.SubmenuRoot>

            <ContextMenu.SubmenuRoot>
              <ContextMenu.SubmenuTrigger>Move to Collection</ContextMenu.SubmenuTrigger>
              <ContextMenu.Portal>
                <ContextMenu.Positioner>
                  <ContextMenu.Popup>
                    {collections.map((c) => (
                      <ContextMenu.Item
                        key={c.name}
                        disabled={c.name === item.collection}
                        onClick={() => moveItem(item.id, c.name).then(onSnapshot).catch((e) => onError(String(e)))}
                      >
                        {c.displayName}
                      </ContextMenu.Item>
                    ))}
                  </ContextMenu.Popup>
                </ContextMenu.Positioner>
              </ContextMenu.Portal>
            </ContextMenu.SubmenuRoot>

            <ContextMenu.Item
              onClick={() => copyText(item.id).catch((e) => onError(String(e)))}
            >
              Copy ID
            </ContextMenu.Item>

            <ContextMenu.Separator />
            <ContextMenu.Item
              onClick={() => deleteItem(item.id).then(onSnapshot).catch((e) => onError(String(e)))}
            >
              Delete
            </ContextMenu.Item>
          </ContextMenu.Popup>
        </ContextMenu.Positioner>
      </ContextMenu.Portal>
    </ContextMenu.Root>
  );
}
