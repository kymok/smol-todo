import { useEffect, useState } from "react";
import { Button, Dialog, Flex, Text, TextArea } from "@radix-ui/themes";
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
      <Dialog.Content maxWidth="560px">
        <Dialog.Title>Edit Prompt{collection ? ` — ${collection}` : ""}</Dialog.Title>
        <TextArea
          value={text}
          onChange={(e) => setText(e.target.value)}
          placeholder="Prompt template…"
          rows={10}
        />
        <Text size="1" color="gray" mt="2" as="p">
          Leave empty to use the app default prompt.
        </Text>
        <Flex gap="2" mt="3" justify="end">
          <Dialog.Close>
            <Button variant="soft" color="gray">Cancel</Button>
          </Dialog.Close>
          <Button onClick={save}>Save</Button>
        </Flex>
      </Dialog.Content>
    </Dialog.Root>
  );
}
