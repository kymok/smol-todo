import { useCallback, useEffect, useRef, useState } from "react";
import { AlertDialog, Button, Flex } from "@radix-ui/themes";
import type { Settings, Snapshot } from "./api/types";
import {
  createItem,
  deleteItem,
  getSettings,
  getSnapshot,
  onStoreChanged,
  setSettings,
} from "./api/client";
import { ALL_COLLECTION, type ViewState } from "./state/view";
import type { ConfirmRequest } from "./state/confirm";
import { Sidebar } from "./components/Sidebar";
import { DetailPane } from "./components/DetailPane";

const EMPTY: Snapshot = { items: [], collections: [], groups: [] };

const DEFAULT_SETTINGS: Settings = {
  usesAutoDraft: true,
  alwaysOnTop: false,
  defaultPromptTemplate: "",
  lastSelectedCollection: null,
};

export function App() {
  const [snapshot, setSnapshot] = useState<Snapshot>(EMPTY);
  const [view, setView] = useState<ViewState>({
    selected: ALL_COLLECTION,
    search: "",
    incompleteOnly: false,
    hideCompleted: false,
    showArchived: false,
  });
  const [focusedId, setFocusedId] = useState<string | null>(null);
  const [editingTarget, setEditingTarget] = useState<{ id: string; field: "title" | "note" } | null>(null);
  const [confirm, setConfirm] = useState<ConfirmRequest | null>(null);
  const [settings, setSettingsState] = useState<Settings>(DEFAULT_SETTINGS);
  const settingsRef = useRef(settings);
  settingsRef.current = settings;

  // Always-current snapshot for keyboard handlers.
  const snapRef = useRef(snapshot);
  snapRef.current = snapshot;
  const viewRef = useRef(view);
  viewRef.current = view;
  const focusRef = useRef(focusedId);
  focusRef.current = focusedId;

  // Initial load + external (CLI) edits.
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    const refresh = () => {
      getSnapshot().then(setSnapshot).catch((e) => console.error(e));
    };
    refresh();
    onStoreChanged(refresh).then((fn) => {
      if (cancelled) fn();
      else unlisten = fn;
    });
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  // Every mutation returns a fresh snapshot; child callbacks call client wrappers
  // and pass the resolved snapshot here.
  const apply = useCallback((next: Snapshot) => setSnapshot(next), []);
  const requestConfirm = useCallback((req: ConfirmRequest) => setConfirm(req), []);

  // Merge a partial change into settings, persist the whole object, and update state
  // from the persisted result.
  const updateSettings = useCallback((patch: Partial<Settings>) => {
    const next = { ...settingsRef.current, ...patch };
    setSettingsState(next); // optimistic
    setSettings(next)
      .then(setSettingsState)
      .catch((e) => console.error(e));
  }, []);

  // Fetch settings on mount; restore lastSelectedCollection if it still exists.
  useEffect(() => {
    getSettings()
      .then((s) => {
        setSettingsState(s);
        const last = s.lastSelectedCollection;
        if (last) {
          const exists = snapRef.current.collections.some((c) => c.name === last);
          if (exists) setView((v) => ({ ...v, selected: last }));
        }
      })
      .catch((e) => console.error(e));
  }, []);

  // Persist the selected collection so it can be restored next launch.
  useEffect(() => {
    const sel = view.selected === ALL_COLLECTION ? null : view.selected;
    if (sel !== settingsRef.current.lastSelectedCollection) {
      updateSettings({ lastSelectedCollection: sel });
    }
  }, [view.selected, updateSettings]);

  const onEdit = useCallback((id: string, field: "title" | "note") => {
    setEditingTarget({ id, field });
  }, []);
  const onEndEdit = useCallback(() => setEditingTarget(null), []);

  // Cmd+N (create in selected collection; "All" → default) and Cmd+Backspace (delete focused).
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.metaKey && (e.key === "n" || e.key === "N")) {
        e.preventDefault();
        const sel = viewRef.current.selected;
        const target = sel === ALL_COLLECTION ? undefined : sel;
        createItem(target)
          .then((snap) => {
            setSnapshot(snap);
            // Focus + edit the newly created (empty) draft: the last item in the target.
            const created = [...snap.items]
              .reverse()
              .find((i) => i.title === "" && i.status === "draft" && (!target || i.collection === target));
            if (created) {
              setFocusedId(created.id);
              setEditingTarget({ id: created.id, field: "title" });
            }
          })
          .catch((err) => console.error(err));
      } else if (e.metaKey && (e.key === "Backspace" || e.key === "Delete")) {
        const target = e.target as HTMLElement | null;
        if (target && (target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.isContentEditable)) {
          return; // let the text field handle Cmd+Backspace; never delete the task while editing/searching
        }
        const id = focusRef.current;
        if (id) {
          e.preventDefault();
          deleteItem(id).then(setSnapshot).catch((err) => console.error(err));
        }
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  return (
    <Flex height="100vh">
      <Sidebar
        snapshot={snapshot}
        selected={view.selected}
        showArchived={view.showArchived}
        hideCompleted={view.hideCompleted}
        onSelect={(name) => setView((v) => ({ ...v, selected: name }))}
        onToggleHideCompleted={() => setView((v) => ({ ...v, hideCompleted: !v.hideCompleted }))}
        onToggleShowArchived={() => setView((v) => ({ ...v, showArchived: !v.showArchived }))}
        onSnapshot={apply}
        onRequestConfirm={requestConfirm}
      />
      <DetailPane
        snapshot={snapshot}
        view={view}
        focusedId={focusedId}
        editingTarget={editingTarget}
        onSearch={(q) => setView((v) => ({ ...v, search: q }))}
        onFocusItem={setFocusedId}
        onEdit={onEdit}
        onEndEdit={onEndEdit}
        onSnapshot={apply}
        onRequestConfirm={requestConfirm}
      />

      <AlertDialog.Root open={confirm !== null} onOpenChange={(o) => { if (!o) setConfirm(null); }}>
        <AlertDialog.Content maxWidth="420px">
          <AlertDialog.Title>{confirm?.title ?? ""}</AlertDialog.Title>
          <AlertDialog.Description size="2">{confirm?.description ?? ""}</AlertDialog.Description>
          <Flex gap="3" mt="4" justify="end">
            <AlertDialog.Cancel>
              <Button variant="soft" color="gray">Cancel</Button>
            </AlertDialog.Cancel>
            <AlertDialog.Action>
              <Button
                color="red"
                onClick={() => {
                  confirm?.onConfirm();
                  setConfirm(null);
                }}
              >
                {confirm?.confirmLabel ?? "Delete"}
              </Button>
            </AlertDialog.Action>
          </Flex>
        </AlertDialog.Content>
      </AlertDialog.Root>
    </Flex>
  );
}
