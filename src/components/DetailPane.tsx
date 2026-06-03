import { Button, Flex, Heading, ScrollArea, TextField } from "@radix-ui/themes";
import { MagnifyingGlassIcon, PlusIcon } from "@radix-ui/react-icons";
import type { Snapshot } from "../api/types";
import type { ConfirmRequest } from "../state/confirm";
import { ALL_COLLECTION, visibleItems, type ViewState } from "../state/view";
import { createItem } from "../api/client";
import type { FocusDir } from "../state/editor";
import { TaskRow } from "./TaskRow";

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
  onRequestConfirm: (req: ConfirmRequest) => void;
}

export function DetailPane({
  snapshot, view, focusedId, editingTarget, usesAutoDraft,
  onSearch, onFocusItem, onEdit, onEndEdit, onSnapshot,
}: DetailPaneProps) {
  const items = visibleItems(snapshot, view);
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
      .catch(console.error);
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
    <Flex direction="column" flexGrow="1" p="3" gap="3">
      <Flex align="center" justify="between">
        <Heading size="4">{title}</Heading>
        <Flex align="center" gap="2">
          <TextField.Root placeholder="Search" value={view.search} onChange={(e) => onSearch(e.target.value)}>
            <TextField.Slot><MagnifyingGlassIcon /></TextField.Slot>
          </TextField.Root>
          <Button onClick={newTask}><PlusIcon /> New Task</Button>
        </Flex>
      </Flex>
      <ScrollArea>
        <Flex direction="column">
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
            />
          ))}
        </Flex>
      </ScrollArea>
    </Flex>
  );
}
