import { useState, useEffect } from "react";
import { Badge, Box, Button, ContextMenu, Dialog, DropdownMenu, Flex, Text, TextField } from "@radix-ui/themes";
import { DotFilledIcon, GearIcon } from "@radix-ui/react-icons";
import type { CollectionColor, CollectionSummary, Snapshot } from "../api/types";
import {
  ALL_COLLECTION, allIncompleteCount, sidebarGroups,
} from "../state/view";
import type { ConfirmRequest } from "../state/confirm";
import {
  clearItems, createCollection, createGroup, deleteCollection, deleteGroup,
  moveCollection, renameCollection, renameGroup, setCollectionArchived, setCollectionColor,
} from "../api/client";

const COLORS: CollectionColor[] = ["gray", "red", "orange", "yellow", "green", "blue", "purple"];

interface PromptState {
  title: string;
  label: string;
  initial: string;
  submit: (value: string) => void;
}

export interface SidebarProps {
  snapshot: Snapshot;
  selected: string;
  showArchived: boolean;
  hideCompleted: boolean;
  usesAutoDraft: boolean;
  alwaysOnTop: boolean;
  onSelect: (name: string) => void;
  onToggleHideCompleted: () => void;
  onToggleShowArchived: () => void;
  onToggleAutoDraft: () => void;
  onToggleAlwaysOnTop: () => void;
  onOpenSettings: () => void;
  onSnapshot: (snap: Snapshot) => void;
  onRequestConfirm: (req: ConfirmRequest) => void;
}

