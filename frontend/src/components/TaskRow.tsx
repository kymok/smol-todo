import { ContextMenu, Flex, Text } from "@radix-ui/themes";
import { DotFilledIcon } from "@radix-ui/react-icons";
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
        <Flex
          align="start"
          gap="2"
          py="1"
          onClick={onFocus}
          style={{ background: focused ? "var(--accent-3)" : undefined, borderRadius: 4 }}
        >
          <Text
            color={STATUS_COLOR[item.status]}
            onClick={advance}
            onContextMenu={toDraft}
            style={{ cursor: "pointer" }}
            title={item.status}
          >
            <DotFilledIcon />
          </Text>
          <Flex direction="column" flexGrow="1">
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
          </Flex>
          {showCollection ? (
            <Text size="1" color="gray">{item.collection}</Text>
          ) : null}
        </Flex>
      </ContextMenu.Trigger>

      <ContextMenu.Content>
        <ContextMenu.Sub>
          <ContextMenu.SubTrigger>Status</ContextMenu.SubTrigger>
          <ContextMenu.SubContent>
            {ALL_STATUSES.map((s) => (
              <ContextMenu.Item
                key={s}
                onSelect={() =>
                  setStatus(s, item.id, item).then(onSnapshot).catch((e) => onError(String(e)))
                }
              >
                {s}
              </ContextMenu.Item>
            ))}
          </ContextMenu.SubContent>
        </ContextMenu.Sub>

        <ContextMenu.Sub>
          <ContextMenu.SubTrigger>Move to Collection</ContextMenu.SubTrigger>
          <ContextMenu.SubContent>
            {collections.map((c) => (
              <ContextMenu.Item
                key={c.name}
                disabled={c.name === item.collection}
                onSelect={() => moveItem(item.id, c.name).then(onSnapshot).catch((e) => onError(String(e)))}
              >
                {c.displayName}
              </ContextMenu.Item>
            ))}
          </ContextMenu.SubContent>
        </ContextMenu.Sub>

        <ContextMenu.Item
          onSelect={() => copyText(item.id).catch((e) => onError(String(e)))}
        >
          Copy ID
        </ContextMenu.Item>

        <ContextMenu.Separator />
        <ContextMenu.Item
          color="red"
          onSelect={() => deleteItem(item.id).then(onSnapshot).catch((e) => onError(String(e)))}
        >
          Delete
        </ContextMenu.Item>
      </ContextMenu.Content>
    </ContextMenu.Root>
  );
}
