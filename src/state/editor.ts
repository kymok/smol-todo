export type FocusDir = "up" | "down";

export type EditorIntent =
  | { type: "Split" }
  | { type: "MergeIntoPrevious" }
  | { type: "Commit"; thenFocus?: FocusDir }
  | { type: "MoveFocus"; dir: FocusDir }
  | { type: "DeleteEmpty"; thenFocus?: FocusDir }
  | { type: "Discard" }
  | { type: "None" };

export interface KeyContext {
  key: string; // KeyboardEvent.key
  metaKey: boolean; // Cmd (macOS)
  shiftKey: boolean;
  altKey: boolean; // Option/Alt
  ctrlKey: boolean; // Control
  caretAtStart: boolean; // selection collapsed at offset 0
  caretAtEnd: boolean; // selection collapsed at end of value
  value: string; // current draft text
  field: "title" | "note";
  composing: boolean; // IME composition in progress
}

function isEmpty(value: string): boolean {
  return value.trim().length === 0;
}

function reduceTitle(c: KeyContext): EditorIntent {
  switch (c.key) {
    case "Escape":
      return { type: "Discard" };
    case "Enter": {
      if (c.composing) return { type: "None" }; // IME commit — never split
      if (c.metaKey) {
        // Cmd+Enter → confirm (Swift handleTitleReturn, guarded by isCommandReturnKey)
        return isEmpty(c.value) ? { type: "DeleteEmpty" } : { type: "Commit" };
      }
      // Plain Enter (no modifiers) → split at caret (Swift handlePlainTitleReturn). Empty title → delete the draft.
      // Shift/Alt/Ctrl+Enter: let the textarea handle it (e.g. Shift+Enter inserts a newline).
      if (!c.shiftKey && !c.altKey && !c.ctrlKey) {
        return isEmpty(c.value) ? { type: "DeleteEmpty" } : { type: "Split" };
      }
      return { type: "None" };
    }
    case "Tab": {
      if (c.composing) return { type: "None" }; // IME — do not commit/move
      // Swift: plain Tab confirms title then moves focus down.
      return isEmpty(c.value)
        ? { type: "DeleteEmpty", thenFocus: "down" }
        : { type: "Commit", thenFocus: "down" };
    }
    case "Backspace": {
      // Swift deleteIfEmptyTitleAtStart: only at caret location 0, collapsed.
      // isModifiedBackspace = metaKey || altKey || ctrlKey (NOT shiftKey).
      // Shift+Backspace still merges; Cmd/Option/Ctrl+Backspace do NOT (e.g. Option+Backspace = delete word).
      if (c.caretAtStart && !c.metaKey && !c.altKey && !c.ctrlKey) {
        return { type: "MergeIntoPrevious" };
      }
      return { type: "None" };
    }
    case "ArrowUp":
      return { type: "MoveFocus", dir: "up" };
    case "ArrowDown":
      return { type: "MoveFocus", dir: "down" };
    default:
      return { type: "None" };
  }
}

function reduceNote(c: KeyContext): EditorIntent {
  switch (c.key) {
    case "Escape":
      return { type: "Discard" };
    case "Enter":
    case "Tab": {
      if (c.composing) return { type: "None" }; // IME — do not commit/move
      // Swift handleNoteKeyDown: Return/Tab move focus down; empty note removes it.
      return isEmpty(c.value)
        ? { type: "DeleteEmpty", thenFocus: "down" }
        : { type: "Commit", thenFocus: "down" };
    }
    case "ArrowUp":
      return { type: "MoveFocus", dir: "up" };
    case "ArrowDown":
      return { type: "MoveFocus", dir: "down" };
    default:
      return { type: "None" };
  }
}

/** Pure key → intent. The component executes the intent (calls the client). */
export function reduceKey(c: KeyContext): EditorIntent {
  return c.field === "title" ? reduceTitle(c) : reduceNote(c);
}
