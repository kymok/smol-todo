import { useState, useEffect } from "react";
import { ContextMenu } from "@base-ui-components/react/context-menu";
import { Dialog } from "@base-ui-components/react/dialog";
import { Menu } from "@base-ui-components/react/menu";
import { Input } from "@base-ui-components/react/input";
import { Circle, Settings } from "lucide-react";
import type { CollectionColor, CollectionSummary, Snapshot } from "../api/types";
import {
  ALL_COLLECTION, allIncompleteCount, sidebarGroups,
} from "../state/view";
import type { ConfirmRequest } from "../state/confirm";
import {
  clearItems, collectionCliCommand, collectionPromptText, createCollection, createGroup,
  deleteCollection, deleteGroup, exportCollection, moveCollection, renameCollection,
  renameGroup, setCollectionArchived, setCollectionColor,
} from "../api/client";
import { copyText } from "../lib/clipboard";
import { save } from "@tauri-apps/plugin-dialog";
import { SidebarContainer } from "./PaneContainers";

const COLORS: CollectionColor[] = ["gray", "red", "orange", "yellow", "green", "blue", "purple"];

interface PromptState {
  title: string;
  label: string;
  initial: string;
  submit: (value: string) => void;
}

// Default resize bounds when the host does not supply them.
const DEFAULT_MIN_WIDTH = 120;
const DEFAULT_MAX_WIDTH = 600;
const DEFAULT_WIDTH = 240;

const clamp = (value: number, min: number, max: number) => Math.min(Math.max(value, min), max);

export interface SidebarProps {
  snapshot: Snapshot;
  selected: string;
  /** Smallest width the sidebar can be dragged to, in px. Defaults to 120. */
  minWidth?: number;
  /** Largest width the sidebar can be dragged to, in px. Defaults to 600. */
  maxWidth?: number;
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
  onEditPrompt: (name: string) => void;
  onChangeStatuses: (name: string) => void;
  onSnapshot: (snap: Snapshot) => void;
  onError: (msg: string) => void;
  onRequestConfirm: (req: ConfirmRequest) => void;
}

