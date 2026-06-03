import { useEffect, useState } from "react";
import { Dialog } from "@base-ui-components/react/dialog";
import type { Snapshot } from "../api/types";
import { setCollectionPrompt } from "../api/client";

export interface PromptEditorDialogProps {
  /** The collection whose override is being edited; null = closed. */
  collection: string | null;
  /** The collection's current raw override (may be empty/undefined). */
  initialTemplate?: string;
  onClose: () => void;
  onSnapshot: (snap: Snapshot) => void;
  onError: (msg: string) => void;
}

export function PromptEditorDialog({
  collection,
  initialTemplate,
  onClose,
  onSnapshot,
  onError,
}: PromptEditorDialogProps) {
  const [text, setText] = useState("");

  // Seed the editor from the collection's current override whenever it opens.
  useEffect(() => {
    if (collection !== null) setText(initialTemplate ?? "");
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [collection]);

  const save = () => {
    if (collection === null) return;
    const trimmed = text.trim();
    setCollectionPrompt(collection, trimmed === "" ? null : text)
      .then((snap) => {
        onSnapshot(snap);
        onClose();
      })
      .catch((e) => onError(String(e)));
  };

  return (
    <Dialog.Root open={collection !== null} onOpenChange={(o) => { if (!o) onClose(); }}>
      <Dialog.Portal>
        <Dialog.Backdrop />
        <Dialog.Popup>
          <Dialog.Title>Edit Prompt{collection ? ` — ${collection}` : ""}</Dialog.Title>
          <textarea
            value={text}
            onChange={(e) => setText(e.target.value)}
            placeholder="Prompt template…"
            rows={10}
          />
          <p>Leave empty to use the app default prompt.</p>
          <div>
            <Dialog.Close>Cancel</Dialog.Close>
            <button onClick={save}>Save</button>
          </div>
        </Dialog.Popup>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
