import { useEffect, useRef, useState } from "react";
import { Text, TextArea } from "@radix-ui/themes";
import type { Snapshot, TaskItem } from "../api/types";
import {
  addNote,
  createItem,
  deleteItem,
  deleteNote,
  mergeItem,
  splitItem,
  updateItem,
  updateNote,
} from "../api/client";
import { reduceKey, type EditorIntent, type FocusDir } from "../state/editor";
import { autoDraftStatus } from "../state/autodraft";

const AUTOSAVE_MS = 500;

export interface InlineEditorProps {
  item: TaskItem;
  field: "title" | "note";
  /** Previous row's item, for the Backspace-merge precondition (title only). */
  previous?: TaskItem;
  editing: boolean;
  usesAutoDraft: boolean;
  onBeginEdit: () => void;
  onEndEdit: () => void;
  onMoveFocus: (dir: FocusDir) => void;
  onSnapshot: (snap: Snapshot) => void;
  onError: (msg: string) => void;
}

/** Swift mergeWithPrevious precondition: previous is draft/ready AND has no note. */
function canMergeInto(previous: TaskItem | undefined): previous is TaskItem {
  return (
    !!previous &&
    (previous.status === "draft" || previous.status === "ready") &&
    !previous.note
  );
}

export function InlineEditor({
  item,
  field,
  previous,
  editing,
  usesAutoDraft,
  onBeginEdit,
  onEndEdit,
  onMoveFocus,
  onSnapshot,
  onError,
}: InlineEditorProps) {
  const initial = field === "title" ? item.title : (item.note?.body ?? "");
  const [draft, setDraft] = useState(initial);
  const composingRef = useRef(false);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const ref = useRef<HTMLTextAreaElement | null>(null);

  // Title editing is locked once the task is in-progress or completed (note stays editable).
  const locked = field === "title" && (item.status === "in-progress" || item.status === "completed");

  // Reset the local draft when (re)entering edit mode, so a stale draft never leaks.
  useEffect(() => {
    if (editing) {
      setDraft(field === "title" ? item.title : (item.note?.body ?? ""));
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [editing]);

  // Clear any pending autosave timer on unmount to prevent state updates on an
  // unmounted component (e.g. when the row is removed while a timer is in flight).
  useEffect(() => {
    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const clearTimer = () => {
    if (timerRef.current) {
      clearTimeout(timerRef.current);
      timerRef.current = null;
    }
  };

  const scheduleAutosave = (value: string) => {
    clearTimer();
    if (composingRef.current) return; // never autosave mid-IME-composition
    timerRef.current = setTimeout(() => {
      void save(value, { fromAutosave: true });
    }, AUTOSAVE_MS);
  };

  // Persist the draft. ifCurrent = the snapshot item we started from (optimistic concurrency).
  const save = async (value: string, opts?: { fromAutosave?: boolean }) => {
    clearTimer();
    const trimmed = value.trim();
    try {
      if (field === "title") {
        if (trimmed === item.title) return; // unchanged
        // Edit phase = debounced autosave / blur. Auto-draft may drop the task to draft.
        const status = autoDraftStatus({
          usesAutoDraft,
          currentStatus: item.status,
          phase: "edit",
          titleChanged: true, // we already returned above when unchanged
        });
        const fields = status ? { title: value, status } : { title: value };
        const snap = await updateItem(item.id, fields, item);
        onSnapshot(snap);
      } else {
        // note
        if (trimmed.length === 0) {
          if (item.note) onSnapshot(await deleteNote(item.id, item));
          return;
        }
        if (item.note) {
          if (trimmed === item.note.body) return;
          onSnapshot(await updateNote(item.id, value)); // no _if_current variant in core
        } else {
          onSnapshot(await addNote(item.id, value, item));
        }
      }
    } catch (e) {
      onError(String(e));
    } finally {
      if (!opts?.fromAutosave) onEndEdit();
    }
  };

  const caretAtStart = () => {
    const el = ref.current;
    return !!el && el.selectionStart === 0 && el.selectionEnd === 0;
  };
  const caretAtEnd = () => {
    const el = ref.current;
    return !!el && el.selectionStart === draft.length && el.selectionEnd === draft.length;
  };

  const execute = async (intent: EditorIntent) => {
    switch (intent.type) {
      case "Split": {
        clearTimer();
        const el = ref.current;
        const caret = el ? el.selectionStart : draft.length;
        const first = draft.slice(0, caret);
        const second = draft.slice(caret);
        if (second.trim().length === 0) {
          // Caret at end → create an empty draft below (Swift createItemBelowFromTitle).
          // First, persist the current (non-empty) title, then create below.
          await updateItem(item.id, { title: first }, item).then(onSnapshot).catch((e) => onError(String(e)));
          await createItem(item.collection).then(onSnapshot).catch((e) => onError(String(e)));
        } else if (first.trim().length === 0) {
          // No usable first title → no-op (Swift returns true without splitting).
        } else {
          await splitItem(item.id, first, second).then(onSnapshot).catch((e) => onError(String(e)));
        }
        onEndEdit();
        break;
      }
      case "MergeIntoPrevious": {
        clearTimer();
        if (canMergeInto(previous)) {
          await mergeItem(item.id, previous.id, draft).then(onSnapshot).catch((e) => onError(String(e)));
          onEndEdit();
        }
        // If not mergeable, swallow the Backspace (do nothing) — matches Swift gate.
        break;
      }
      case "Commit": {
        if (field === "title") {
          clearTimer();
          const trimmed = draft.trim();
          const status = autoDraftStatus({
            usesAutoDraft,
            currentStatus: item.status,
            phase: "confirm",
            titleChanged: trimmed !== item.title,
          });
          try {
            // On confirm, always persist (title may be unchanged but status may still
            // promote a draft to ready). Send status when defined, else title only.
            const fields = status ? { title: draft, status } : { title: draft };
            onSnapshot(await updateItem(item.id, fields, item));
          } catch (e) {
            onError(String(e));
          } finally {
            onEndEdit();
          }
        } else {
          await save(draft);
        }
        if (intent.thenFocus) onMoveFocus(intent.thenFocus);
        break;
      }
      case "DeleteEmpty": {
        clearTimer();
        if (field === "title") {
          await deleteItem(item.id).then(onSnapshot).catch((e) => onError(String(e)));
        } else if (item.note) {
          await deleteNote(item.id, item).then(onSnapshot).catch((e) => onError(String(e)));
        }
        onEndEdit();
        if (intent.thenFocus) onMoveFocus(intent.thenFocus);
        break;
      }
      case "MoveFocus":
        onEndEdit();
        onMoveFocus(intent.dir);
        break;
      case "Discard":
        clearTimer();
        setDraft(initial);
        onEndEdit();
        break;
      case "None":
        break;
    }
  };

  if (!editing || locked) {
    const display = field === "title" ? (item.title || "Untitled") : (item.note?.body ?? "");
    if (field === "note" && !item.note) return null;
    const dim = field === "title" && (item.status === "completed" || item.status === "in-progress");
    return (
      <Text
        size={field === "title" ? "2" : "1"}
        color={field === "title" ? (dim ? "gray" : undefined) : "gray"}
        onClick={() => {
          if (!locked) onBeginEdit();
        }}
        style={{ cursor: locked ? "default" : "text" }}
      >
        {display}
      </Text>
    );
  }

  return (
    <TextArea
      ref={ref}
      size={field === "title" ? "2" : "1"}
      autoFocus
      value={draft}
      rows={1}
      onChange={(e) => {
        setDraft(e.target.value);
        scheduleAutosave(e.target.value);
      }}
      onCompositionStart={() => {
        composingRef.current = true;
        clearTimer();
      }}
      onCompositionEnd={(e) => {
        composingRef.current = false;
        scheduleAutosave((e.target as HTMLTextAreaElement).value);
      }}
      onKeyDown={(e) => {
        const intent = reduceKey({
          key: e.key,
          metaKey: e.metaKey,
          shiftKey: e.shiftKey,
          altKey: e.altKey,
          ctrlKey: e.ctrlKey,
          caretAtStart: caretAtStart(),
          caretAtEnd: caretAtEnd(),
          value: draft,
          field,
          composing: composingRef.current || e.nativeEvent.isComposing,
        });
        if (intent.type !== "None") {
          e.preventDefault();
          void execute(intent);
        }
      }}
      onBlur={() => {
        clearTimer();
        if (!composingRef.current) void save(draft); // blur commits (Swift focus-loss save)
      }}
    />
  );
}