export function Sidebar({
  snapshot, selected, showArchived, hideCompleted, usesAutoDraft, alwaysOnTop,
  minWidth = DEFAULT_MIN_WIDTH, maxWidth = DEFAULT_MAX_WIDTH,
  onSelect, onToggleHideCompleted, onToggleShowArchived, onToggleAutoDraft, onToggleAlwaysOnTop,
  onOpenSettings, onEditPrompt, onChangeStatuses, onSnapshot, onError, onRequestConfirm,
}: SidebarProps) {
  const groupNames = snapshot.groups.map((g) => g.name);

  const [width, setWidth] = useState(() => clamp(DEFAULT_WIDTH, minWidth, maxWidth));
  const [dragging, setDragging] = useState(false);

  // Keep the current width within bounds if min/max change underneath us.
  useEffect(() => {
    setWidth((w) => clamp(w, minWidth, maxWidth));
  }, [minWidth, maxWidth]);

  // Drag the right-edge handle to resize. We capture the starting width/x on
  // pointerdown and track movement on the window so the drag continues even if
  // the pointer leaves the thin handle.
  const startResize = (e: React.PointerEvent) => {
    e.preventDefault();
    const startX = e.clientX;
    const startWidth = width;
    setDragging(true);
    const onMove = (ev: PointerEvent) => {
      setWidth(clamp(startWidth + (ev.clientX - startX), minWidth, maxWidth));
    };
    const onUp = () => {
      setDragging(false);
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
    };
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
  };

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
      submit: (v) => { renameCollection(c.name, v).then(onSnapshot).catch((e) => onError(String(e))); },
    });
  };

  const renameGrp = (group: { name: string }) => {
    setPrompt({
      title: "Rename Group",
      label: "New name",
      initial: group.name,
      submit: (v) => renameGroup(group.name, v).then(onSnapshot).catch((e) => onError(String(e))),
    });
  };

  const addCollectionTo = (group: { name: string }) => {
    setPrompt({
      title: "New Collection",
      label: "Collection name",
      initial: "",
      submit: (v) => createCollection(v, group.name).then(onSnapshot).catch((e) => onError(String(e))),
    });
  };

  const handleSubmit = () => {
    const v = promptValue.trim();
    if (v && prompt) prompt.submit(v);
    setPrompt(null);
  };

  const exportAs = (name: string, format: "json" | "jsonl") => {
    const ext = format; // "json" | "jsonl"
    save({
      defaultPath: `${name}.${ext}`,
      filters: [{ name: format.toUpperCase(), extensions: [ext] }],
    })
      .then((path) => {
        if (path) return exportCollection(name, format, path);
      })
      .catch((e) => onError(String(e)));
  };

  return (
    <>
      <Dialog.Root open={prompt !== null} onOpenChange={(o) => { if (!o) setPrompt(null); }}>
        <Dialog.Portal>
          <Dialog.Backdrop />
          <Dialog.Popup>
            <Dialog.Title>{prompt?.title}</Dialog.Title>
            <Input
              value={promptValue}
              onChange={(e) => setPromptValue(e.target.value)}
              placeholder={prompt?.label}
              autoFocus
              onKeyDown={(e) => { if (e.key === "Enter") handleSubmit(); }}
            />
            <div>
              <Dialog.Close>Cancel</Dialog.Close>
              <button onClick={handleSubmit}>Save</button>
            </div>
          </Dialog.Popup>
        </Dialog.Portal>
      </Dialog.Root>

      <SidebarContainer width={width}>
        <button aria-current={selected === ALL_COLLECTION} onClick={() => onSelect(ALL_COLLECTION)}>
          <span>All</span>
          <span>{allIncompleteCount(snapshot)}</span>
        </button>

        {sidebarGroups(snapshot, showArchived).map((group) => (
          <div key={group.name}>
            <ContextMenu.Root>
              <ContextMenu.Trigger>
                <span>{group.name === "DefaultGroup" ? "No Group" : group.name}</span>
              </ContextMenu.Trigger>
              <ContextMenu.Portal>
                <ContextMenu.Positioner>
                  <ContextMenu.Popup>
                    <ContextMenu.Item disabled={group.name === "DefaultGroup"} onClick={() => renameGrp(group)}>
                      Rename Group
                    </ContextMenu.Item>
                    <ContextMenu.Item onClick={() => addCollectionTo(group)}>Add Collection</ContextMenu.Item>
                    <ContextMenu.Separator />
                    <ContextMenu.Item
                      disabled={group.name === "DefaultGroup"}
                      onClick={() =>
                        onRequestConfirm({
                          title: `Delete group "${group.name}"?`,
                          description: "Its collections move to No Group. This cannot be undone.",
                          confirmLabel: "Delete",
                          onConfirm: () => deleteGroup(group.name).then(onSnapshot).catch((e) => onError(String(e))),
                        })
                      }
                    >
                      Delete Group
                    </ContextMenu.Item>
                  </ContextMenu.Popup>
                </ContextMenu.Positioner>
              </ContextMenu.Portal>
            </ContextMenu.Root>

            {group.collections.map((c) => (
              <ContextMenu.Root key={c.name}>
                <ContextMenu.Trigger>
                  <button
                    aria-current={selected === c.name}
                    onClick={() => onSelect(c.name)}
                    style={c.isArchived ? { opacity: 0.5 } : undefined}
                  >
                    <span style={{ color: c.color }}><Circle size={12} fill="currentColor" /></span>
                    <span>{c.displayName}</span>
                    <span>{c.incompleteCount}</span>
                  </button>
                </ContextMenu.Trigger>
                <ContextMenu.Portal>
                  <ContextMenu.Positioner>
                    <ContextMenu.Popup>
                      <ContextMenu.Item onClick={() => renameCol(c)}>Rename</ContextMenu.Item>
                      <ContextMenu.Item onClick={() => onEditPrompt(c.name)}>Edit Prompt…</ContextMenu.Item>
                      <ContextMenu.Item
                        onClick={() =>
                          collectionPromptText(c.name).then(copyText).catch((e) => onError(String(e)))
                        }
                      >
                        Copy Prompt
                      </ContextMenu.Item>
                      <ContextMenu.Item
                        onClick={() =>
                          collectionCliCommand(c.name).then(copyText).catch((e) => onError(String(e)))
                        }
                      >
                        Copy CLI Command
                      </ContextMenu.Item>
                      <ContextMenu.Separator />

                      <ContextMenu.SubmenuRoot>
                        <ContextMenu.SubmenuTrigger>Color</ContextMenu.SubmenuTrigger>
                        <ContextMenu.Portal>
                          <ContextMenu.Positioner>
                            <ContextMenu.Popup>
                              {COLORS.map((color) => (
                                <ContextMenu.Item
                                  key={color}
                                  onClick={() => setCollectionColor(c.name, color).then(onSnapshot).catch((e) => onError(String(e)))}
                                >
                                  <span style={{ color }}><Circle size={12} fill="currentColor" /></span> {color}
                                </ContextMenu.Item>
                              ))}
                            </ContextMenu.Popup>
                          </ContextMenu.Positioner>
                        </ContextMenu.Portal>
                      </ContextMenu.SubmenuRoot>

                      <ContextMenu.Item
                        onClick={() =>
                          setCollectionArchived(c.name, !c.isArchived).then(onSnapshot).catch((e) => onError(String(e)))
                        }
                      >
                        {c.isArchived ? "Unarchive" : "Archive"}
                      </ContextMenu.Item>

                      <ContextMenu.SubmenuRoot>
                        <ContextMenu.SubmenuTrigger>Move to Group</ContextMenu.SubmenuTrigger>
                        <ContextMenu.Portal>
                          <ContextMenu.Positioner>
                            <ContextMenu.Popup>
                              {groupNames.map((g) => (
                                <ContextMenu.Item
                                  key={g}
                                  disabled={g === c.groupName}
                                  onClick={() => moveCollection(c.name, g).then(onSnapshot).catch((e) => onError(String(e)))}
                                >
                                  {g === "DefaultGroup" ? "No Group" : g}
                                </ContextMenu.Item>
                              ))}
                            </ContextMenu.Popup>
                          </ContextMenu.Positioner>
                        </ContextMenu.Portal>
                      </ContextMenu.SubmenuRoot>

                      <ContextMenu.SubmenuRoot>
                        <ContextMenu.SubmenuTrigger>Clear</ContextMenu.SubmenuTrigger>
                        <ContextMenu.Portal>
                          <ContextMenu.Positioner>
                            <ContextMenu.Popup>
                              <ContextMenu.Item onClick={() => clearItems(c.name, false).then(onSnapshot).catch((e) => onError(String(e)))}>
                                All Items
                              </ContextMenu.Item>
                              <ContextMenu.Item onClick={() => clearItems(c.name, true).then(onSnapshot).catch((e) => onError(String(e)))}>
                                Completed Items
                              </ContextMenu.Item>
                            </ContextMenu.Popup>
                          </ContextMenu.Positioner>
                        </ContextMenu.Portal>
                      </ContextMenu.SubmenuRoot>

                      <ContextMenu.SubmenuRoot>
                        <ContextMenu.SubmenuTrigger>Export Collection</ContextMenu.SubmenuTrigger>
                        <ContextMenu.Portal>
                          <ContextMenu.Positioner>
                            <ContextMenu.Popup>
                              <ContextMenu.Item onClick={() => exportAs(c.name, "json")}>
                                As JSON
                              </ContextMenu.Item>
                              <ContextMenu.Item onClick={() => exportAs(c.name, "jsonl")}>
                                As JSONL
                              </ContextMenu.Item>
                            </ContextMenu.Popup>
                          </ContextMenu.Positioner>
                        </ContextMenu.Portal>
                      </ContextMenu.SubmenuRoot>

                      <ContextMenu.Item onClick={() => onChangeStatuses(c.name)}>
                        Change Statuses…
                      </ContextMenu.Item>

                      <ContextMenu.Separator />
                      <ContextMenu.Item
                        onClick={() =>
                          onRequestConfirm({
                            title: `Delete collection "${c.displayName}"?`,
                            description: "All its tasks are permanently deleted. This cannot be undone.",
                            confirmLabel: "Delete",
                            onConfirm: () => deleteCollection(c.name).then(onSnapshot).catch((e) => onError(String(e))),
                          })
                        }
                      >
                        Delete
                      </ContextMenu.Item>
                    </ContextMenu.Popup>
                  </ContextMenu.Positioner>
                </ContextMenu.Portal>
              </ContextMenu.Root>
            ))}
          </div>
        ))}

        <div style={{ flexGrow: 1 }} />

        <Menu.Root>
          <Menu.Trigger><Settings /> View</Menu.Trigger>
          <Menu.Portal>
            <Menu.Positioner>
              <Menu.Popup>
                <Menu.CheckboxItem checked={hideCompleted} onCheckedChange={onToggleHideCompleted}>
                  Hide Completed
                </Menu.CheckboxItem>
                <Menu.CheckboxItem checked={showArchived} onCheckedChange={onToggleShowArchived}>
                  Show Archived
                </Menu.CheckboxItem>
                <Menu.CheckboxItem checked={usesAutoDraft} onCheckedChange={onToggleAutoDraft}>
                  Automatic Drafts
                </Menu.CheckboxItem>
                <Menu.CheckboxItem checked={alwaysOnTop} onCheckedChange={onToggleAlwaysOnTop}>
                  Always On Top
                </Menu.CheckboxItem>
                <Menu.Separator />
                <Menu.Item
                  onClick={() =>
                    setPrompt({
                      title: "New Group",
                      label: "Group name",
                      initial: "",
                      submit: (v) => createGroup(v).then(onSnapshot).catch((e) => onError(String(e))),
                    })
                  }
                >
                  Add a Group
                </Menu.Item>
                <Menu.Separator />
                <Menu.Item onClick={onOpenSettings}>Settings…</Menu.Item>
              </Menu.Popup>
            </Menu.Positioner>
          </Menu.Portal>
        </Menu.Root>

        {/* Right-edge resize handle. */}
        <div
          onPointerDown={startResize}
          role="separator"
          aria-orientation="vertical"
          className="group top-0 right-0 absolute flex justify-center -mr-4 w-8 h-full cursor-col-resize"
        >
          <div
            className={`h-full w-px transition-colors ${
              dragging ? "bg-gray-300" : "bg-gray-50 group-hover:bg-gray-200"
            }`}
          />
        </div>
      </SidebarContainer>
    </>
  );
}
