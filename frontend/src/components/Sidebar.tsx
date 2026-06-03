import { useState, useEffect } from "react";
import { Accordion } from "@base-ui-components/react/accordion";
import { ContextMenu } from "@base-ui-components/react/context-menu";
import { Dialog } from "@base-ui-components/react/dialog";
import { Menu } from "@base-ui-components/react/menu";
import { Input } from "@base-ui-components/react/input";
import { Archive, Circle, CircleSmall, Inbox, Settings } from "lucide-react";
import type { CollectionColor, CollectionSummary, Snapshot } from "../api/types";
import {
  ALL_COLLECTION, DEFAULT_COLLECTION, allIncompleteCount, sidebarGroups,
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
import { FilledCircle } from "./icons/FilledCircle";

const COLORS: CollectionColor[] = ["gray", "red", "orange", "yellow", "green", "blue", "purple"];

// Maps each collection color to a Tailwind text-color class. Full class strings
// (not interpolated) so Tailwind's scanner picks them up.
const COLLECTION_COLOR_CLASS: Record<CollectionColor, string> = {
  gray: "text-neutral-600",
  red: "text-red-600",
  orange: "text-orange-600",
  yellow: "text-yellow-600",
  green: "text-green-600",
  blue: "text-blue-600",
  purple: "text-purple-600",
};

// Shared row chrome for sidebar items (All, collections, group headers, View).
// Each row fills the container (px-2) full width, and px-2 re-insets its content
// so the content lines up with the detail pane while the active background
// extends toward the edge. Groups/View can't become active yet but share this for
// future use. Vertical padding is per-row (groups py-1, everything else py-2).
//
// SIDEBAR_ROW_BASE omits the hover and active-background styles so group headers
// can show hover/active via text color only; the other rows add the background
// hover and active background on top.
const SIDEBAR_ROW_BASE =
  "w-full px-2 rounded-lg aria-[current=true]:text-sky-600";
const SIDEBAR_ROW_CLASS = `${SIDEBAR_ROW_BASE} hover:bg-neutral-50 aria-[current=true]:bg-sky-100`;

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
  const visibleGroups = sidebarGroups(snapshot, showArchived);
  const allCount = allIncompleteCount(snapshot);

  // Accordion open/close is tracked as the set of *closed* groups so newly added
  // groups default to open. The accordion's controlled value is the open ones.
  const [closedGroups, setClosedGroups] = useState<string[]>([]);
  const openGroupValue = visibleGroups
    .map((g) => g.name)
    .filter((name) => !closedGroups.includes(name));
  const handleGroupOpenChange = (open: (string | null)[]) => {
    setClosedGroups(visibleGroups.map((g) => g.name).filter((name) => !open.includes(name)));
  };

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
        <button
          aria-current={selected === ALL_COLLECTION}
          onClick={() => onSelect(ALL_COLLECTION)}
          className={`group flex items-center gap-2 py-2 -mt-2 text-sm text-neutral-800 ${SIDEBAR_ROW_CLASS}`}
        >
          <span className="text-neutral-600 shrink-0 group-aria-[current=true]:text-sky-600"><CircleSmall size={16} /></span>
          <span className="flex-1 min-w-0 font-normal text-left truncate">All</span>
          {allCount > 0 && <span className="shrink-0 text-xs text-neutral-500 group-aria-[current=true]:text-sky-500">{allCount}</span>}
        </button>

        {/* Keep mt-* (gap above the first group, i.e. between All and the groups)
            in sync with gap-* (gap between groups) so the spacing stays uniform. */}
        <Accordion.Root
          multiple
          className="flex flex-col gap-2 mt-2"
          value={openGroupValue}
          onValueChange={handleGroupOpenChange}
        >
        {visibleGroups.map((group) => {
          const isOpen = !closedGroups.includes(group.name);
          // A collapsed group whose collection is currently selected shows the
          // active color, since the selected row is hidden inside it.
          const headerActive =
            !isOpen && group.collections.some((c) => c.name === selected);
          return (
          <Accordion.Item key={group.name} value={group.name} className="flex flex-col">
            <Accordion.Header>
            <ContextMenu.Root>
              <ContextMenu.Trigger>
                <Accordion.Trigger
                  aria-current={headerActive}
                  className={`${SIDEBAR_ROW_BASE} hover:text-neutral-500 py-1 text-left font-medium text-neutral-400 text-xs`}
                >
                  {group.name === "DefaultGroup" ? "No Group" : group.name}
                </Accordion.Trigger>
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
            </Accordion.Header>

            <Accordion.Panel className="flex flex-col">
            {group.collections.length === 0 ? (
              // Empty-group placeholder: no icon, but a spacer matching the icon
              // width keeps the text aligned with the other rows' labels.
              <div className="flex items-center gap-2 px-2 py-2 text-sm text-neutral-400">
                <span className="w-4 shrink-0" aria-hidden />
                <span>No Collections</span>
              </div>
            ) : group.collections.map((c) => (
              <ContextMenu.Root key={c.name}>
                <ContextMenu.Trigger>
                  <button
                    aria-current={selected === c.name}
                    onClick={() => onSelect(c.name)}
                    style={c.isArchived ? { opacity: 0.5 } : undefined}
                    className={`group flex items-center gap-2 py-2 text-sm text-neutral-800 ${SIDEBAR_ROW_CLASS}`}
                  >
                    <span className={`shrink-0 ${c.name === DEFAULT_COLLECTION ? "text-neutral-600 group-aria-[current=true]:text-sky-600" : COLLECTION_COLOR_CLASS[c.color]}`}>
                      {c.isArchived ? <Archive size={16} />
                        : c.name === DEFAULT_COLLECTION ? <Inbox size={16} />
                        : <FilledCircle size={16} />}
                    </span>
                    <span className="flex-1 min-w-0 font-normal text-left truncate">{c.displayName}</span>
                    {c.incompleteCount > 0 && <span className="shrink-0 text-xs text-neutral-500 group-aria-[current=true]:text-sky-500">{c.incompleteCount}</span>}
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
                                  <span className={COLLECTION_COLOR_CLASS[color]}><Circle size={12} fill="currentColor" /></span> {color}
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
            </Accordion.Panel>
          </Accordion.Item>
          );
        })}
        </Accordion.Root>

        <div style={{ flexGrow: 1 }} />

        <Menu.Root>
          <Menu.Trigger className={`flex items-center gap-2 py-2 text-sm ${SIDEBAR_ROW_CLASS}`}>
            <Settings /> View
          </Menu.Trigger>
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
              dragging ? "bg-neutral-300" : "bg-neutral-100 group-hover:bg-neutral-200"
            }`}
          />
        </div>
      </SidebarContainer>
    </>
  );
}
