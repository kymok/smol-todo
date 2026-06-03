import { useEffect, useState } from "react";
import { Dialog } from "@base-ui-components/react/dialog";
import { Select } from "@base-ui-components/react/select";
import type { Snapshot, TaskStatus } from "../api/types";
import { setStatuses } from "../api/client";
import { presentStatuses } from "../state/bulkStatus";

const STATUS_LABELS: Record<TaskStatus, string> = {
  draft: "Draft",
  ready: "Ready",
  "in-progress": "In Progress",
  completed: "Completed",
  "on-hold": "On Hold",
  rejected: "Rejected",
  aborted: "Aborted",
};

const ALL_STATUSES: TaskStatus[] = [
  "draft", "ready", "in-progress", "completed", "on-hold", "rejected", "aborted",
];

export interface BulkStatusDialogProps {
  /** The collection whose statuses are being remapped; null = closed. */
  collection: string | null;
  snapshot: Snapshot;
  onClose: () => void;
  onSnapshot: (snap: Snapshot) => void;
  onError: (msg: string) => void;
}

export function BulkStatusDialog({
  collection,
  snapshot,
  onClose,
  onSnapshot,
  onError,
}: BulkStatusDialogProps) {
  const rows = collection === null ? [] : presentStatuses(snapshot, collection);
  // Selection per present status; default = unchanged (the same status).
  const [selections, setSelections] = useState<Record<string, TaskStatus>>({});

  // Reset selections to "unchanged" whenever the dialog (re)opens for a collection.
  useEffect(() => {
    if (collection === null) return;
    const init: Record<string, TaskStatus> = {};
    for (const s of presentStatuses(snapshot, collection)) init[s] = s;
    setSelections(init);
    // Depend on collection only: opening seeds once; re-seeding on every snapshot
    // would clobber an in-progress selection.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [collection]);

  const confirm = () => {
    if (collection === null) return;
    const replacements: Record<string, string> = {};
    for (const from of rows) {
      const to = selections[from] ?? from;
      if (to !== from) replacements[from] = to;
    }
    setStatuses(replacements, collection)
      .then((snap) => { onSnapshot(snap); onClose(); })
      .catch((e) => onError(String(e)));
  };

  return (
    <Dialog.Root open={collection !== null} onOpenChange={(o) => { if (!o) onClose(); }}>
      <Dialog.Portal>
        <Dialog.Backdrop />
        <Dialog.Popup>
          <Dialog.Title>Change Statuses{collection ? ` — ${collection}` : ""}</Dialog.Title>
          {rows.length === 0 ? (
            <span>This collection has no items.</span>
          ) : (
            <div>
              {rows.map((from) => (
                <div key={from}>
                  <span>{STATUS_LABELS[from]}</span>
                  <span>→</span>
                  <Select.Root
                    value={selections[from] ?? from}
                    onValueChange={(v) => setSelections((s) => ({ ...s, [from]: v as TaskStatus }))}
                  >
                    <Select.Trigger>
                      <Select.Value />
                    </Select.Trigger>
                    <Select.Portal>
                      <Select.Positioner>
                        <Select.Popup>
                          {ALL_STATUSES.map((s) => (
                            <Select.Item key={s} value={s}>
                              <Select.ItemText>{STATUS_LABELS[s]}</Select.ItemText>
                            </Select.Item>
                          ))}
                        </Select.Popup>
                      </Select.Positioner>
                    </Select.Portal>
                  </Select.Root>
                </div>
              ))}
            </div>
          )}
          <div>
            <Dialog.Close>Cancel</Dialog.Close>
            <button onClick={confirm} disabled={rows.length === 0}>OK</button>
          </div>
        </Dialog.Popup>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
