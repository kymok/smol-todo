import { ContextMenu } from "@base-ui-components/react/context-menu";
import { Menu } from "@base-ui-components/react/menu";
import { Circle, Ellipsis } from "lucide-react";
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

// ContextMenu and Menu expose the same parts, so the action items render once
// against either namespace (passed as `M`) — used by both the right-click menu
// and the explicit (⋯) trigger.
type MenuParts = {
  Item: React.ElementType;
  Separator: React.ElementType;
  SubmenuRoot: React.ElementType;
  SubmenuTrigger: React.ElementType;
  Portal: React.ElementType;
  Positioner: React.ElementType;
  Popup: React.ElementType;
};

function ActionMenuItems({ M, item, collections, onSnapshot, onError }: {
  M: MenuParts;
  item: TaskItem;
  collections: CollectionSummary[];
  onSnapshot: (snap: Snapshot) => void;
  onError: (msg: string) => void;
}) {
  return (
    <>
      <M.SubmenuRoot>
        <M.SubmenuTrigger>Status</M.SubmenuTrigger>
        <M.Portal>
          <M.Positioner>
            <M.Popup>
              {ALL_STATUSES.map((s) => (
                <M.Item key={s} onClick={() => setStatus(s, item.id, item).then(onSnapshot).catch((e) => onError(String(e)))}>
                  {s}
                </M.Item>
              ))}
            </M.Popup>
          </M.Positioner>
        </M.Portal>
      </M.SubmenuRoot>

      <M.SubmenuRoot>
        <M.SubmenuTrigger>Move to Collection</M.SubmenuTrigger>
        <M.Portal>
          <M.Positioner>
            <M.Popup>
              {collections.map((c) => (
                <M.Item key={c.name} disabled={c.name === item.collection} onClick={() => moveItem(item.id, c.name).then(onSnapshot).catch((e) => onError(String(e)))}>
                  {c.displayName}
                </M.Item>
              ))}
            </M.Popup>
          </M.Positioner>
        </M.Portal>
      </M.SubmenuRoot>

      <M.Item onClick={() => copyText(item.id).catch((e) => onError(String(e)))}>Copy ID</M.Item>
      <M.Separator />
      <M.Item onClick={() => deleteItem(item.id).then(onSnapshot).catch((e) => onError(String(e)))}>Delete</M.Item>
    </>
  );
}

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
  item, previous, collections,
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
          className="flex items-start gap-2 text-sm"
          style={{ background: focused ? "#e0e0e0" : undefined, borderRadius: 4 }}
        >
          {/* Leading controls center on the title's first line: a 1-line-tall box
              (h-[1lh] tracks the line-height) with the glyph centered. */}
          <span
            className="flex h-[1lh] shrink-0 items-center"
            style={{ color: STATUS_COLOR[item.status], cursor: "pointer" }}
            onClick={advance}
            onContextMenu={toDraft}
            title={item.status}
          >
            <Circle size={20} fill="currentColor" />
          </span>

          {/* Explicit action-menu trigger (same items as the right-click menu). */}
          <Menu.Root>
            <Menu.Trigger className="flex h-[1lh] shrink-0 items-center text-neutral-400" aria-label="Actions" onClick={(e) => e.stopPropagation()}>
              <Ellipsis size={16} />
            </Menu.Trigger>
            <Menu.Portal>
              <Menu.Positioner>
                <Menu.Popup>
                  <ActionMenuItems M={Menu} item={item} collections={collections} onSnapshot={onSnapshot} onError={onError} />
                </Menu.Popup>
              </Menu.Positioner>
            </Menu.Portal>
          </Menu.Root>

          <div className="min-w-0 flex-1">
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
        </div>
      </ContextMenu.Trigger>

      <ContextMenu.Portal>
        <ContextMenu.Positioner>
          <ContextMenu.Popup>
            <ActionMenuItems M={ContextMenu} item={item} collections={collections} onSnapshot={onSnapshot} onError={onError} />
          </ContextMenu.Popup>
        </ContextMenu.Positioner>
      </ContextMenu.Portal>
    </ContextMenu.Root>
  );
}
