import { useCallback, useEffect, useRef, useState } from "react";
import { AlertDialog, Button, Flex } from "@radix-ui/themes";
import { getCurrentWindow } from "@tauri-apps/api/window";
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
import { SettingsDialog } from "./components/SettingsDialog";
import { PromptEditorDialog } from "./components/PromptEditorDialog";

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
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [promptCollection, setPromptCollection] = useState<string | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const onError = useCallback((msg: string) => setErrorMessage(msg), []);
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

  // Restore-on-launch guards: settingsLoadedRef becomes true once getSettings()
  // resolves; restoredRef becomes true once the one-shot restore runs (or is
  // deliberately skipped) so it never fires again.
  const settingsLoadedRef = useRef(false);
  const pendingLastCollectionRef = useRef<string | null>(null);
  const restoredRef = useRef(false);

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

  // Fetch settings on mount; stash lastSelectedCollection for the snapshot-gated restore below.
  useEffect(() => {
    getSettings()
      .then((s) => {
        setSettingsState(s);
        pendingLastCollectionRef.current = s.lastSelectedCollection;
        settingsLoadedRef.current = true;
      })
      .catch((e) => console.error(e));
  }, []);

  // Restore lastSelectedCollection once, after BOTH settings have loaded AND the
  // snapshot has at least one collection.  Using snapshot.collections as the
  // dependency means this re-runs each time the collection list changes until the
  // one-shot guard (restoredRef) trips.
  useEffect(() => {
    if (restoredRef.current) return;
    if (!settingsLoadedRef.current) return;
    if (snapshot.collections.length === 0) return;

    // Mark as done before any state write so a re-render can't double-fire.
    restoredRef.current = true;

    const last = pendingLastCollectionRef.current;
    if (
      last &&
      viewRef.current.selected === ALL_COLLECTION &&
      snapshot.collections.some((c) => c.name === last)
    ) {
      setView((v) => ({ ...v, selected: last }));
    }
  }, [snapshot.collections]);

  // Apply always-on-top to the window whenever the setting changes (and on mount).
  useEffect(() => {
    getCurrentWindow()
      .setAlwaysOnTop(settings.alwaysOnTop)
      .catch((e) => console.error(e));
  }, [settings.alwaysOnTop]);

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
          .catch((err) => onError(String(err)));
      } else if (e.metaKey && (e.key === "Backspace" || e.key === "Delete")) {
        const target = e.target as HTMLElement | null;
        if (target && (target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.isContentEditable)) {
          return; // let the text field handle Cmd+Backspace; never delete the task while editing/searching
        }
        const id = focusRef.current;
        if (id) {
          e.preventDefault();
          deleteItem(id).then(setSnapshot).catch((err) => onError(String(err)));
        }
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const promptInitial =
    promptCollection === null
      ? undefined
      : snapshot.collections.find((c) => c.name === promptCollection)?.promptTemplate;

  return (
    <Flex height="100vh">
      <Sidebar
        snapshot={snapshot}
        selected={view.selected}
        showArchived={view.showArchived}
        hideCompleted={view.hideCompleted}
        usesAutoDraft={settings.usesAutoDraft}
        alwaysOnTop={settings.alwaysOnTop}
        onSelect={(name) => setView((v) => ({ ...v, selected: name }))}
        onToggleHideCompleted={() => setView((v) => ({ ...v, hideCompleted: !v.hideCompleted }))}
        onToggleShowArchived={() => setView((v) => ({ ...v, showArchived: !v.showArchived }))}
        onToggleAutoDraft={() => updateSettings({ usesAutoDraft: !settingsRef.current.usesAutoDraft })}
        onToggleAlwaysOnTop={() => updateSettings({ alwaysOnTop: !settingsRef.current.alwaysOnTop })}
        onOpenSettings={() => setSettingsOpen(true)}
        onEditPrompt={(name) => setPromptCollection(name)}
        onSnapshot={apply}
        onError={onError}
        onRequestConfirm={requestConfirm}
      />
      <DetailPane
        snapshot={snapshot}
        view={view}
        focusedId={focusedId}
        editingTarget={editingTarget}
        usesAutoDraft={settings.usesAutoDraft}
        onSearch={(q) => setView((v) => ({ ...v, search: q }))}
        onFocusItem={setFocusedId}
        onEdit={onEdit}
        onEndEdit={onEndEdit}
        onSnapshot={apply}
        onError={onError}
        onRequestConfirm={requestConfirm}
      />

      <SettingsDialog open={settingsOpen} onOpenChange={setSettingsOpen} />

      <PromptEditorDialog
        collection={promptCollection}
        initialTemplate={promptInitial}
        onClose={() => setPromptCollection(null)}
        onSnapshot={apply}
        onError={onError}
      />

      <AlertDialog.Root open={errorMessage !== null} onOpenChange={(o) => { if (!o) setErrorMessage(null); }}>
        <AlertDialog.Content maxWidth="420px">
          <AlertDialog.Title>Something went wrong</AlertDialog.Title>
          <AlertDialog.Description size="2">{errorMessage ?? ""}</AlertDialog.Description>
          <Flex gap="3" mt="4" justify="end">
            <AlertDialog.Action>
              <Button onClick={() => setErrorMessage(null)}>OK</Button>
            </AlertDialog.Action>
          </Flex>
        </AlertDialog.Content>
      </AlertDialog.Root>

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