export function Sidebar({
  snapshot, selected, showArchived, hideCompleted, usesAutoDraft, alwaysOnTop,
  onSelect, onToggleHideCompleted, onToggleShowArchived, onToggleAutoDraft, onToggleAlwaysOnTop,
  onOpenSettings, onSnapshot, onRequestConfirm,
}: SidebarProps) {
  const groupNames = snapshot.groups.map((g) => g.name);

  const [prompt, setPrompt] = useState<PromptState | null>(null);
  const [promptValue, setPromptValue] = useState("");

  useEffect(() => {
    if (prompt !== null) {
      setPromptValue(prompt.initial);
    }
  }, [prompt]);

  const renameCol = (c: CollectionSummary) => {
    setPrompt({
      title: "Rename Collection",
      label: "New name",
      initial: c.displayName,
      submit: (v) => { renameCollection(c.name, v).then(onSnapshot).catch(console.error); },
    });
  };

  const renameGrp = (group: { name: string }) => {
    setPrompt({
      title: "Rename Group",
      label: "New name",
      initial: group.name,
      submit: (v) => renameGroup(group.name, v).then(onSnapshot).catch(console.error),
    });
  };

  const addCollectionTo = (group: { name: string }) => {
    setPrompt({
      title: "New Collection",
      label: "Collection name",
      initial: "",
      submit: (v) => createCollection(v, group.name).then(onSnapshot).catch(console.error),
    });
  };

  const handleSubmit = () => {
    const v = promptValue.trim();
    if (v && prompt) prompt.submit(v);
    setPrompt(null);
  };

  return (
    <>
      <Dialog.Root open={prompt !== null} onOpenChange={(o) => { if (!o) setPrompt(null); }}>
        <Dialog.Content>
          <Dialog.Title>{prompt?.title}</Dialog.Title>
          <TextField.Root
            value={promptValue}
            onChange={(e) => setPromptValue(e.target.value)}
            placeholder={prompt?.label}
            autoFocus
            onKeyDown={(e) => { if (e.key === "Enter") handleSubmit(); }}
          />
          <Flex gap="2" mt="3" justify="end">
            <Dialog.Close>
              <Button variant="soft" color="gray">Cancel</Button>
            </Dialog.Close>
            <Button onClick={handleSubmit}>Save</Button>
          </Flex>
        </Dialog.Content>
      </Dialog.Root>

      <Flex direction="column" gap="1" p="2" style={{ width: 240 }}>
        <Button variant={selected === ALL_COLLECTION ? "soft" : "ghost"} onClick={() => onSelect(ALL_COLLECTION)}>
          <Flex align="center" gap="2" flexGrow="1">
            <Box flexGrow="1"><Text align="left">All</Text></Box>
            <Badge>{allIncompleteCount(snapshot)}</Badge>
          </Flex>
        </Button>

        {sidebarGroups(snapshot, showArchived).map((group) => (
          <Box key={group.name} mt="2">
            <ContextMenu.Root>
              <ContextMenu.Trigger>
                <Text size="1" color="gray">{group.name === "DefaultGroup" ? "No Group" : group.name}</Text>
              </ContextMenu.Trigger>
              <ContextMenu.Content>
                <ContextMenu.Item disabled={group.name === "DefaultGroup"} onSelect={() => renameGrp(group)}>
                  Rename Group
                </ContextMenu.Item>
                <ContextMenu.Item onSelect={() => addCollectionTo(group)}>Add Collection</ContextMenu.Item>
                <ContextMenu.Separator />
                <ContextMenu.Item
                  color="red"
                  disabled={group.name === "DefaultGroup"}
                  onSelect={() =>
                    onRequestConfirm({
                      title: `Delete group "${group.name}"?`,
                      description: "Its collections move to No Group. This cannot be undone.",
                      confirmLabel: "Delete",
                      onConfirm: () => deleteGroup(group.name).then(onSnapshot).catch(console.error),
                    })
                  }
                >
                  Delete Group
                </ContextMenu.Item>
              </ContextMenu.Content>
            </ContextMenu.Root>

            {group.collections.map((c) => (
              <ContextMenu.Root key={c.name}>
                <ContextMenu.Trigger>
                  <Button
                    variant={selected === c.name ? "soft" : "ghost"}
                    onClick={() => onSelect(c.name)}
                    style={c.isArchived ? { opacity: 0.5 } : undefined}
                  >
                    <Flex align="center" gap="2" flexGrow="1">
                      <Text color={c.color}><DotFilledIcon /></Text>
                      <Box flexGrow="1"><Text align="left">{c.displayName}</Text></Box>
                      <Badge>{c.incompleteCount}</Badge>
                    </Flex>
                  </Button>
                </ContextMenu.Trigger>
                <ContextMenu.Content>
                  <ContextMenu.Item onSelect={() => renameCol(c)}>Rename</ContextMenu.Item>

                  <ContextMenu.Sub>
                    <ContextMenu.SubTrigger>Color</ContextMenu.SubTrigger>
                    <ContextMenu.SubContent>
                      {COLORS.map((color) => (
                        <ContextMenu.Item
                          key={color}
                          onSelect={() => setCollectionColor(c.name, color).then(onSnapshot).catch(console.error)}
                        >
                          <Text color={color}><DotFilledIcon /></Text> {color}
                        </ContextMenu.Item>
                      ))}
                    </ContextMenu.SubContent>
                  </ContextMenu.Sub>

                  <ContextMenu.Item
                    onSelect={() =>
                      setCollectionArchived(c.name, !c.isArchived).then(onSnapshot).catch(console.error)
                    }
                  >
                    {c.isArchived ? "Unarchive" : "Archive"}
                  </ContextMenu.Item>

                  <ContextMenu.Sub>
                    <ContextMenu.SubTrigger>Move to Group</ContextMenu.SubTrigger>
                    <ContextMenu.SubContent>
                      {groupNames.map((g) => (
                        <ContextMenu.Item
                          key={g}
                          disabled={g === c.groupName}
                          onSelect={() => moveCollection(c.name, g).then(onSnapshot).catch(console.error)}
                        >
                          {g === "DefaultGroup" ? "No Group" : g}
                        </ContextMenu.Item>
                      ))}
                    </ContextMenu.SubContent>
                  </ContextMenu.Sub>

                  <ContextMenu.Sub>
                    <ContextMenu.SubTrigger>Clear</ContextMenu.SubTrigger>
                    <ContextMenu.SubContent>
                      <ContextMenu.Item onSelect={() => clearItems(c.name, false).then(onSnapshot).catch(console.error)}>
                        All Items
                      </ContextMenu.Item>
                      <ContextMenu.Item onSelect={() => clearItems(c.name, true).then(onSnapshot).catch(console.error)}>
                        Completed Items
                      </ContextMenu.Item>
                    </ContextMenu.SubContent>
                  </ContextMenu.Sub>

                  <ContextMenu.Separator />
                  <ContextMenu.Item
                    color="red"
                    onSelect={() =>
                      onRequestConfirm({
                        title: `Delete collection "${c.displayName}"?`,
                        description: "All its tasks are permanently deleted. This cannot be undone.",
                        confirmLabel: "Delete",
                        onConfirm: () => deleteCollection(c.name).then(onSnapshot).catch(console.error),
                      })
                    }
                  >
                    Delete
                  </ContextMenu.Item>
                </ContextMenu.Content>
              </ContextMenu.Root>
            ))}
          </Box>
        ))}

        <Box flexGrow="1" />

        <DropdownMenu.Root>
          <DropdownMenu.Trigger>
            <Button variant="ghost"><GearIcon /> View</Button>
          </DropdownMenu.Trigger>
          <DropdownMenu.Content>
            <DropdownMenu.CheckboxItem checked={hideCompleted} onCheckedChange={onToggleHideCompleted}>
              Hide Completed
            </DropdownMenu.CheckboxItem>
            <DropdownMenu.CheckboxItem checked={showArchived} onCheckedChange={onToggleShowArchived}>
              Show Archived
            </DropdownMenu.CheckboxItem>
            <DropdownMenu.CheckboxItem checked={usesAutoDraft} onCheckedChange={onToggleAutoDraft}>
              Automatic Drafts
            </DropdownMenu.CheckboxItem>
            <DropdownMenu.CheckboxItem checked={alwaysOnTop} onCheckedChange={onToggleAlwaysOnTop}>
              Always On Top
            </DropdownMenu.CheckboxItem>
            <DropdownMenu.Separator />
            <DropdownMenu.Item
              onSelect={() =>
                setPrompt({
                  title: "New Group",
                  label: "Group name",
                  initial: "",
                  submit: (v) => createGroup(v).then(onSnapshot).catch(console.error),
                })
              }
            >
              Add a Group
            </DropdownMenu.Item>
            <DropdownMenu.Separator />
            <DropdownMenu.Item onSelect={onOpenSettings}>Settings…</DropdownMenu.Item>
          </DropdownMenu.Content>
        </DropdownMenu.Root>
      </Flex>
    </>
  );
}
